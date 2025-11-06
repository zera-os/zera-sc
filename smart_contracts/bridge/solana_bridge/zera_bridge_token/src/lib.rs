use anchor_lang::prelude::*;
use anchor_lang::solana_program::hash::hash;
use anchor_lang::solana_program::program_option::COption;
use anchor_lang::solana_program::program::invoke_signed;
use anchor_lang::solana_program::system_instruction::transfer;
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{self, Burn, Mint, Token, TokenAccount};
use mpl_token_metadata::instructions::CreateV1CpiBuilder;
use mpl_token_metadata::types::{PrintSupply, TokenStandard};

declare_id!("WrapZ8f88HR8waSp7wR8Vgc68z4hKj3p3i2b81oeSxR");

const ROUTER_SIGNER_SEED: &[u8] = b"router_signer";
const EXPECTED_CORE_ID: Pubkey = pubkey!("zera3giq7oM9QJaD6mY1ajGmakv9TZcax5Giky99HD8");
const MINT_AUTH_SEED: &[u8] = b"mint_authority";
const MINT_SEED: &[u8] = b"mint";
const VAULT_SEED: &[u8] = b"vault";
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

    //WORKING / should pause on level 2 - pause working
    pub fn lock_sol(ctx: Context<LockSol>, amount: u64, zera_address: Vec<u8>) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);
        require!(
            zera_address.len() <= 64,
            SimpleErr::InvalidZeraAddressLength
        );

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (outgoing operation - block if level >= 2)
        check_pause(&router_cfg_data, 2)?;

        // Check rate limits (outgoing operation with single tx limit check)
        // NEW ********************************************
        let amount_usd_cents = get_usd_value(
            amount,
            9, // SOL has 9 decimals
            &System::id(), // Native SOL uses system_program as mint identifier
            &ctx.accounts.token_price_registry,
        )? as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_cents,
            true, // is_outgoing
            true, // check_single_tx_limit
        )?;
        //******************** RATE LIMIT CHECK END *********************************** */
        let zera_address_str = String::from_utf8(zera_address)
            .map_err(|_| SimpleErr::InvalidZeraAddressLength)?;

        // Enforce/initialize the vault PDA owned by this program
        let (expected_vault, _bump) = Pubkey::find_program_address(&[VAULT_SEED], &crate::id());
        let vault_bump = ctx.bumps.vault;
        require_keys_eq!(
            expected_vault,
            ctx.accounts.vault.key(),
            SimpleErr::BadVaultPda
        );
        if ctx.accounts.vault.to_account_info().lamports() == 0 {
            let lamports = Rent::get()?.minimum_balance(0);
            let signer_seeds: &[&[u8]] = &[VAULT_SEED, &[vault_bump]];
            anchor_lang::system_program::create_account(
                CpiContext::new_with_signer(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::CreateAccount {
                        from: ctx.accounts.payer.to_account_info(),
                        to: ctx.accounts.vault.to_account_info(),
                    },
                    &[signer_seeds],
                ),
                lamports,
                0,
                &crate::id(),
            )?;
        } else {
            require_keys_eq!(*ctx.accounts.vault.owner, crate::id(),SimpleErr::BadVaultOwner);
        }

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.payer.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            ),
            amount,
        )?;

        msg!(
            r#"{{"event":"Lock_SOL","version":"{}","payer":"{}","vault":"{}","amount":"{}","zera_address":"{}", "solana_sender":"{}"}}"#,
            "1",
            ctx.accounts.payer.key(),
            ctx.accounts.vault.key(),
            amount,
            zera_address_str,
            ctx.accounts.payer.key()
        );

        Ok(())
    }
    //WORKING / should pause on level 2 - pause working
    pub fn lock_spl(ctx: Context<LockSpl>, amount: u64, zera_address: Vec<u8>) -> Result<()> {
        require!(
            zera_address.len() <= 64,
            SimpleErr::InvalidZeraAddressLength
        );
        let zera_address_str = String::from_utf8(zera_address)
            .map_err(|_| SimpleErr::InvalidZeraAddressLength)?;

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (outgoing operation - block if level >= 2)
        check_pause(&router_cfg_data, 2)?;

        require!(amount > 0, SimpleErr::InvalidAmount);

        // Check rate limits (outgoing operation with single tx limit check)
        // NEW ********************************************
        let amount_usd_cents = get_usd_value(
            amount,
            ctx.accounts.mint.decimals,
            &ctx.accounts.mint.key(),
            &ctx.accounts.token_price_registry,
        )? as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_cents,
            true, // is_outgoing
            true, // check_single_tx_limit
        )?;
        //******************** RATE LIMIT CHECK END *********************************** */
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
            "1",
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
    pub fn release_sol(
        ctx: Context<ReleaseSol>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        event_index: u32,
        amount: u64,
        recipient: Pubkey,
        usd_price_cents: u64, // Guardian-attested price in USD cents
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (incoming operation - block if level >= 1)
        check_pause(&router_cfg_data, 1)?;

        require_keys_eq!(
            recipient,
            ctx.accounts.recipient.key(),
            SimpleErr::BadRecipient
        );

        // Optional freshness
        if expiry != 0 {
            require!(
                Clock::get()?.unix_timestamp <= expiry as i64,
                SimpleErr::Expired
            );
        }

        // Build payload: amount + recipient + usd_price_cents
        let mut payload = Vec::with_capacity(8 + 32 + 8);
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(recipient.as_ref());
        payload.extend_from_slice(&usd_price_cents.to_be_bytes());

        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_TOKEN_DOMAIN,
            ACTION_RELEASE_SOL,
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

        // VAA verified! Now safe to update registry and check rate limits
        update_token_price_registry(
            &mut ctx.accounts.token_price_registry,
            System::id(), // Native SOL
            usd_price_cents,
        )?;

        // Track rate limits (incoming operation, NO single tx limit check)
        // Calculate USD value directly from VAA's guardian-attested price
        let amount_u128 = amount as u128;
        let price_u128 = usd_price_cents as u128;
        let usd_cents = (amount_u128 * price_u128) / 1_000_000_000; // SOL has 9 decimals
        let amount_usd_cents = usd_cents.min(u64::MAX as u128) as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_cents,
            false, // is_outgoing (this is incoming)
            false, // check_single_tx_limit (no check for incoming)
        )?;

        let (expected_vault, _bump) =
            Pubkey::find_program_address(&[VAULT_SEED], &crate::id());
        let vault_bump = ctx.bumps.vault;

        require_keys_eq!(
            expected_vault,
            ctx.accounts.vault.key(),
            SimpleErr::BadVaultPda
        );
        require_keys_eq!(
            *ctx.accounts.vault.owner,
            crate::id(),
            SimpleErr::BadVaultOwner
        );
        
        // Ensure vault maintains rent-exempt minimum after transfer
        let rent_minimum = Rent::get()?.minimum_balance(0);
        require!(
            ctx.accounts.vault.lamports() >= amount + rent_minimum,
            SimpleErr::InsufficientVaultBalance
        );

        // IMPORTANT: Perform all CPIs BEFORE manual lamport transfers
        // This avoids "sum of account balances before and after instruction do not match" error
        // Create redeemed marker to prevent replay (CPI must happen first)
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

        // NOW perform manual lamport transfer AFTER all CPIs are complete
        // Manual transfer required because vault is owned by this program, not system program
        **ctx.accounts.vault.to_account_info().try_borrow_mut_lamports()? -= amount;
        **ctx.accounts.recipient.to_account_info().try_borrow_mut_lamports()? += amount;

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
        usd_price_cents: u64, // Guardian-attested price in USD cents
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (incoming operation - block if level >= 1)
        check_pause(&router_cfg_data, 1)?;

        // Optional freshness
        if expiry != 0 {
            require!(
                Clock::get()?.unix_timestamp <= expiry as i64,
                SimpleErr::Expired
            );
        }

        // Build payload: amount (8) + recipient (32) + mint (32) + usd_price_cents (8)
        let mut payload = Vec::with_capacity(8 + 32 + 32 + 8);
        payload.extend_from_slice(&amount.to_be_bytes());
        payload.extend_from_slice(ctx.accounts.recipient.key().as_ref());
        payload.extend_from_slice(ctx.accounts.mint.key().as_ref());
        payload.extend_from_slice(&usd_price_cents.to_be_bytes());

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

        // VAA verified! Now safe to update registry and check rate limits
        update_token_price_registry(
            &mut ctx.accounts.token_price_registry,
            ctx.accounts.mint.key(),
            usd_price_cents,
        )?;

        // Track rate limits (incoming operation, NO single tx limit check)
        // Calculate USD value directly from VAA's guardian-attested price
        let amount_u128 = amount as u128;
        let price_u128 = usd_price_cents as u128;
        let decimals_divisor = 10u128.pow(ctx.accounts.mint.decimals as u32);
        let usd_cents = (amount_u128 * price_u128) / decimals_divisor;
        let amount_usd_cents = usd_cents.min(u64::MAX as u128) as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_cents,
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
        usd_price_cents: u64, // Guardian-attested price in USD cents
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);
        require!(
            zera_contract_id.len() > 0 && zera_contract_id.len() <= 64,
            SimpleErr::InvalidContractId
        );

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (incoming operation - block if level >= 1)
        check_pause(&router_cfg_data, 1)?;

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
            //          + uri_len (2) + uri (var) + usd_price_cents (8)
            let name_bytes = name_str.as_bytes();
            let symbol_bytes = symbol_str.as_bytes();
            let uri_bytes = uri_str.as_bytes();
            
            let mut p = Vec::with_capacity(
                8 + 32 + 2 + zera_contract_id.len() + 1 + 1 + name_bytes.len() + 1 + symbol_bytes.len() + 2 + uri_bytes.len() + 8
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
            p.extend_from_slice(&usd_price_cents.to_be_bytes());

            (ACTION_MINT_WRAPPED_INIT, p, dec, wrapped_n, wrapped_s, uri_str)
        } else {
            // SUBSEQUENT MINT - Minimal payload (no metadata needed)
            // Payload: amount (8) + recipient (32) + contract_id_len (2) + contract_id (var) + usd_price_cents (8)
            let mut p = Vec::with_capacity(8 + 32 + 2 + zera_contract_id.len() + 8);
            p.extend_from_slice(&amount.to_be_bytes());
            p.extend_from_slice(recipient.as_ref());
            p.extend_from_slice(&(zera_contract_id.len() as u16).to_be_bytes());
            p.extend_from_slice(&zera_contract_id);
            p.extend_from_slice(&usd_price_cents.to_be_bytes());

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

        // VAA verified! Now safe to update registry and check rate limits
        update_token_price_registry(
            &mut ctx.accounts.token_price_registry,
            mint_key,
            usd_price_cents,
        )?;

        // Determine decimals for rate limit check
        let decimals_for_rate_limit = if is_first_mint {
            decimals_val
        } else {
            // Subsequent mint - deserialize existing mint to get decimals
            let mint_account_info = ctx.accounts.wrapped_mint.to_account_info();
            let mint_data = mint_account_info.try_borrow_data()?;
            // Mint account: decimals is at offset 44 (after authority fields)
            // See: https://docs.rs/spl-token/latest/spl_token/state/struct.Mint.html
            mint_data[44]
        };

        // Track rate limits (incoming operation, NO single tx limit check)
        // Calculate USD value directly from VAA's guardian-attested price
        let amount_u128 = amount as u128;
        let price_u128 = usd_price_cents as u128;
        let decimals_divisor = 10u128.pow(decimals_for_rate_limit as u32);
        let usd_cents = (amount_u128 * price_u128) / decimals_divisor;
        let amount_usd_cents = usd_cents.min(u64::MAX as u128) as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_cents,
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

        //TODO: add event logging
        Ok(())
    }
    //WORKING / should pause on level 2 - pause working
    pub fn burn_wrapped(
        ctx: Context<BurnWrapped>,
        amount: u64,
        zera_recipient: Vec<u8>,
    ) -> Result<()> {
        require!(amount > 0, SimpleErr::InvalidAmount);
        require!(
            zera_recipient.len() > 0 && zera_recipient.len() <= 128,
            SimpleErr::InvalidZeraAddressLength
        );

        // Load and validate router_cfg from core program
        let router_cfg_data = load_router_cfg(&ctx.accounts.router_cfg.to_account_info())?;
        
        // Check pause (outgoing operation - block if level >= 2)
        check_pause(&router_cfg_data, 2)?;

        // NEW ********************************************
        // Check rate limits (outgoing operation with single tx limit check)
        let amount_usd_cents = get_usd_value(
            amount,
            ctx.accounts.wrapped_mint.decimals,
            &ctx.accounts.wrapped_mint.key(),
            &ctx.accounts.token_price_registry,
        )? as i64;
        check_and_update_rate_limit(
            &mut ctx.accounts.rate_limit_state,
            &router_cfg_data,
            amount_usd_cents,
            true, // is_outgoing
            true, // check_single_tx_limit
        )?;
        //******************** RATE LIMIT CHECK END *********************************** */
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
            r#"{{"event":"Burn_Wrapped","version":"1", "authority":"{}", "mint":"{}","zera_contract_id":"{}","amount":"{}","zera_address":"{}","solana_sender":"{}"}}"#,
            ctx.accounts.authority.key(),
            ctx.accounts.wrapped_mint.key(),
            String::from_utf8_lossy(&stored_info.zera_contract_id),
            amount,
            String::from_utf8_lossy(&zera_recipient),
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
    pub pause_level: u8,      // 0=Active, 1=IncomingOnly, 2=Complete
    pub pause_expiry: i64,    // Unix timestamp, 0=indefinite
    pub rate_limit_usd: u64,  // 24-hour rate limit in cents
    pub single_tx_limit_usd: u64, // Per-transaction limit in cents
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

// Update or add entry in token price registry
// If usd_price_cents is 0, skip adding to registry (treat as $0 token)
fn update_token_price_registry(
    registry: &mut TokenPriceRegistry,
    mint: Pubkey,
    usd_price_cents: u64,
) -> Result<()> {
    // If price is 0, don't add to registry (will default to $0 on lookup)
    if usd_price_cents == 0 {
        msg!("Token price is $0, not adding to registry: {}", mint);
        return Ok(());
    }

    let now = Clock::get()?.unix_timestamp;
    
    // Check if entry exists, update it
    if let Some(entry) = registry.entries.iter_mut().find(|e| e.mint == mint) {
        entry.usd_price_cents = usd_price_cents;
        entry.last_updated = now;
        msg!("Updated price for mint {}: ${}.{:02}", 
            mint, 
            usd_price_cents / 100, 
            usd_price_cents % 100
        );
    } else {
        // Add new entry
        require!(
            registry.entries.len() < TokenPriceRegistry::MAX_ENTRIES,
            SimpleErr::RegistryFull
        );
        registry.entries.push(TokenPriceEntry {
            mint,
            usd_price_cents,
            last_updated: now,
        });
        msg!("Added new price for mint {}: ${}.{:02}", 
            mint, 
            usd_price_cents / 100, 
            usd_price_cents % 100
        );
    }
    
    Ok(())
}

// Get USD value from guardian-attested prices in registry
// Returns USD value in cents, or 0 if not in registry
fn get_usd_value(
    amount: u64,
    decimals: u8,
    mint: &Pubkey,
    registry: &TokenPriceRegistry,
) -> Result<u64> {
    // Look up price in registry
    let price_cents = registry.entries.iter()
        .find(|e| &e.mint == mint)
        .map(|e| e.usd_price_cents)
        .unwrap_or(0);
    
    if price_cents == 0 {
        return Ok(0);
    }

    // Calculate USD value: (amount * price_cents) / (10^decimals)
    // Use u128 to prevent overflow
    let amount_u128 = amount as u128;
    let price_u128 = price_cents as u128;
    let decimals_divisor = 10u128.pow(decimals as u32);
    
    let usd_cents = (amount_u128 * price_u128) / decimals_divisor;
    
    // Cap at u64::MAX for safety
    Ok(usd_cents.min(u64::MAX as u128) as u64)
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
        if abs_amount > router_cfg.single_tx_limit_usd {
            msg!("❌ Single transaction limit exceeded: {} > {}", abs_amount, router_cfg.single_tx_limit_usd);
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
    let limit = router_cfg.rate_limit_usd as i64;
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
    /// CHECK: vault PDA [b"vault"]
    #[account(mut, seeds=[VAULT_SEED], bump)]
    pub vault: UncheckedAccount<'info>,
    /// CHECK: core used marker PDA [b"used", chain_le, expected_hash]
    #[account()]
    pub used_marker: UncheckedAccount<'info>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// Token price registry PDA [b"token_price_registry"]
    #[account(seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
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
    /// Price feed registry PDA [b"token_price_registry"]
    #[account(seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct ReleaseSol<'info> {
    #[account(address = EXPECTED_CORE_ID)]
    pub core_program: UncheckedAccount<'info>,
    /// Core program's router config (for pause checking)
    /// CHECK: deserialization verified in function via load_router_cfg
    #[account(owner = EXPECTED_CORE_ID)]
    pub router_cfg: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    /// CHECK: vault PDA [b"vault"]
    #[account(mut, seeds=[VAULT_SEED], bump)]
    pub vault: UncheckedAccount<'info>,
    /// CHECK: core used marker PDA [b"verified_transfer", expected_hash]
    #[account()]
    pub used_marker: UncheckedAccount<'info>,
    /// CHECK: token redeemed marker PDA [b"released_transfer", expected_hash]
    #[account(mut)]
    pub redeemed_marker: UncheckedAccount<'info>,
    #[account(mut)]
    pub recipient: UncheckedAccount<'info>,
    /// Rate limit state PDA [b"rate_limit_state"]
    #[account(mut, seeds=[RATE_LIMIT_STATE_SEED], bump)]
    pub rate_limit_state: Account<'info, RateLimitState>,
    /// Price feed registry PDA [b"token_price_registry"]
    #[account(mut, seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
    #[account()]
    pub system_program: Program<'info, System>,
    
    /// CHECK: verified by address constraint
    #[account(address = crate::id())]
    pub target_program: UncheckedAccount<'info>,
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
    /// Price feed registry PDA [b"token_price_registry"]
    #[account(mut, seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
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
    /// Price feed registry PDA [b"token_price_registry"]
    #[account(mut, seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
    
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
    /// Price feed registry PDA [b"token_price_registry"]
    #[account(seeds=[TOKEN_PRICE_REGISTRY_SEED], bump)]
    pub token_price_registry: Account<'info, TokenPriceRegistry>,
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
    pub hourly_buckets: [i64; 24],   // Net flow per hour in USD cents (signed)
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
    pub usd_price_cents: u64,      // Guardian-attested price in USD cents
    pub last_updated: i64,         // Timestamp when guardians last updated
}

impl TokenPriceEntry {
    pub const SIZE: usize = 32 + 8 + 8; // mint + usd_price_cents + last_updated
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
}
