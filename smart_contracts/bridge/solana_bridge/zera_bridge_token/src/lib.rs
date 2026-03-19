use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::solana_program::program_option::COption;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::system_instruction::transfer;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Burn, Mint, SyncNative, Token, TokenAccount};
use mpl_token_metadata::instructions::CreateV1CpiBuilder;
use mpl_token_metadata::types::{PrintSupply, TokenStandard};

declare_id!("WrapZ8f88HR8waSp7wR8Vgc68z4hKj3p3i2b81oeSxR");

/// Program version - update this single constant for each release
pub const PROGRAM_VERSION: &str = "1.1.0";

const ROUTER_SIGNER_SEED: &[u8] = b"router_signer";
const EXPECTED_CORE_ID: Pubkey = pubkey!("zera3giq7oM9QJaD6mY1ajGmakv9TZcax5Giky99HD8");
const MINT_AUTH_SEED: &[u8] = b"mint_authority";
const MINT_SEED: &[u8] = b"mint";
const VAULT_SEED: &[u8] = b"vault";
/// Wrapped SOL mint address (native_mint)
const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
const VERIFIED_SEED: &[u8] = b"verified_transfer"; // must match core
const RELEASED_SEED: &[u8] = b"released_transfer";
const VERIFIED_ADMIN_SEED: &[u8] = b"verified_admin"; // must match core
const MAX_CONTRACT_ID_LEN: usize = 64;
const ACTION_RELEASE_SOL: u8 = 0;
const ACTION_RELEASE_SPL: u8 = 1;
const ACTION_MINT_WRAPPED_INIT: u8 = 2;  // First mint with metadata
const ACTION_MINT_WRAPPED: u8 = 3;        // Subsequent mints without metadata
const BRIDGE_INFO_SEED: &[u8] = b"bridge_info";
const RATE_LIMIT_STATE_SEED: &[u8] = b"rate_limit_state";
const TOKEN_PRICE_REGISTRY_SEED: &[u8] = b"token_price_registry";
const ACTION_RESET_RATE_LIMIT: u8 = 100; // must match core
const ACTION_REGISTER_TOKEN: u8 = 4;  // New: Register SPL token for bridging

// Token Registration constants
const TOKEN_REGISTRATION_SEED: &[u8] = b"token_registration";
const PENDING_REGISTRATION_SEED: &[u8] = b"pending_registration";
const TIER_UNPRICED: u8 = 0;   // Low liquidity tokens, $0 value for rate limits, can lock/release
const TIER_PRICED: u8 = 1;     // Normal tokens, uses price/liquidity for rate limits, can lock/release
const TIER_EXIT_ONLY: u8 = 2;  // Delisted tokens, no new locks/burns, releases/mints allowed (exit only)

pub const ZERA_BRIDGE_TOKEN_DOMAIN: &[u8] = b"SOLANA_BRIDGE_TOKEN";

#[program]
pub mod zera_bridge_token_v1 {
    use super::*;

    /// Initialize the rate limit state PDA
    pub fn initialize_rate_limit_state(ctx: Context<InitializeRateLimitState>) -> Result<()> {
        let rate_limit_state = &mut ctx.accounts.rate_limit_state;
        rate_limit_state.current_hour = (Clock::get()?.unix_timestamp / 3600) as u64;
        rate_limit_state.hourly_buckets = [0; 24];
        rate_limit_state.current_bucket_index = 0;
        
        msg!("✅ Rate limit state initialized");
        msg!("   Current hour: {}", rate_limit_state.current_hour);
        
        Ok(())
    }

    /// Initialize the token price registry PDA
    pub fn initialize_token_price_registry(ctx: Context<InitializeTokenPriceRegistry>) -> Result<()> {
        let registry = &mut ctx.accounts.token_price_registry;
        registry.entries = Vec::new();

        msg!("✅ Token price registry initialized");

        Ok(())
    }

    /// Register an SPL token for bridging. Requires guardian-signed VAA.
    /// Creates a TokenRegistration PDA that stores price, liquidity, and tier info.
    /// liquidity_usd_nano serves as both the liquidity value AND the per-token 24h rate limit.
    pub fn register_token(
        ctx: Context<RegisterToken>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        tx_signature: [u8; 32],
        event_index: u32,
        usd_price_nano: u64,
        liquidity_usd_nano: u64,
        tier: u8,
    ) -> Result<()> {
        // Validate tier
        require!(tier <= TIER_EXIT_ONLY, SimpleErr::InvalidTier);

        // Optional freshness check
        if expiry != 0 {
            require!(
                Clock::get()?.unix_timestamp <= expiry as i64,
                SimpleErr::Expired
            );
        }

        // Build payload: mint (32) + price (8) + liquidity (8) + tier (1)
        let mut payload = Vec::with_capacity(32 + 8 + 8 + 1);
        payload.extend_from_slice(ctx.accounts.mint.key().as_ref());
        payload.extend_from_slice(&usd_price_nano.to_be_bytes());
        payload.extend_from_slice(&liquidity_usd_nano.to_be_bytes());
        payload.push(tier);

        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_TOKEN_DOMAIN,
            ACTION_REGISTER_TOKEN,
            timestamp,
            expiry,
            tx_signature,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        // Verify used_marker PDA exists (proves guardian signatures were verified by core)
        let seeds: &[&[u8]] = &[VERIFIED_SEED, &expected_hash];
        let (used_pda, _bump) = Pubkey::find_program_address(seeds, &EXPECTED_CORE_ID);

        require_keys_eq!(
            used_pda,
            ctx.accounts.used_marker.key(),
            SimpleErr::BadUsedMarkerPda
        );
        require_keys_eq!(
            *ctx.accounts.used_marker.owner,
            EXPECTED_CORE_ID,
            SimpleErr::BadUsedMarkerOwner
        );
        require!(
            ctx.accounts.used_marker.lamports() > 0,
            SimpleErr::BadUsedMarkerLamports
        );

        // Initialize or update TokenRegistration
        let token_registration = &mut ctx.accounts.token_registration;
        let is_new = token_registration.mint == Pubkey::default();

        if is_new {
            // First registration - initialize all fields
            token_registration.mint = ctx.accounts.mint.key();
            token_registration.current_hour = (Clock::get()?.unix_timestamp / 3600) as u64;
            token_registration.hourly_buckets = [0; 24];
            token_registration.current_bucket_index = 0;
        }
        // Always update these fields (both new and existing)
        token_registration.tier = tier;
        token_registration.usd_price_nano = usd_price_nano;
        token_registration.liquidity_usd_nano = liquidity_usd_nano;

        msg!(
            r#"{{"event":"{}","mint":"{}","tier":"{}","price_nano":"{}","liquidity_nano":"{}"}}"#,
            if is_new { "Register_Token" } else { "Update_Token_Registration" },
            ctx.accounts.mint.key(),
            tier,
            usd_price_nano,
            liquidity_usd_nano
        );

        Ok(())
    }

    /// Request token registration - User-initiated on-chain request.
    /// Creates a PendingTokenRegistration PDA that guardians can observe off-chain.
    /// Guardians then decide whether to approve (via register_token VAA) or reject (no action).
    pub fn request_token_registration(ctx: Context<RequestTokenRegistration>) -> Result<()> {
        let pending = &mut ctx.accounts.pending_registration;
        pending.mint = ctx.accounts.mint.key();

        msg!(
            r#"{{"event":"Request_Token_Registration","mint":"{}","version":"{}"}}"#,
            ctx.accounts.mint.key(),
            PROGRAM_VERSION
        );

        Ok(())
    }

    //WORKING / should pause on level 2 - pause working
    pub fn lock_sol(ctx: Context<LockSol>, amount: u64, zera_address: Vec<u8>) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);

        // Validate Zera address format (base58, max 64 chars)
        let zera_address_str = validate_zera_address(&zera_address, 64)?;

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;

        // Check pause (outgoing operation - block if level >= 2)
        check_pause(&router_cfg_data, 2)?;

        // Check rate limits (outgoing operation with single tx limit check)
        // NOTE: get_usd_value returns value in nano-dollars
        // Use wSOL mint for price lookup (SOL and wSOL share the same price)
        let amount_usd_nano = get_usd_value(
            amount,
            9, // SOL has 9 decimals
            &WSOL_MINT,
            &ctx.accounts.token_price_registry,
        )? as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_nano,
            true, // is_outgoing
            true, // check_single_tx_limit
        )?;

        // Validate wSOL mint
        require_keys_eq!(
            ctx.accounts.wsol_mint.key(),
            WSOL_MINT,
            SimpleErr::BadTokenAccount
        );

        // Validate router_signer PDA
        let (expected_router_signer, _) =
            Pubkey::find_program_address(&[ROUTER_SIGNER_SEED], ctx.program_id);
        require_keys_eq!(
            expected_router_signer,
            ctx.accounts.router_signer.key(),
            SimpleErr::BadRouterSigner
        );

        // Validate vault ATA is the router_signer's wSOL ATA
        let expected_vault_ata = anchor_spl::associated_token::get_associated_token_address(
            &expected_router_signer,
            &WSOL_MINT,
        );
        require_keys_eq!(
            expected_vault_ata,
            ctx.accounts.vault_ata.key(),
            SimpleErr::BadVaultPda
        );

        // Lazily create vault wSOL ATA if it doesn't exist
        if ctx.accounts.vault_ata.to_account_info().data_is_empty() {
            anchor_spl::associated_token::create(CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: ctx.accounts.payer.to_account_info(),
                    associated_token: ctx.accounts.vault_ata.to_account_info(),
                    authority: ctx.accounts.router_signer.to_account_info(),
                    mint: ctx.accounts.wsol_mint.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
            ))?;
        }

        // Lazily create payer's wSOL ATA if it doesn't exist
        if ctx.accounts.payer_wsol_ata.to_account_info().data_is_empty() {
            anchor_spl::associated_token::create(CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: ctx.accounts.payer.to_account_info(),
                    associated_token: ctx.accounts.payer_wsol_ata.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                    mint: ctx.accounts.wsol_mint.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
            ))?;
        }

        // Step 1: Transfer native SOL from payer into payer's wSOL ATA
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.payer_wsol_ata.to_account_info(),
                },
            ),
            amount,
        )?;

        // Step 2: Sync native - tells Token program to recognize the deposited lamports as wSOL
        token::sync_native(CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            SyncNative {
                account: ctx.accounts.payer_wsol_ata.to_account_info(),
            },
        ))?;

        // Step 3: Transfer wSOL from payer's ATA to vault ATA
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.payer_wsol_ata.to_account_info(),
                    to: ctx.accounts.vault_ata.to_account_info(),
                    authority: ctx.accounts.payer.to_account_info(),
                },
            ),
            amount,
        )?;

        msg!(
            r#"{{"event":"Lock_SOL","version":"{}","payer":"{}","vault_ata":"{}","mint":"{}","amount":"{}","zera_address":"{}","solana_sender":"{}"}}"#,
            PROGRAM_VERSION,
            ctx.accounts.payer.key(),
            ctx.accounts.vault_ata.key(),
            WSOL_MINT,
            amount,
            zera_address_str,
            ctx.accounts.payer.key()
        );

        Ok(())
    }
    //WORKING / should pause on level 2 - pause working
    pub fn lock_spl(ctx: Context<LockSpl>, amount: u64, zera_address: Vec<u8>) -> Result<()> {
        // Validate Zera address format (base58, max 64 chars)
        let zera_address_str = validate_zera_address(&zera_address, 64)?;

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;

        // Check pause (outgoing operation - block if level >= 2)
        check_pause(&router_cfg_data, 2)?;

        require!(amount > 0, SimpleErr::InvalidAmount);

        // Block exit-only tokens (tier 2) - these can only be released, not locked
        require!(
            ctx.accounts.token_registration.tier != TIER_EXIT_ONLY,
            SimpleErr::TokenExitOnly
        );

        // Check delisted status - price must be > 0 to lock new tokens
        require!(
            ctx.accounts.token_registration.usd_price_nano > 0,
            SimpleErr::TokenDelisted
        );

        // Calculate effective USD value using TokenRegistration (in nano-dollars)
        let effective_usd_nano = calculate_effective_value(
            amount,
            ctx.accounts.mint.decimals,
            &ctx.accounts.token_registration,
        );

        // 1. Check single TX limit (global, outbound only)
        if effective_usd_nano > router_cfg_data.single_tx_limit_usd_nano {
            msg!("❌ Single transaction limit exceeded: {} > {}", effective_usd_nano, router_cfg_data.single_tx_limit_usd_nano);
            return Err(error!(SimpleErr::SingleTxLimitExceeded));
        }

        // 2. Check and update per-token 24h rate limit
        check_and_update_token_rate_limit(
            &mut ctx.accounts.token_registration,
            effective_usd_nano as i64,
            true, // is_outgoing
        )?;

        // 3. Check and update global 24h rate limit
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            effective_usd_nano as i64,
            true,  // is_outgoing
            false, // single tx already checked above
        )?;

        // disallow bridge-wrapped mints
        let (bridge_mint_auth, _) = Pubkey::find_program_address(
            &[MINT_AUTH_SEED, ctx.accounts.mint.key().as_ref()],
            ctx.program_id,
        );

        // disallow bridge-wrapped mints
        let is_bridge =
            matches!(ctx.accounts.mint.mint_authority, COption::Some(pk) if pk == bridge_mint_auth);
        require!(!is_bridge, SimpleErr::NotForeignMint);

        // Derive this program's router_signer PDA and expected vault ATA
        let (expected_router_signer, _) =
            Pubkey::find_program_address(&[ROUTER_SIGNER_SEED], ctx.program_id);
        require_keys_eq!(
            expected_router_signer,
            ctx.accounts.router_signer.key(),
            SimpleErr::BadRouterSigner
        );
        let expected_vault_ata = anchor_spl::associated_token::get_associated_token_address(
            &expected_router_signer,
            &ctx.accounts.mint.key(),
        );

        require_keys_eq!(
            expected_vault_ata,
            ctx.accounts.vault_ata.key(),
            SimpleErr::BadVaultPda
        );

        // lazily create vault ATA (owner = this program's router_signer PDA), if missing
        if ctx.accounts.vault_ata.to_account_info().data_is_empty() {
            anchor_spl::associated_token::create(CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: ctx.accounts.payer.to_account_info(),
                    associated_token: ctx.accounts.vault_ata.to_account_info(),
                    authority: ctx.accounts.router_signer.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
            ))?;
        } else {
            // exists: basic safety checks
            require_keys_eq!(
                *ctx.accounts.vault_ata.owner,
                token::ID,
                SimpleErr::BadVaultOwner
            );
        }

            let vault_ai = ctx.accounts.vault_ata.to_account_info();
                let ata: TokenAccount =
                    TokenAccount::try_deserialize(&mut &vault_ai.data.borrow()[..])?;
        require_keys_eq!(ata.mint, ctx.accounts.mint.key(), SimpleErr::BadTokenAccount);
                require_keys_eq!(
                    ata.owner,
                    ctx.accounts.router_signer.key(),
            SimpleErr::BadTokenAccountOwner);

        require_keys_eq!(ctx.accounts.from_ata.owner, ctx.accounts.payer.key(), SimpleErr::BadTokenAccountOwner);
        require_keys_eq!(ctx.accounts.from_ata.mint, ctx.accounts.mint.key(), SimpleErr::BadTokenAccount);
        require!(ctx.accounts.from_ata.amount >= amount, SimpleErr::InvalidAmount);
        
        // transfer tokens from payer ATA -> vault ATA
        let cpi = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.from_ata.to_account_info(),
                to: ctx.accounts.vault_ata.to_account_info(),
                authority: ctx.accounts.payer.to_account_info(),
            },
        );

        token::transfer(cpi, amount)?;

        msg!(
            r#"{{"event":"Lock_SPL","version":"{}","payer":"{}","vault_ata":"{}", "mint":"{}", "amount":"{}","zera_address":"{}", "solana_sender":"{}"}}"#,
            PROGRAM_VERSION,
            ctx.accounts.payer.key(),
            ctx.accounts.vault_ata.key(),
            ctx.accounts.mint.key(),
            amount,
            zera_address_str,
            ctx.accounts.payer.key()
        );

        Ok(())
    }

    //WORKING / should pause on level 1 - pause working
    pub fn release_spl(
        ctx: Context<ReleaseSpl>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        event_index: u32,
        amount: u64,
        usd_price_nano: u64,         // Guardian-attested price in nano-dollars (10^-9 USD)
        liquidity_usd_nano: u64,     // Guardian-attested liquidity = per-token 24h rate limit
        tier: u8,                    // Token tier (0=Unpriced, 1=Priced)
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);
        require!(tier <= TIER_EXIT_ONLY, SimpleErr::InvalidTier);

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;

        // Check pause (incoming operation - block if level >= 1)
        check_pause(&router_cfg_data, 1)?;

        // SECURITY: Validate recipient is a user wallet (EOA), not a program
        // This prevents permanent fund loss if user specifies a program address
        if *ctx.accounts.recipient.owner != System::id() {
            msg!(
                r#"{{"event":"Transfer_Rejected","reason":"InvalidRecipientType","recipient":"{}","recipient_owner":"{}","amount":"{}","txn_hash":"{}","event_index":"{}","action":"MANUAL_REFUND_REQUIRED"}}"#,
                ctx.accounts.recipient.key(),
                ctx.accounts.recipient.owner,
                amount,
                to_hex(&txn_hash),
                event_index
            );
            return Err(error!(SimpleErr::InvalidRecipientType));
        }

        // Optional freshness
        if expiry != 0 {
            require!(
                Clock::get()?.unix_timestamp <= expiry as i64,
                SimpleErr::Expired
            );
        }

        // Build payload: amount (8) + recipient (32) + mint (32) + price (8) + liquidity (8) + tier (1)
        let mut payload = Vec::with_capacity(8 + 32 + 32 + 8 + 8 + 1);
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(ctx.accounts.recipient.key().as_ref());
        payload.extend_from_slice(ctx.accounts.mint.key().as_ref());
        payload.extend_from_slice(&usd_price_nano.to_be_bytes());
        payload.extend_from_slice(&liquidity_usd_nano.to_be_bytes());
        payload.push(tier);

        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_TOKEN_DOMAIN,
            ACTION_RELEASE_SPL,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        // Validate used_marker PDA
        let seeds: &[&[u8]] = &[VERIFIED_SEED, &expected_hash];

        // Find used_marker PDA
        let (used_pda, _bump) = Pubkey::find_program_address(seeds, &EXPECTED_CORE_ID);

        // Check used_marker PDA
        require_keys_eq!(
            used_pda,
            ctx.accounts.used_marker.key(),
            SimpleErr::BadUsedMarkerPda
        );

        // Check used_marker owner
        require_keys_eq!(
            *ctx.accounts.used_marker.owner,
            EXPECTED_CORE_ID,
            SimpleErr::BadUsedMarkerOwner
        );

        // Check used_marker lamports to ensure it doesnt exist
        require!(
            ctx.accounts.used_marker.lamports() > 0,
            SimpleErr::BadUsedMarkerLamports
        );

            
        let released_seeds: &[&[u8]] = &[RELEASED_SEED, &expected_hash];
        let (released_pda, released_bump) =
            Pubkey::find_program_address(released_seeds, ctx.program_id);
        require_keys_eq!(
            released_pda,
            ctx.accounts.redeemed_marker.key(),
            SimpleErr::BadRedeemedMarkerPda
        );
        require!(
            ctx.accounts.redeemed_marker.lamports() == 0,
            SimpleErr::BadRedeemedMarkerLamports
        );

        // VAA verified! Now safe to update TokenRegistration and check rate limits

        // Update TokenRegistration with latest guardian-attested values
        ctx.accounts.token_registration.usd_price_nano = usd_price_nano;
        ctx.accounts.token_registration.liquidity_usd_nano = liquidity_usd_nano;
        ctx.accounts.token_registration.tier = tier;

        // Calculate effective USD value for rate limiting (in nano-dollars)
        let effective_usd_nano = calculate_effective_value(
            amount,
            ctx.accounts.mint.decimals,
            &ctx.accounts.token_registration,
        );

        // 1. Check and update per-token 24h rate limit
        check_and_update_token_rate_limit(
            &mut ctx.accounts.token_registration,
            effective_usd_nano as i64,
            false, // is_outgoing = false (this is incoming)
        )?;

        // 2. Check and update global 24h rate limit
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            effective_usd_nano as i64,
            false, // is_outgoing (this is incoming)
            false, // check_single_tx_limit (no check for incoming)
        )?;

        // Validate router_signer PDA
        let (expected_router_signer, router_bump) =
            Pubkey::find_program_address(&[ROUTER_SIGNER_SEED], ctx.program_id);
        require_keys_eq!(
            expected_router_signer,
            ctx.accounts.router_signer.key(),
            SimpleErr::BadRouterSigner
        );

        // Validate vault_ata
        let expected_vault_ata = anchor_spl::associated_token::get_associated_token_address(
            &expected_router_signer,
            &ctx.accounts.mint.key(),
        );
        require_keys_eq!(
            expected_vault_ata,
            ctx.accounts.vault_ata.key(),
            SimpleErr::BadVaultPda
        );

        // Validate vault_ata data
        require_keys_eq!(ctx.accounts.vault_ata.mint, ctx.accounts.mint.key(), SimpleErr::BadTokenAccount);
        require_keys_eq!(
            ctx.accounts.vault_ata.owner,
            ctx.accounts.router_signer.key(),
            SimpleErr::BadTokenAccountOwner
        );
        require!(
            ctx.accounts.vault_ata.amount >= amount,
            SimpleErr::InsufficientVaultBalance
        );

        // Validate recipient_ata
        require_keys_eq!(ctx.accounts.recipient_ata.mint, ctx.accounts.mint.key(), SimpleErr::BadTokenAccount);
        require_keys_eq!(ctx.accounts.recipient_ata.owner, ctx.accounts.recipient.key(), SimpleErr::BadTokenAccountOwner);

        // Transfer SPL tokens from vault_ata to recipient_ata using router_signer PDA
        let router_signer_seeds: &[&[u8]] = &[ROUTER_SIGNER_SEED, &[router_bump]];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.vault_ata.to_account_info(),
                    to: ctx.accounts.recipient_ata.to_account_info(),
                    authority: ctx.accounts.router_signer.to_account_info(),
                },
                &[router_signer_seeds],
            ),
            amount,
        )?;

        // Create redeemed marker to prevent replay
        let lamports = Rent::get()?.minimum_balance(0);
        let released_signer_seeds: &[&[u8]] = &[RELEASED_SEED, &expected_hash, &[released_bump]];
        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.redeemed_marker.to_account_info(),
                },
                &[released_signer_seeds],
            ),
            lamports,
            0,
            ctx.program_id,
        )?;

        msg!(
            r#"{{"event":"Release_SPL","version":"{}","recipient":"{}","mint":"{}","vault_ata":"{}","amount":"{}","txn_hash":"{}","event_index":"{}"}}"#,
            version,
            ctx.accounts.recipient.key(),
            ctx.accounts.mint.key(),
            ctx.accounts.vault_ata.key(),
            amount,
            to_hex(&txn_hash),
            event_index
        );

        Ok(())
    }
    //WORKING / should pause on level 1 - pause working
    pub fn mint_wrapped(
        ctx: Context<MintWrapped>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        event_index: u32,
        amount: u64,
        zera_contract_id: Vec<u8>,
        decimals: Option<u8>,
        name: Option<String>,
        symbol: Option<String>,
        uri: Option<String>,
        usd_price_nano: u64,         // Guardian-attested price in nano-dollars (10^-9 USD)
        liquidity_usd_nano: u64,     // Guardian-attested liquidity = per-token 24h rate limit
        tier: u8,                    // Token tier (0=Unpriced, 1=Priced)
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);
        require!(
            zera_contract_id.len() > 0 && zera_contract_id.len() <= 64,
            SimpleErr::InvalidContractId
        );
        require!(tier <= TIER_EXIT_ONLY, SimpleErr::InvalidTier);

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (incoming operation - block if level >= 1)
        check_pause(&router_cfg_data, 1)?;

        // SECURITY: Validate recipient is a user wallet (EOA), not a program
        // This prevents permanent fund loss if user specifies a program address
        if *ctx.accounts.recipient.owner != System::id() {
            msg!(
                r#"{{"event":"Transfer_Rejected","reason":"InvalidRecipientType","recipient":"{}","recipient_owner":"{}","amount":"{}","zera_contract_id":"{}","txn_hash":"{}","event_index":"{}","action":"MANUAL_REFUND_REQUIRED"}}"#,
                ctx.accounts.recipient.key(),
                ctx.accounts.recipient.owner,
                amount,
                String::from_utf8_lossy(&zera_contract_id),
                to_hex(&txn_hash),
                event_index
            );
            return Err(error!(SimpleErr::InvalidRecipientType));
        }

        // Determine if this is first mint by checking if mint exists
        let is_first_mint = ctx.accounts.wrapped_mint.to_account_info().data_is_empty();

        let recipient = ctx.accounts.recipient.key();

        // Optional freshness
        if expiry != 0 {
            require!(
                Clock::get()?.unix_timestamp <= expiry as i64,
                SimpleErr::Expired
            );
        }

        // Extract keys for payload (don't update registry yet - validate VAA first)
        let mint_key = ctx.accounts.wrapped_mint.key();
        
        // Build payload based on first mint vs subsequent mint
        let (action, payload, decimals_val, wrapped_name, wrapped_symbol, uri_val) = if is_first_mint {
            // FIRST MINT - Require full metadata
            let dec = decimals.ok_or(SimpleErr::InvalidDecimals)?;
            let name_str = name.ok_or(SimpleErr::InvalidMetadata)?;
            let symbol_str = symbol.ok_or(SimpleErr::InvalidMetadata)?;
            let uri_str = uri.ok_or(SimpleErr::InvalidMetadata)?;

            // Validate metadata
            require!(dec <= 18, SimpleErr::InvalidDecimals);
            require!(name_str.len() > 0 && name_str.len() <= 24, SimpleErr::InvalidMetadata);
            require!(symbol_str.len() > 0 && symbol_str.len() <= 9, SimpleErr::InvalidMetadata);
            require!(uri_str.len() <= 200, SimpleErr::InvalidMetadata);

            let wrapped_n = format!("Wrapped {}", name_str);
            let wrapped_s = format!("w{}", symbol_str);

            // Payload: amount (8) + recipient (32) + contract_id_len (2) + contract_id (var)
            //          + decimals (1) + name_len (1) + name (var) + symbol_len (1) + symbol (var)
            //          + uri_len (2) + uri (var) + price (8) + liquidity (8) + tier (1)
            let name_bytes = name_str.as_bytes();
            let symbol_bytes = symbol_str.as_bytes();
            let uri_bytes = uri_str.as_bytes();

            let mut p = Vec::with_capacity(
                8 + 32 + 2 + zera_contract_id.len() + 1 + 1 + name_bytes.len() + 1 + symbol_bytes.len() + 2 + uri_bytes.len() + 8 + 8 + 1
            );
            p.extend_from_slice(&amount.to_be_bytes());
            p.extend_from_slice(recipient.as_ref());
            p.extend_from_slice(&(zera_contract_id.len() as u16).to_be_bytes());
            p.extend_from_slice(&zera_contract_id);
            p.push(dec);
            p.push(name_bytes.len() as u8);
            p.extend_from_slice(name_bytes);
            p.push(symbol_bytes.len() as u8);
            p.extend_from_slice(symbol_bytes);
            p.extend_from_slice(&(uri_bytes.len() as u16).to_be_bytes());
            p.extend_from_slice(uri_bytes);
            p.extend_from_slice(&usd_price_nano.to_be_bytes());
            p.extend_from_slice(&liquidity_usd_nano.to_be_bytes());
            p.push(tier);

            (ACTION_MINT_WRAPPED_INIT, p, dec, wrapped_n, wrapped_s, uri_str)
        } else {
            // SUBSEQUENT MINT - Minimal payload (no metadata needed)
            // Payload: amount (8) + recipient (32) + contract_id_len (2) + contract_id (var) + price (8) + liquidity (8) + tier (1)
            let mut p = Vec::with_capacity(8 + 32 + 2 + zera_contract_id.len() + 8 + 8 + 1);
            p.extend_from_slice(&amount.to_be_bytes());
            p.extend_from_slice(recipient.as_ref());
            p.extend_from_slice(&(zera_contract_id.len() as u16).to_be_bytes());
            p.extend_from_slice(&zera_contract_id);
            p.extend_from_slice(&usd_price_nano.to_be_bytes());
            p.extend_from_slice(&liquidity_usd_nano.to_be_bytes());
            p.push(tier);

            (ACTION_MINT_WRAPPED, p, 0, String::new(), String::new(), String::new())
        };


        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_TOKEN_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        let seeds: &[&[u8]] = &[VERIFIED_SEED, &expected_hash];

        let (used_pda, _bump) = Pubkey::find_program_address(seeds, &EXPECTED_CORE_ID);

        require_keys_eq!(
            used_pda,
            ctx.accounts.used_marker.key(),
            SimpleErr::BadUsedMarkerPda
        );
        require_keys_eq!(
            *ctx.accounts.used_marker.owner,
            EXPECTED_CORE_ID,
            SimpleErr::BadUsedMarkerOwner
        );
        require!(
            ctx.accounts.used_marker.lamports() > 0,
            SimpleErr::BadUsedMarkerLamports
        );

        let released_seeds: &[&[u8]] = &[RELEASED_SEED, &expected_hash];
        let (released_pda, released_bump) =
            Pubkey::find_program_address(released_seeds, ctx.program_id);
        require_keys_eq!(
            released_pda,
            ctx.accounts.redeemed_marker.key(),
            SimpleErr::BadRedeemedMarkerPda
        );
        require!(
            ctx.accounts.redeemed_marker.lamports() == 0,
            SimpleErr::BadRedeemedMarkerLamports
        );

        // VAA verified! Now safe to create/update TokenRegistration and check rate limits

        // Derive expected TokenRegistration PDA
        let (expected_registration, registration_bump) = Pubkey::find_program_address(
            &[TOKEN_REGISTRATION_SEED, mint_key.as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(
            expected_registration,
            ctx.accounts.token_registration.key(),
            SimpleErr::BadTokenRegistration
        );

        // Determine decimals for rate limit check
        let decimals_for_rate_limit = if is_first_mint {
            decimals_val
        } else {
            // Subsequent mint - deserialize existing mint to get decimals
            let mint_account_info = ctx.accounts.wrapped_mint.to_account_info();
            let mint_data = mint_account_info.try_borrow_data()?;
            // Mint account: decimals is at offset 44 (after authority fields)
            mint_data[44]
        };

        // Create or update TokenRegistration
        if ctx.accounts.token_registration.to_account_info().data_is_empty() {
            // FIRST MINT - Create TokenRegistration
            let registration_lamports = Rent::get()?.minimum_balance(8 + TokenRegistration::SIZE);
            let registration_seeds: &[&[u8]] = &[
                TOKEN_REGISTRATION_SEED,
                mint_key.as_ref(),
                &[registration_bump],
            ];

            anchor_lang::system_program::create_account(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::CreateAccount {
                        from: ctx.accounts.payer.to_account_info(),
                        to: ctx.accounts.token_registration.to_account_info(),
                    },
                    &[registration_seeds],
                ),
                registration_lamports,
                (8 + TokenRegistration::SIZE) as u64,
                ctx.program_id,
            )?;

            // Initialize TokenRegistration
            let mut registration_data = ctx.accounts.token_registration.try_borrow_mut_data()?;
            let registration = TokenRegistration {
                mint: mint_key,
                tier,
                usd_price_nano,
                liquidity_usd_nano,
                current_hour: (Clock::get()?.unix_timestamp / 3600) as u64,
                hourly_buckets: [0; 24],
                current_bucket_index: 0,
            };
            // Write discriminator and data
            registration.try_serialize(&mut &mut registration_data[..])?;
        } else {
            // SUBSEQUENT MINT - Update TokenRegistration
            let mut registration_data = ctx.accounts.token_registration.try_borrow_mut_data()?;
            let mut registration = TokenRegistration::try_deserialize(&mut &registration_data[..])?;

            registration.usd_price_nano = usd_price_nano;
            registration.liquidity_usd_nano = liquidity_usd_nano;
            registration.tier = tier;

            registration.try_serialize(&mut &mut registration_data[..])?;
        }

        // Load registration for rate limit check
        let registration_data = ctx.accounts.token_registration.try_borrow_data()?;
        let registration = TokenRegistration::try_deserialize(&mut &registration_data[..])?;
        drop(registration_data);

        // Calculate effective USD value for rate limiting (in nano-dollars)
        let effective_usd_nano = calculate_effective_value(
            amount,
            decimals_for_rate_limit,
            &registration,
        );

        // Update per-token rate limit buckets (need mutable borrow)
        {
            let mut registration_data = ctx.accounts.token_registration.try_borrow_mut_data()?;
            let mut registration = TokenRegistration::try_deserialize(&mut &registration_data[..])?;

            // Rotate buckets if needed and update
            let current_time = Clock::get()?.unix_timestamp;
            let current_hour = (current_time / 3600) as u64;

            if current_hour != registration.current_hour {
                let hours_elapsed = current_hour.saturating_sub(registration.current_hour);
                if hours_elapsed >= 24 {
                    registration.hourly_buckets = [0; 24];
                    registration.current_bucket_index = 0;
                } else {
                    for _ in 0..hours_elapsed {
                        registration.current_bucket_index = (registration.current_bucket_index + 1) % 24;
                        registration.hourly_buckets[registration.current_bucket_index as usize] = 0;
                    }
                }
                registration.current_hour = current_hour;
            }

            // Check per-token limit (liquidity_usd_nano serves as the rate limit)
            let current_net_flow: i64 = registration.hourly_buckets.iter().sum();
            let flow_delta = effective_usd_nano as i64; // incoming = positive
            let new_net_flow = current_net_flow + flow_delta;

            let limit = registration.liquidity_usd_nano as i64;
            if new_net_flow.abs() > limit {
                msg!("❌ Per-token rate limit exceeded: new_net_flow={} limit={}", new_net_flow, limit);
                return Err(error!(SimpleErr::TokenRateLimitExceeded));
            }

            // Update bucket
            registration.hourly_buckets[registration.current_bucket_index as usize] += flow_delta;
            msg!("✅ Per-token rate limit check passed: net_flow={} (token: {})", new_net_flow, registration.mint);

            registration.try_serialize(&mut &mut registration_data[..])?;
        }

        // Check and update global rate limit
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            effective_usd_nano as i64,
            false, // is_outgoing (this is incoming)
            false, // check_single_tx_limit (no check for incoming)
        )?;

        // Validate wrapped mint PDA [b"mint", hash(zera_contract_id)]
        // Hash the contract ID to get fixed-length seed
        let contract_id_hash = hash(&zera_contract_id).to_bytes();
        let (expected_mint, mint_bump) =
            Pubkey::find_program_address(&[MINT_SEED, &contract_id_hash], ctx.program_id);
        require_keys_eq!(
            expected_mint,
            ctx.accounts.wrapped_mint.key(),
            SimpleErr::BadMintPda
        );

        // Validate metadata PDA from Metaplex
        let wrapped_mint_key = ctx.accounts.wrapped_mint.key();
        let metadata_seeds = &[
            b"metadata",
            mpl_token_metadata::ID.as_ref(),
            wrapped_mint_key.as_ref(),
        ];
        let (expected_metadata, _) = Pubkey::find_program_address(
            metadata_seeds,
            &mpl_token_metadata::ID
        );
        require_keys_eq!(
            expected_metadata,
            ctx.accounts.metadata.key(),
            SimpleErr::BadMetadataPda
        );

        // Validate mint authority PDA [b"mint_authority", wrapped_mint]
        let (expected_mint_auth, mint_auth_bump) = Pubkey::find_program_address(
            &[MINT_AUTH_SEED, ctx.accounts.wrapped_mint.key().as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(
            expected_mint_auth,
            ctx.accounts.mint_authority.key(),
            SimpleErr::BadMintAuthority
        );

        // Validate bridge_info PDA [b"bridge_info", wrapped_mint]
        let (expected_bridge_info, bridge_info_bump) = Pubkey::find_program_address(
            &[BRIDGE_INFO_SEED, ctx.accounts.wrapped_mint.key().as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(
            expected_bridge_info,
            ctx.accounts.bridge_info.key(),
            SimpleErr::BadBridgeInfo
        );

        // Validate recipient_ata PDA
        let expected_recipient_ata = anchor_spl::associated_token::get_associated_token_address(
            &recipient,
            &ctx.accounts.wrapped_mint.key(),
        );
        require_keys_eq!(
            expected_recipient_ata,
            ctx.accounts.recipient_ata.key(),
            SimpleErr::BadTokenAccount
        );
        
        // If recipient_ata already exists, validate its owner
        if !ctx.accounts.recipient_ata.to_account_info().data_is_empty() {
            let recipient_ata_data = ctx.accounts.recipient_ata.to_account_info();
            let owner = Pubkey::try_from(&recipient_ata_data.data.borrow()[32..64])
                .map_err(|_| SimpleErr::BadTokenAccountOwner)?;
            require_keys_eq!(owner, recipient, SimpleErr::BadRecipient);
        }

        // Check if mint exists - if not, this is the first mint
        if ctx.accounts.wrapped_mint.to_account_info().data_is_empty() {
            // First mint - initialize mint, metadata, and bridge_info
            // CRITICAL: bridge_info MUST be empty too, preventing re-initialization attacks
            require!(
                ctx.accounts.bridge_info.to_account_info().data_is_empty(),
                SimpleErr::BadBridgeInfo
            );
            
 
            let lamports = Rent::get()?.minimum_balance(82); // Mint account size
            let mint_signer_seeds: &[&[u8]] = &[MINT_SEED, &contract_id_hash, &[mint_bump]];
            
            anchor_lang::system_program::create_account(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::CreateAccount {
                        from: ctx.accounts.payer.to_account_info(),
                        to: ctx.accounts.wrapped_mint.to_account_info(),
                    },
                    &[mint_signer_seeds],
                ),
                lamports,
                82,
                &token::ID,
            )?;

        
            let cpi_accounts = token::InitializeMint {
                mint: ctx.accounts.wrapped_mint.to_account_info(),
                rent: ctx.accounts.rent.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
            
            token::initialize_mint(cpi_ctx, decimals_val, &expected_mint_auth, None)?;
            
            //TODO : ADD BACK FOR DEVNET/MAINNET
            /* COMMENTED OUT FOR LOCALNET - Uncomment for devnet/mainnet: */
            
            CreateV1CpiBuilder::new(&ctx.accounts.metadata_program.to_account_info())
                .metadata(&ctx.accounts.metadata.to_account_info())
                .mint(&ctx.accounts.wrapped_mint.to_account_info(), false)
                .authority(&ctx.accounts.mint_authority.to_account_info())
                .payer(&ctx.accounts.payer.to_account_info())
                .update_authority(&ctx.accounts.mint_authority.to_account_info(), false)
                .system_program(&ctx.accounts.system_program.to_account_info())
                .sysvar_instructions(&ctx.accounts.sysvar_instructions.to_account_info())
                .spl_token_program(Some(&ctx.accounts.token_program.to_account_info()))
                .name(wrapped_name.clone())
                .symbol(wrapped_symbol.clone())
                .uri(uri_val.clone())
                .seller_fee_basis_points(0)
                .decimals(decimals_val)
                .token_standard(TokenStandard::Fungible)
                .print_supply(PrintSupply::Zero)
                .invoke_signed(&[&[
                    MINT_AUTH_SEED,
                    ctx.accounts.wrapped_mint.key().as_ref(),
                    &[mint_auth_bump],
                ]])?;
    

            let bridge_info_lamports = Rent::get()?.minimum_balance(BridgeTokenInfo::MAX_SIZE);
            let wrapped_mint_key_for_bridge = ctx.accounts.wrapped_mint.key();
            let bridge_info_seeds: &[&[u8]] = &[
                BRIDGE_INFO_SEED,
                wrapped_mint_key_for_bridge.as_ref(),
                &[bridge_info_bump],
            ];
            
            // Create the bridge_info account
            anchor_lang::system_program::create_account(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::CreateAccount {
                        from: ctx.accounts.payer.to_account_info(),
                        to: ctx.accounts.bridge_info.to_account_info(),
                    },
                    &[bridge_info_seeds],
                ),
                bridge_info_lamports,
                BridgeTokenInfo::MAX_SIZE as u64,
                ctx.program_id,
            )?;

            // Serialize and write BridgeTokenInfo data
            let bridge_info_data = BridgeTokenInfo {
                mint: ctx.accounts.wrapped_mint.key(),
                zera_contract_id: zera_contract_id.clone(),
                source_chain: "Zera".to_string(),
                first_minted_at: Clock::get()?.unix_timestamp,
                decimals: decimals_val,
            };
            
            let bridge_info_account = ctx.accounts.bridge_info.to_account_info();
            let mut data = bridge_info_account.try_borrow_mut_data()?;
            bridge_info_data.try_serialize(&mut &mut data[..])?;
        } else {
    
            // Parse mint authority from mint account data manually
            // Mint layout: mint_authority (COption<Pubkey>) at offset 0
            let mint_account = ctx.accounts.wrapped_mint.to_account_info();
            let mint_data = mint_account.data.borrow();
            require!(mint_data.len() >= 82, SimpleErr::BadMintPda); // Standard mint size
            
            // Check COption discriminator (0 = None, 1 = Some)
            // COption uses 4-byte discriminant, but we only check first byte
            require!(mint_data[0] == 1, SimpleErr::MintAuthorityNotSet); // Must have authority
            
            // Read the mint authority pubkey (bytes 4-36, after 4-byte COption discriminant)
            let mint_authority = Pubkey::try_from(&mint_data[4..36])
                .map_err(|_| SimpleErr::MintAuthorityParseError)?;
            
            require_keys_eq!(mint_authority, expected_mint_auth, SimpleErr::MintAuthorityMismatch);

            // CRITICAL: bridge_info must exist for subsequent mints
            require!(
                !ctx.accounts.bridge_info.to_account_info().data_is_empty(),
                SimpleErr::BadBridgeInfo
            );

            // Deserialize and validate bridge_info
            let bridge_info_account_info = ctx.accounts.bridge_info.to_account_info();
            let bridge_info_data = bridge_info_account_info.data.borrow();
            let stored_info: BridgeTokenInfo = BridgeTokenInfo::try_deserialize(&mut &bridge_info_data[..])
                .map_err(|_| SimpleErr::BadBridgeInfo)?;
            
            // Validate bridge info matches
            require_keys_eq!(
                stored_info.mint,
                ctx.accounts.wrapped_mint.key(),
                SimpleErr::BadBridgeInfo
            );
            require!(
                stored_info.zera_contract_id == zera_contract_id,
                SimpleErr::ContractIdMismatch
            );
        }

        // Create recipient ATA if it doesn't exist
        if ctx.accounts.recipient_ata.to_account_info().data_is_empty() {
            anchor_spl::associated_token::create(CpiContext::new(
                ctx.accounts.associated_token_program.to_account_info(),
                anchor_spl::associated_token::Create {
                    payer: ctx.accounts.payer.to_account_info(),
                    associated_token: ctx.accounts.recipient_ata.to_account_info(),
                    authority: ctx.accounts.recipient.to_account_info(),
                    mint: ctx.accounts.wrapped_mint.to_account_info(),
                    system_program: ctx.accounts.system_program.to_account_info(),
                    token_program: ctx.accounts.token_program.to_account_info(),
                },
            ))?;
        }

        // Mint tokens to recipient using mint_authority PDA
        let wrapped_mint_key_for_mint = ctx.accounts.wrapped_mint.key();
        let mint_auth_signer_seeds: &[&[u8]] = &[
            MINT_AUTH_SEED,
            wrapped_mint_key_for_mint.as_ref(),
            &[mint_auth_bump],
        ];
        
        token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.wrapped_mint.to_account_info(),
                    to: ctx.accounts.recipient_ata.to_account_info(),
                    authority: ctx.accounts.mint_authority.to_account_info(),
                },
                &[mint_auth_signer_seeds],
            ),
            amount,
        )?;

        // Create redeemed marker to prevent replay
        let lamports = Rent::get()?.minimum_balance(0);
        let released_signer_seeds: &[&[u8]] = &[RELEASED_SEED, &expected_hash, &[released_bump]];
        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::CreateAccount {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.redeemed_marker.to_account_info(),
                },
                &[released_signer_seeds],
            ),
            lamports,
            0,
            ctx.program_id,
        )?;

        if is_first_mint {
            msg!(
                r#"{{"event":"Mint_Wrapped_Init","version":"{}","recipient":"{}","mint":"{}","zera_contract_id":"{}","amount":"{}","decimals":"{}","name":"{}","symbol":"{}","txn_hash":"{}","event_index":"{}"}}"#,
                version,
                recipient,
                ctx.accounts.wrapped_mint.key(),
                String::from_utf8_lossy(&zera_contract_id),
                amount,
                decimals_val,
                wrapped_name,
                wrapped_symbol,
                to_hex(&txn_hash),
                event_index
            );
        } else {
            msg!(
                r#"{{"event":"Mint_Wrapped","version":"{}","recipient":"{}","mint":"{}","zera_contract_id":"{}","amount":"{}","txn_hash":"{}","event_index":"{}"}}"#,
                version,
                recipient,
                ctx.accounts.wrapped_mint.key(),
                String::from_utf8_lossy(&zera_contract_id),
                amount,
                to_hex(&txn_hash),
                event_index
            );
        }

        Ok(())
    }

    pub fn burn_wrapped(
        ctx: Context<BurnWrapped>,
        amount: u64,
        zera_recipient: Vec<u8>,
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);

        // Validate Zera recipient address format (base58, max 64 chars)
        let zera_recipient_str = validate_zera_address(&zera_recipient, 64)?;

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;

        // Check pause (outgoing operation - block if level >= 2)
        check_pause(&router_cfg_data, 2)?;

        // Block exit-only tokens (tier 2) - these can only receive mints, not burn back
        require!(
            ctx.accounts.token_registration.tier != TIER_EXIT_ONLY,
            SimpleErr::TokenExitOnly
        );

        // Calculate effective USD value using TokenRegistration (in nano-dollars)
        let effective_usd_nano = calculate_effective_value(
            amount,
            ctx.accounts.wrapped_mint.decimals,
            &ctx.accounts.token_registration,
        );

        // 1. Check single TX limit (global, outbound only)
        if effective_usd_nano > router_cfg_data.single_tx_limit_usd_nano {
            msg!("❌ Single transaction limit exceeded: {} > {}", effective_usd_nano, router_cfg_data.single_tx_limit_usd_nano);
            return Err(error!(SimpleErr::SingleTxLimitExceeded));
        }

        // 2. Check and update per-token 24h rate limit
        check_and_update_token_rate_limit(
            &mut ctx.accounts.token_registration,
            effective_usd_nano as i64,
            true, // is_outgoing
        )?;

        // 3. Check and update global 24h rate limit
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            effective_usd_nano as i64,
            true,  // is_outgoing
            false, // single tx already checked above
        )?;

        // Validate bridge_info exists (this is a wrapped token)
        require!(
            !ctx.accounts.bridge_info.to_account_info().data_is_empty(),
            SimpleErr::BadBridgeInfo
        );

        // Deserialize and validate bridge_info
        let bridge_info_account_info = ctx.accounts.bridge_info.to_account_info();
        let bridge_info_data = bridge_info_account_info.data.borrow();
        let stored_info: BridgeTokenInfo = BridgeTokenInfo::try_deserialize(&mut &bridge_info_data[..])
            .map_err(|_| SimpleErr::BadBridgeInfo)?;

        // Validate the mint matches bridge_info
        require_keys_eq!(
            stored_info.mint,
            ctx.accounts.wrapped_mint.key(),
            SimpleErr::BadBridgeInfo
        );

        // Validate user's ATA
        let expected_user_ata = anchor_spl::associated_token::get_associated_token_address(
            &ctx.accounts.authority.key(),
            &ctx.accounts.wrapped_mint.key(),
        );
        require_keys_eq!(
            expected_user_ata,
            ctx.accounts.user_ata.key(),
            SimpleErr::BadTokenAccount
        );

        // Validate user owns the ATA
        require_keys_eq!(
            ctx.accounts.user_ata.owner,
            ctx.accounts.authority.key(),
            SimpleErr::BadTokenAccountOwner
        );

        // Validate mint authority PDA
        let (expected_mint_auth, _mint_auth_bump) = Pubkey::find_program_address(
            &[MINT_AUTH_SEED, ctx.accounts.wrapped_mint.key().as_ref()],
            ctx.program_id,
        );
        require_keys_eq!(
            expected_mint_auth,
            ctx.accounts.mint_authority.key(),
            SimpleErr::BadMintAuthority
        );


        token::burn(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Burn {
                    mint: ctx.accounts.wrapped_mint.to_account_info(),
                    from: ctx.accounts.user_ata.to_account_info(),
                    authority: ctx.accounts.authority.to_account_info(),
                },
            ),
            amount,
        )?;

        // Log the burn event for off-chain indexing
        msg!(
            r#"{{"event":"Burn_Wrapped","version":"{}", "authority":"{}", "mint":"{}","zera_contract_id":"{}","amount":"{}","zera_address":"{}","solana_sender":"{}"}}"#,
            PROGRAM_VERSION,
            ctx.accounts.authority.key(),
            ctx.accounts.wrapped_mint.key(),
            String::from_utf8_lossy(&stored_info.zera_contract_id),
            amount,
            zera_recipient_str,
            ctx.accounts.authority.key()
        );

        Ok(())
    }

    pub fn execute_reset_rate_limit(
        ctx: Context<ExecuteResetRateLimit>,
        nonce: [u8; 32],
    ) -> Result<()> {
        // Verify admin action marker exists in core program
        let seeds: &[&[u8]] = &[VERIFIED_ADMIN_SEED, &nonce];
        let (admin_marker_pda, _bump) = Pubkey::find_program_address(seeds, &EXPECTED_CORE_ID);
        
        require_keys_eq!(
            admin_marker_pda,
            ctx.accounts.admin_action_marker.key(),
            SimpleErr::BadAdminActionMarker
        );
        require_keys_eq!(
            *ctx.accounts.admin_action_marker.owner,
            EXPECTED_CORE_ID,
            SimpleErr::BadAdminActionMarker
        );
        require!(
            ctx.accounts.admin_action_marker.lamports() > 0,
            SimpleErr::BadAdminActionMarker
        );

        // Reset all buckets to 0
        ctx.accounts.rate_limit_state.hourly_buckets = [0; 24];
        ctx.accounts.rate_limit_state.current_bucket_index = 0;
        ctx.accounts.rate_limit_state.current_hour = (Clock::get()?.unix_timestamp / 3600) as u64;

        msg!("✅ Rate limit state reset successfully");
        msg!("All buckets cleared, current_hour set to {}", ctx.accounts.rate_limit_state.current_hour);

        Ok(())
    }
    
}

// Core program's RouterConfig structure (read-only view)
/// Read-only copy of core bridge's RouterConfig for pause checking
/// NOTE: Must match core bridge's RouterConfig struct exactly!
#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct CoreRouterConfig {
    pub guardians: Vec<Pubkey>,
    pub guardian_threshold: u8,
    pub version: u32,
    pub cfg_bump: u8,
    pub signer_bump: u8,
    pub pause_level: u8,            // 0=Active, 1=IncomingOnly, 2=Complete
    pub pause_expiry: i64,          // Unix timestamp, 0=indefinite
    pub rate_limit_usd_nano: u64,   // 24-hour rate limit in nano-dollars (10^-9 USD)
    pub single_tx_limit_usd_nano: u64, // Per-transaction limit in nano-dollars
}

fn check_pause(cfg: &CoreRouterConfig, required_level: u8) -> Result<()> {
    let current_level = if cfg.pause_level > 0 && cfg.pause_expiry > 0 {
        // Check if timed pause has expired
        let current_time = Clock::get()?.unix_timestamp;
        if current_time >= cfg.pause_expiry {
            msg!("Timed pause expired, allowing operation");
            0 // Auto-unpause
        } else {
            cfg.pause_level
        }
    } else {
        cfg.pause_level
    };
    
    require!(current_level < required_level, SimpleErr::BridgePaused);
    Ok(())
}

// Helper to load and validate router_cfg from core program
fn load_router_cfg(router_cfg_info: &AccountInfo) -> Result<CoreRouterConfig> {
    // Owner is verified by #[account(owner = EXPECTED_CORE_ID)] constraint
    // Deserialize, skipping 8-byte discriminator (different program has different discriminator)
    let mut data_slice = &router_cfg_info.data.borrow()[8..];
    CoreRouterConfig::deserialize(&mut data_slice)
        .map_err(|_| error!(SimpleErr::BadRouterConfig))
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use core::fmt::Write;
        let _ = write!(&mut s, "{:02x}", b);
    }
    s
}

/// Validates a Zera address format (base58 encoded).
///
/// Valid Zera addresses must:
/// - Be non-empty and at most max_len bytes
/// - Be valid UTF-8
/// - Contain only valid base58 characters (alphanumeric except 0, O, I, l)
///
/// Base58 alphabet: 123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz
///
/// Returns the validated address as a String if valid.
fn validate_zera_address(address_bytes: &[u8], max_len: usize) -> Result<String> {
    // Check length bounds
    require!(
        !address_bytes.is_empty() && address_bytes.len() <= max_len,
        SimpleErr::InvalidZeraAddressLength
    );

    // Validate UTF-8 encoding
    let address_str = String::from_utf8(address_bytes.to_vec())
        .map_err(|_| error!(SimpleErr::InvalidZeraAddressFormat))?;

    // Validate all characters are valid base58
    // Base58 alphabet excludes: 0, O, I, l (to avoid visual ambiguity)
    for b in address_str.bytes() {
        let is_valid_base58 = matches!(b,
            b'1'..=b'9' |                           // 1-9 (no 0)
            b'A'..=b'H' | b'J'..=b'N' | b'P'..=b'Z' | // A-Z except I and O
            b'a'..=b'k' | b'm'..=b'z'   // a-z except l
        );

        if !is_valid_base58 {
            return Err(error!(SimpleErr::InvalidZeraAddressFormat));
        }
    }

    Ok(address_str)
}

// Compute body hash exactly like core
fn vaa_body_hash(
    version: u8,
    domain: &[u8],
    action: u8,
    timestamp: u64,
    expiry: u64,
    txn_hash: [u8; 32],
    event_index: u32,
    target_program: Pubkey,
    payload: &[u8],
) -> [u8; 32] {
    let mut buf = Vec::with_capacity(1 + domain.len() + 1 + 8 + 8 + 32 + 4 + 32 + payload.len());
    buf.push(version);
    buf.extend_from_slice(domain);
    buf.push(action);
    buf.extend_from_slice(&timestamp.to_be_bytes());
    buf.extend_from_slice(&expiry.to_be_bytes());
    buf.extend_from_slice(&txn_hash);
    buf.extend_from_slice(&event_index.to_be_bytes());
    buf.extend_from_slice(&target_program.as_ref());
    buf.extend_from_slice(payload);

    hash(&buf).to_bytes()
}

// Get USD value from guardian-attested prices in registry
// Returns USD value in nano-dollars, or 0 if not in registry
fn get_usd_value(
    amount: u64,
    decimals: u8,
    mint: &Pubkey,
    registry: &TokenPriceRegistry,
) -> Result<u64> {
    // Look up price in registry
    let price_nano = registry.entries.iter()
        .find(|e| &e.mint == mint)
        .map(|e| e.usd_price_nano)
        .unwrap_or(0);

    if price_nano == 0 {
        return Ok(0);
    }

    // Calculate USD value in nano-dollars: (amount * price_nano) / (10^decimals)
    // Use u128 to prevent overflow
    let amount_u128 = amount as u128;
    let price_u128 = price_nano as u128;
    let decimals_divisor = 10u128.pow(decimals as u32);

    let usd_nano = (amount_u128 * price_u128) / decimals_divisor;

    // Cap at u64::MAX for safety
    Ok(usd_nano.min(u64::MAX as u128) as u64)
}

// Check and update rate limits
// is_outgoing: true for lock/burn (subtract), false for release/mint (add)
fn check_and_update_rate_limit(
    rate_limit_state: &mut RateLimitState,
    router_cfg: &CoreRouterConfig,
    amount_usd: i64,
    is_outgoing: bool,
    check_single_tx_limit: bool,
) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    let current_hour = (current_time / 3600) as u64;

    // Check single transaction limit (only for outgoing)
    if check_single_tx_limit && is_outgoing {
        let abs_amount = amount_usd.abs() as u64;
        if abs_amount > router_cfg.single_tx_limit_usd_nano {
            msg!("❌ Single transaction limit exceeded: {} > {}", abs_amount, router_cfg.single_tx_limit_usd_nano);
            return Err(error!(SimpleErr::SingleTxLimitExceeded));
        }
    }

    // Rotate buckets if we've moved to a new hour
    if current_hour != rate_limit_state.current_hour {
        let hours_elapsed = current_hour.saturating_sub(rate_limit_state.current_hour);
        
        if hours_elapsed >= 24 {
            // More than 24 hours passed, reset all buckets
            rate_limit_state.hourly_buckets = [0; 24];
            rate_limit_state.current_bucket_index = 0;
        } else {
            // Rotate buckets forward
            for _ in 0..hours_elapsed {
                rate_limit_state.current_bucket_index = (rate_limit_state.current_bucket_index + 1) % 24;
                rate_limit_state.hourly_buckets[rate_limit_state.current_bucket_index as usize] = 0;
            }
        }
        
        rate_limit_state.current_hour = current_hour;
    }

    // Calculate current net flow across all 24 buckets
    let current_net_flow: i64 = rate_limit_state.hourly_buckets.iter().sum();
    
    // Calculate what the new net flow would be
    let flow_delta = if is_outgoing { -amount_usd } else { amount_usd };
    let new_net_flow = current_net_flow + flow_delta;

    // Check if new flow would exceed limits
    let limit = router_cfg.rate_limit_usd_nano as i64;
    if new_net_flow.abs() > limit {
        msg!("❌ Rate limit exceeded: new_net_flow={} limit={}", new_net_flow, limit);
        msg!("   Current flow: {} | Delta: {} | Direction: {}",
            current_net_flow, flow_delta, if is_outgoing { "OUT" } else { "IN" });
        return Err(error!(SimpleErr::RateLimitExceeded));
    }

    // Update current bucket
    rate_limit_state.hourly_buckets[rate_limit_state.current_bucket_index as usize] += flow_delta;

    msg!("✅ Rate limit check passed: net_flow={} (was {})", new_net_flow, current_net_flow);

    Ok(())
}

/// Calculate effective USD value for rate limiting, applying tier and liquidity caps
/// Returns value in nano-dollars (10^-9 USD)
/// Returns 0 for Tier 0 tokens or delisted tokens (price = 0)
fn calculate_effective_value(
    amount: u64,
    decimals: u8,
    registration: &TokenRegistration,
) -> u64 {
    // Tier 0 (unpriced) = always $0 (doesn't affect rate limits)
    // Tier 2 (exit-only) = always $0 (releases don't affect rate limits)
    if registration.tier == TIER_UNPRICED || registration.tier == TIER_EXIT_ONLY {
        return 0;
    }

    // Delisted (price = 0) = $0
    if registration.usd_price_nano == 0 {
        return 0;
    }

    // Calculate raw USD value in nano-dollars: (amount * price_nano) / 10^decimals
    let amount_u128 = amount as u128;
    let price_u128 = registration.usd_price_nano as u128;
    let decimals_divisor = 10u128.pow(decimals as u32);
    let raw_usd_nano = (amount_u128 * price_u128) / decimals_divisor;

    // Cap by liquidity (prevents price manipulation attacks)
    let effective = raw_usd_nano.min(registration.liquidity_usd_nano as u128);

    effective.min(u64::MAX as u128) as u64
}

/// Check and update per-token rate limit
/// Mirrors the global rate limit logic but operates on TokenRegistration
fn check_and_update_token_rate_limit(
    registration: &mut TokenRegistration,
    amount_usd: i64,
    is_outgoing: bool,
) -> Result<()> {
    let current_time = Clock::get()?.unix_timestamp;
    let current_hour = (current_time / 3600) as u64;

    // Rotate buckets if we've moved to a new hour
    if current_hour != registration.current_hour {
        let hours_elapsed = current_hour.saturating_sub(registration.current_hour);

        if hours_elapsed >= 24 {
            // More than 24 hours passed, reset all buckets
            registration.hourly_buckets = [0; 24];
            registration.current_bucket_index = 0;
        } else {
            // Rotate buckets forward
            for _ in 0..hours_elapsed {
                registration.current_bucket_index = (registration.current_bucket_index + 1) % 24;
                registration.hourly_buckets[registration.current_bucket_index as usize] = 0;
            }
        }

        registration.current_hour = current_hour;
    }

    // Calculate current net flow across all 24 buckets
    let current_net_flow: i64 = registration.hourly_buckets.iter().sum();

    // Calculate what the new net flow would be
    let flow_delta = if is_outgoing { -amount_usd } else { amount_usd };
    let new_net_flow = current_net_flow + flow_delta;

    // Check if new flow would exceed per-token limit (liquidity_usd_nano serves as the rate limit)
    let limit = registration.liquidity_usd_nano as i64;
    if new_net_flow.abs() > limit {
        msg!("❌ Per-token rate limit exceeded: new_net_flow={} limit={}", new_net_flow, limit);
        msg!("   Token: {} | Current flow: {} | Delta: {}",
            registration.mint, current_net_flow, flow_delta);
        return Err(error!(SimpleErr::TokenRateLimitExceeded));
    }

    // Update current bucket
    registration.hourly_buckets[registration.current_bucket_index as usize] += flow_delta;

    msg!("✅ Per-token rate limit check passed: net_flow={} (token: {})", new_net_flow, registration.mint);

    Ok(())
}

#[derive(Accounts)]
pub struct InitializeRateLimitState<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + RateLimitState::SIZE,
        seeds = [RATE_LIMIT_STATE_SEED],
        bump
    )]
    pub rate_limit_state: Account<'info, RateLimitState>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LockSol<'info> {
    /// CHECK: verified by address constraint
    #[account(address = EXPECTED_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    /// Core program's router config (for pause checking and rate limits)
    /// CHECK: deserialization verified in function via load_router_cfg
    #[account(owner = EXPECTED_CORE_ID)]
    pub router_cfg: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// Payer's wSOL associated token account (created lazily if needed)
    /// CHECK: validated in function via ATA derivation
    #[account(mut)]
    pub payer_wsol_ata: UncheckedAccount<'info>,
    /// Vault wSOL ATA (router_signer's wSOL ATA, same vault as lock_spl uses)
    /// CHECK: validated in function via ATA derivation
    #[account(mut)]
    pub vault_ata: UncheckedAccount<'info>,
    /// CHECK: router_signer PDA of this program [b"router_signer"]
    #[account(seeds=[ROUTER_SIGNER_SEED], bump)]
    pub router_signer: UncheckedAccount<'info>,
    /// wSOL mint account
    #[account(address = WSOL_MINT)]
    pub wsol_mint: Account<'info, Mint>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// Token price registry PDA [b"token_price_registry"]
    #[account(seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct InitializeTokenPriceRegistry<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + TokenPriceRegistry::SIZE,
        seeds = [TOKEN_PRICE_REGISTRY_SEED],
        bump
    )]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RegisterToken<'info> {
    /// CHECK: verified by address constraint
    #[account(address = EXPECTED_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    /// The token mint being registered
    pub mint: Account<'info, Mint>,

    /// TokenRegistration PDA - created on first registration, updated on subsequent
    /// ///////
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + TokenRegistration::SIZE,
        seeds = [TOKEN_REGISTRATION_SEED, mint.key().as_ref()],
        bump
    )]
    pub token_registration: Account<'info, TokenRegistration>,

    /// Core's verified transfer marker (proves guardian signatures)
    /// CHECK: validated against expected PDA in instruction
    pub used_marker: UncheckedAccount<'info>,

    /// CHECK: verified by address constraint
    #[account(address = crate::id())]
    pub target_program: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct RequestTokenRegistration<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// The token mint to request registration for
    pub mint: Account<'info, Mint>,

    /// PendingTokenRegistration PDA - created if needed, updated if exists
    #[account(
        init_if_needed,
        payer = payer,
        space = 8 + PendingTokenRegistration::SIZE,
        seeds = [PENDING_REGISTRATION_SEED, mint.key().as_ref()],
        bump
    )]
    pub pending_registration: Account<'info, PendingTokenRegistration>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct LockSpl<'info> {
    /// CHECK: verified by address constraint
    #[account(address = EXPECTED_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    /// Core program's router config (for pause checking and rate limits)
    /// CHECK: deserialization verified in function via load_router_cfg
    #[account(owner = EXPECTED_CORE_ID)]
    pub router_cfg: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    #[account(mut)]
    pub from_ata: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    /// CHECK: router_signer PDA of this program [b"router_signer"]
    #[account(seeds=[ROUTER_SIGNER_SEED], bump)]
    pub router_signer: UncheckedAccount<'info>,
    /// CHECK: ATA for router_signer + mint; created/checked at runtime
    #[account(mut)]
    pub vault_ata: UncheckedAccount<'info>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// TokenRegistration PDA for this token [b"token_registration", mint]
    #[account(
        mut,
        seeds = [TOKEN_REGISTRATION_SEED, mint.key().as_ref()],
        bump,
        constraint = token_registration.mint == mint.key() @ SimpleErr::BadTokenRegistration
    )]
    pub token_registration: Account<'info, TokenRegistration>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseSpl<'info> {
    #[account(address = EXPECTED_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    /// Core program's router config (for pause checking)
    /// CHECK: deserialization verified in function via load_router_cfg
    #[account(owner = EXPECTED_CORE_ID)]
    pub router_cfg: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub mint: Account<'info, Mint>,
    /// CHECK: router_signer PDA of this program [b"router_signer"]
    #[account(seeds=[ROUTER_SIGNER_SEED], bump)]
    pub router_signer: UncheckedAccount<'info>,
    /// CHECK: vault ATA (validated at runtime)
    #[account(mut)]
    pub vault_ata: Account<'info, TokenAccount>,
    /// CHECK: recipient wallet - validated as owner of recipient_ata by init_if_needed
    pub recipient: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = payer,
        associated_token::mint = mint,
        associated_token::authority = recipient
    )]
    pub recipient_ata: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: core used marker PDA [b"verified_transfer", expected_hash]
    #[account()]
    pub used_marker: UncheckedAccount<'info>,
    /// CHECK: token redeemed marker PDA [b"released_transfer", expected_hash]
    #[account(mut)]
    pub redeemed_marker: UncheckedAccount<'info>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// TokenRegistration PDA for this token [b"token_registration", mint]
    #[account(
        mut,
        seeds = [TOKEN_REGISTRATION_SEED, mint.key().as_ref()],
        bump,
        constraint = token_registration.mint == mint.key() @ SimpleErr::BadTokenRegistration
    )]
    pub token_registration: Account<'info, TokenRegistration>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,

    /// CHECK: verified by address constraint
    #[account(address = crate::id())]
    pub target_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct MintWrapped<'info> {
    #[account(address = EXPECTED_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    /// Core program's router config (for pause checking)
    /// CHECK: deserialization verified in function via load_router_cfg
    #[account(owner = EXPECTED_CORE_ID)]
    pub router_cfg: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: wrapped mint PDA [b"mint", hash(zera_contract_id)]
    #[account(mut)]
    pub wrapped_mint: UncheckedAccount<'info>,
    /// CHECK: mint authority PDA [b"mint_authority", wrapped_mint]
    #[account()]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK: Metaplex metadata account
    #[account(mut)]
    pub metadata: UncheckedAccount<'info>,
    /// CHECK: Bridge token info PDA [b"bridge_info", wrapped_mint] - manually initialized
    #[account(mut)]
    pub bridge_info: UncheckedAccount<'info>,
    /// CHECK: recipient account (validated in function)
    #[account()]
    pub recipient: UncheckedAccount<'info>,
    /// CHECK: recipient's ATA (validated in function)
    #[account(mut)]
    pub recipient_ata: UncheckedAccount<'info>,
    /// CHECK: core used marker PDA [b"verified_transfer", expected_hash]
    #[account()]
    pub used_marker: UncheckedAccount<'info>,
    /// CHECK: token redeemed marker PDA [b"released_transfer", expected_hash]
    #[account(mut)]
    pub redeemed_marker: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// CHECK: Metaplex Token Metadata Program
    #[account(address = mpl_token_metadata::ID)]
    pub metadata_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
    /// CHECK: Sysvar Instructions
    #[account(address = anchor_lang::solana_program::sysvar::instructions::ID)]
    pub sysvar_instructions: UncheckedAccount<'info>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// TokenRegistration PDA for this wrapped token [b"token_registration", mint]
    /// CHECK: Created on first mint if needed, validated in function
    #[account(mut)]
    pub token_registration: UncheckedAccount<'info>,

    /// CHECK: verified by address constraint
    #[account(address = crate::id())]
    pub target_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct ExecuteResetRateLimit<'info> {
    /// CHECK: Admin action marker from core program [b"verified_admin", nonce]
    #[account()]
    pub admin_action_marker: UncheckedAccount<'info>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
}

#[derive(Accounts)]
pub struct BurnWrapped<'info> {
    /// Core program's router config (for pause checking and rate limits)
    /// CHECK: deserialization verified in function via load_router_cfg
    #[account(owner = EXPECTED_CORE_ID)]
    pub router_cfg: UncheckedAccount<'info>,
    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: wrapped mint (validated via bridge_info)
    #[account(mut)]
    pub wrapped_mint: Account<'info, Mint>,
    /// CHECK: mint authority PDA [b"mint_authority", wrapped_mint]
    #[account()]
    pub mint_authority: UncheckedAccount<'info>,
    /// CHECK: Bridge token info PDA [b"bridge_info", wrapped_mint]
    #[account()]
    pub bridge_info: UncheckedAccount<'info>,
    /// User's token account to burn from
    #[account(mut)]
    pub user_ata: Account<'info, TokenAccount>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// TokenRegistration PDA for this wrapped token [b"token_registration", mint]
    #[account(
        mut,
        seeds = [TOKEN_REGISTRATION_SEED, wrapped_mint.key().as_ref()],
        bump,
        constraint = token_registration.mint == wrapped_mint.key() @ SimpleErr::BadTokenRegistration
    )]
    pub token_registration: Account<'info, TokenRegistration>,
    pub token_program: Program<'info, Token>,
}

#[account]
pub struct BridgeTokenInfo {
    pub mint: Pubkey,                   // The wrapped mint this info belongs to
    pub zera_contract_id: Vec<u8>,      // Original Zera contract ID (e.g., "$ZRA+0000")
    pub source_chain: String,            // Source chain name (e.g., "Zera")
    pub first_minted_at: i64,           // Timestamp when first minted
    pub decimals: u8,                   // Token decimals
}

impl BridgeTokenInfo {
    pub const MAX_SIZE: usize = 8 + // discriminator
        32 +                        // mint pubkey
        4 + 64 +                    // zera_contract_id (vec with max 64 bytes)
        4 + 32 +                    // source_chain (string with max 32 bytes)
        8 +                         // first_minted_at (i64)
        1;                          // decimals (u8)
}

#[account]
pub struct RateLimitState {
    pub current_hour: u64,           // Current hour (Unix timestamp / 3600)
    pub hourly_buckets: [i64; 24],   // Net flow per hour in nano-dollars (signed)
    pub current_bucket_index: u8,    // Which bucket we're currently in (0-23)
}

impl RateLimitState {
    pub const SIZE: usize = 8 + // discriminator
        8 +                     // current_hour (u64)
        (8 * 24) +              // hourly_buckets (i64 * 24)
        1;                      // current_bucket_index (u8)
}

#[account]
pub struct TokenPriceRegistry {
    pub entries: Vec<TokenPriceEntry>,
}

impl TokenPriceRegistry {
    pub const MAX_ENTRIES: usize = 50;
    pub const SIZE: usize = 4 + (TokenPriceEntry::SIZE * Self::MAX_ENTRIES); // vec length + entries (discriminator added separately in init)
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Debug)]
pub struct TokenPriceEntry {
    pub mint: Pubkey,              // Token mint (System::id() for native SOL)
    pub usd_price_nano: u64,       // Guardian-attested price in nano-dollars (10^-9 USD)
    pub last_updated: i64,         // Timestamp when guardians last updated
}

impl TokenPriceEntry {
    pub const SIZE: usize = 32 + 8 + 8; // mint + usd_price_nano + last_updated
}


/// Per-token registration account for SPL tokens bridging to/from Zera
/// Created via guardian VAA for native SPL tokens, or on first mint for wrapped tokens
/// PDA: [b"token_registration", mint.as_ref()]
#[account]
pub struct TokenRegistration {
    // === Token Info ===
    pub mint: Pubkey,                    // 32 bytes - The token mint
    pub tier: u8,                        // 1 byte - 0=Unpriced ($0 value), 1=Priced (uses liquidity as rate limit)
    pub usd_price_nano: u64,             // 8 bytes - Guardian-attested price in nano-dollars (0 = delisted)
    pub liquidity_usd_nano: u64,         // 8 bytes - Guardian-attested liquidity = per-token 24h rate limit

    // === Per-Token Rate Limit State ===
    pub current_hour: u64,               // 8 bytes - Current hour for bucket rotation
    pub hourly_buckets: [i64; 24],       // 192 bytes - Net flow per hour in nano-dollars (signed)
    pub current_bucket_index: u8,        // 1 byte - Which bucket we're currently in (0-23)
}

impl TokenRegistration {
    pub const SIZE: usize =
        32 +  // mint
        1 +   // tier
        8 +   // usd_price_nano
        8 +   // liquidity_usd_nano (also serves as rate limit)
        8 +   // current_hour
        192 + // hourly_buckets (8 * 24)
        1;    // current_bucket_index
    // Total: 250 bytes (+ 8 discriminator = 258 bytes for account)
}

/// PendingTokenRegistration - User-initiated registration request.
/// Guardians observe these on-chain and decide whether to approve or reject.
/// If approved: guardians submit register_token VAA.
/// If rejected: no action taken, user can cancel to reclaim rent.
#[account]
pub struct PendingTokenRegistration {
    pub mint: Pubkey,  // 32 bytes - The token mint requesting registration
}

impl PendingTokenRegistration {
    pub const SIZE: usize = 32;  // mint only (+ 8 discriminator = 40 bytes for account)
}

#[error_code]
pub enum SimpleErr {
    #[msg("Invalid amount")]
    InvalidAmount,
    #[msg("Invalid zera address length")]
    InvalidZeraAddressLength,
    #[msg("Bad proxy program")]
    BadProxyProgram,
    #[msg("Bad vault PDA")]
    BadVaultPda,
    #[msg("Bad vault owner")]
    BadVaultOwner,
    #[msg("Bad router signer")]
    BadRouterSigner,
    #[msg("Expired")]
    Expired,
    #[msg("Invalid length")]
    InvalidLength,
    #[msg("Bad domain")]
    BadDomain,
    #[msg("Wrong target program")]
    WrongTargetProgram,
    #[msg("Wrong core program")]
    WrongCoreProgram,
    #[msg("Bad used marker")]
    BadUsedMarker,
    #[msg("Not foreign mint")]
    NotForeignMint,
    #[msg("Bad redeemed marker")]
    BadRedeemedMarker,
    #[msg("Insufficient vault balance")]
    InsufficientVaultBalance,
    #[msg("Bad used marker PDA")]
    BadUsedMarkerPda,
    #[msg("Bad used marker owner")]
    BadUsedMarkerOwner,
    #[msg("Bad used marker lamports")]
    BadUsedMarkerLamports,
    #[msg("Bad redeemed marker PDA")]
    BadRedeemedMarkerPda,
    #[msg("Bad redeemed marker lamports")]
    BadRedeemedMarkerLamports,
    #[msg("Bad recipient")]
    BadRecipient,
    #[msg("Bad token account")]
    BadTokenAccount,
    #[msg("Bad token account owner")]
    BadTokenAccountOwner,
    #[msg("Bad mint PDA")]
    BadMintPda,
    #[msg("Bad mint authority")]
    BadMintAuthority,
    #[msg("Mint authority not set (COption is None)")]
    MintAuthorityNotSet,
    #[msg("Failed to parse mint authority from mint account data")]
    MintAuthorityParseError,
    #[msg("Mint authority mismatch - stored authority doesn't match expected PDA")]
    MintAuthorityMismatch,
    #[msg("Invalid contract ID")]
    InvalidContractId,
    #[msg("Invalid decimals")]
    InvalidDecimals,
    #[msg("Invalid metadata")]
    InvalidMetadata,
    #[msg("Bad metadata PDA")]
    BadMetadataPda,
    #[msg("Bad bridge info")]
    BadBridgeInfo,
    #[msg("Contract ID mismatch")]
    ContractIdMismatch,
    #[msg("Insufficient amount")]
    InsufficientAmount,
    #[msg("Bridge is paused")]
    BridgePaused,
    #[msg("Failed to deserialize router config")]
    BadRouterConfig,
    #[msg("Rate limit exceeded")]
    RateLimitExceeded,
    #[msg("Single transaction limit exceeded")]
    SingleTxLimitExceeded,
    #[msg("Bad rate limit state PDA")]
    BadRateLimitStatePda,
    #[msg("Bad admin action marker")]
    BadAdminActionMarker,
    #[msg("Token price registry is full")]
    RegistryFull,
    #[msg("Recipient is not a user wallet (must be owned by System Program)")]
    InvalidRecipientType,
    #[msg("Token not registered - must register via guardian VAA first")]
    TokenNotRegistered,
    #[msg("Token is delisted - new locks not allowed")]
    TokenDelisted,
    #[msg("Token is exit-only (tier 2) - only releases/mints allowed, no new locks/burns")]
    TokenExitOnly,
    #[msg("Bad token registration PDA")]
    BadTokenRegistration,
    #[msg("Invalid tier value - must be 0, 1, or 2")]
    InvalidTier,
    #[msg("Token already registered")]
    TokenAlreadyRegistered,
    #[msg("Per-token rate limit exceeded")]
    TokenRateLimitExceeded,
    #[msg("Invalid Zera address format - must be valid base58 characters")]
    InvalidZeraAddressFormat,
}
