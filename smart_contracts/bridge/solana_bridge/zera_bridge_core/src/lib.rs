
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    ed25519_program,
    hash::hash,
    instruction::Instruction,
    program::invoke_signed,
    sysvar::instructions as sysvar_instructions,
};
use anchor_spl::associated_token::{self, AssociatedToken};
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer, TransferChecked};

// Replace with your real program id when building
declare_id!("zera3giq7oM9QJaD6mY1ajGmakv9TZcax5Giky99HD8");

// Seeds
const ROUTER_CFG_SEED: &[u8] = b"router_cfg";
const ROUTER_SIGNER_SEED: &[u8] = b"router_signer";
const PROXY_VAULT_SEED: &[u8] = b"vault";
const VERIFIED_SEED: &[u8] = b"verified_transfer";
const VERIFIED_ADMIN_SEED: &[u8] = b"verified_admin";

pub const ZERA_BRIDGE_TOKEN_DOMAIN: &[u8] = b"SOLANA_BRIDGE_TOKEN";
pub const ZERA_BRIDGE_GOV_DOMAIN:   &[u8] = b"SOLANA_BRIDGE_GOV";
pub const ADMIN_ACTION_DOMAIN: &[u8] = b"SOLANA_BRIDGE_ADMIN";

// Admin action types
const ACTION_RESET_RATE_LIMIT: u8 = 100;

#[program]
pub mod zera_bridge_core {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg; // mutable handle to router config PDA
        let guardian_key = "C68BgMJks69fsn5yr4cKNnYuw9yztW3vBNyk4hCyr3iE"
            .parse::<Pubkey>()
            .map_err(|_| CoreErr::InvalidGuardian)?;
        
        let guardian_key1 = "B1NgczXgVbJjJLUdbHkQ5xe6fxnzvzQk7MP7o6JqK3dp"
            .parse::<Pubkey>()
            .map_err(|_| CoreErr::InvalidGuardian)?;

        let guardian_key2 = "9aZ6ZymbUETdA9neSnLjvjj9iD8SqHfKo8L9QFtv1PGJ"
            .parse::<Pubkey>()
            .map_err(|_| CoreErr::InvalidGuardian)?;
        cfg.guardians.push(guardian_key1);
        cfg.guardians.push(guardian_key2);
        cfg.guardians.push(guardian_key);

        cfg.guardian_threshold = 2; // threshold for guardians to update guardians and post verified transfers (governance/owners)
        cfg.cfg_bump = ctx.bumps.router_cfg; // stored bump for router_cfg PDA seeds
                                             // Precompute & store signer bump
        let (_pda, signer_bump) =
            Pubkey::find_program_address(&[ROUTER_SIGNER_SEED], ctx.program_id); // derive router_signer PDA bump
        cfg.signer_bump = signer_bump; // cache signer PDA bump for invoke_signed CPIs
        cfg.version = 1; // initial proxy version for upgrades/migrations tracking
        cfg.pause_level = 0; // initially active (not paused)
        cfg.pause_expiry = 0; // no expiry
        //TODO: change back to 10M and 1M
        cfg.rate_limit_usd = 1_000_000_000; // $10M in cents (10,000,000 * 100)
        cfg.single_tx_limit_usd = 100_000_000; // $1M in cents (1,000,000 * 100)

        Ok(())
    }

    pub fn set_guardians_with_sigs(
        ctx: Context<SetImplWithSigs>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        new_guardians: Vec<Pubkey>,
        new_threshold: u8,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Enforce target program is this core program for governance updates
        require_keys_eq!(ctx.accounts.target_program.key(), crate::id(), CoreErr::NotAuthorizedInCPI);

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        // Validate new guardians and threshold
        let num = new_guardians.len();
        require!(num > 0 && num <= RouterConfig::MAX_GUARDIANS, CoreErr::TooManyGuardians);
        require!(new_threshold > 0 && (new_threshold as usize) <= num, CoreErr::GuardianSignatures);

        // Build payload on-chain: [threshold: u8][num: u8][guardians: 32*num]
        let mut payload = Vec::with_capacity(2 + 32 * num);
        payload.push(new_threshold);
        payload.push(num as u8);
        for guardian in &new_guardians {
            payload.extend_from_slice(guardian.as_ref());
        }

        let action = 0; // ACTION_SET_GUARDIANS
        let event_index = 0u32; // Always 0 for governance actions
        
        // Compute hash and verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );
        
        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Update config with new guardians
        cfg.guardians = new_guardians;
        cfg.guardian_threshold = new_threshold;

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn upgrade_token_bridge(
        ctx: Context<UpgradeTokenBridge>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
    ) -> Result<()> {
        let cfg = &ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 1; // ACTION_UPGRADE_TOKEN_BRIDGE
        let event_index = 0u32; // Always 0 for governance actions
        
        // Build payload on-chain from accounts: [buffer_address: 32 bytes, spill_address: 32 bytes]
        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(ctx.accounts.buffer.key().as_ref());
        payload.extend_from_slice(ctx.accounts.spill.key().as_ref());
        
        // Compute hash and verify guardian signatures
        // Target program is always this core program for governance actions
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            crate::id(),
            &payload,
        );
        
        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Call BPF Loader Upgradeable to perform the upgrade
        // The upgrade authority is this program's governance PDA
        let upgrade_instruction = anchor_lang::solana_program::bpf_loader_upgradeable::upgrade(
            &ctx.accounts.token_bridge_program.key(),
            &ctx.accounts.buffer.key(),
            &ctx.accounts.governance_pda.key(),
            &ctx.accounts.spill.key(),
        );

        let governance_seeds: &[&[u8]] = &[
            b"governance",
            &[ctx.bumps.governance_pda],
        ];

        invoke_signed(
            &upgrade_instruction,
            &[
                ctx.accounts.token_bridge_program_data.to_account_info(),
                ctx.accounts.token_bridge_program.to_account_info(),
                ctx.accounts.buffer.to_account_info(),
                ctx.accounts.spill.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.clock.to_account_info(),
                ctx.accounts.governance_pda.to_account_info(),
            ],
            &[governance_seeds],
        )?;

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn upgrade_self(
        ctx: Context<UpgradeSelf>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
    ) -> Result<()> {
        let cfg = &ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 5; // ACTION_UPGRADE_SELF
        let event_index = 0u32; // Always 0 for governance actions
        
        // Build payload on-chain from accounts: [buffer_address: 32 bytes, spill_address: 32 bytes]
        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(ctx.accounts.buffer.key().as_ref());
        payload.extend_from_slice(ctx.accounts.spill.key().as_ref());
        
        // Compute hash and verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            crate::id(),
            &payload,
        );
        
        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Call BPF Loader Upgradeable to upgrade THIS program
        // The upgrade authority is this program's governance PDA
        let upgrade_instruction = anchor_lang::solana_program::bpf_loader_upgradeable::upgrade(
            &crate::id(), // Upgrade THIS program
            &ctx.accounts.buffer.key(),
            &ctx.accounts.governance_pda.key(),
            &ctx.accounts.spill.key(),
        );

        let governance_seeds: &[&[u8]] = &[
            b"governance",
            &[ctx.bumps.governance_pda],
        ];

        invoke_signed(
            &upgrade_instruction,
            &[
                ctx.accounts.core_bridge_program_data.to_account_info(),
                ctx.accounts.core_bridge_program.to_account_info(),
                ctx.accounts.buffer.to_account_info(),
                ctx.accounts.spill.to_account_info(),
                ctx.accounts.rent.to_account_info(),
                ctx.accounts.clock.to_account_info(),
                ctx.accounts.governance_pda.to_account_info(),
            ],
            &[governance_seeds],
        )?;

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn pause_incoming(
        ctx: Context<SetImplWithSigs>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        duration_seconds: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 2; // ACTION_PAUSE_INCOMING
        let event_index = 0;
        
        // Build payload: [pause_level: u8, duration_seconds: u64 BE]
        let mut payload = Vec::with_capacity(1 + 8);
        payload.push(1); // pause_level = 1 (IncomingOnly)
        payload.extend_from_slice(&duration_seconds.to_be_bytes());

        // Verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Set pause level to 1 (IncomingOnly)
        cfg.pause_level = 1;
        cfg.pause_expiry = if duration_seconds == 0 {
            0 // Indefinite
        } else {
            Clock::get()?.unix_timestamp + duration_seconds as i64
        };

        msg!("Bridge paused: INCOMING ONLY (level 1)");
        if cfg.pause_expiry == 0 {
            msg!("   Duration: INDEFINITE");
        } else {
            msg!("   Auto-unpause at: {}", cfg.pause_expiry);
        }

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn pause_complete(
        ctx: Context<SetImplWithSigs>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        duration_seconds: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 3; // ACTION_PAUSE_COMPLETE
        let event_index = 0;
        
        // Build payload: [pause_level: u8, duration_seconds: u64 BE]
        let mut payload = Vec::with_capacity(1 + 8);
        payload.push(2); // pause_level = 2 (Complete)
        payload.extend_from_slice(&duration_seconds.to_be_bytes());

        // Verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Set pause level to 2 (Complete)
        cfg.pause_level = 2;
        cfg.pause_expiry = if duration_seconds == 0 {
            0 // Indefinite
        } else {
            Clock::get()?.unix_timestamp + duration_seconds as i64
        };

        msg!("Bridge paused: COMPLETE (level 2)");
        if cfg.pause_expiry == 0 {
            msg!("   Duration: INDEFINITE");
        } else {
            msg!("   Auto-unpause at: {}", cfg.pause_expiry);
        }

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn unpause(
        ctx: Context<SetImplWithSigs>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 4; // ACTION_UNPAUSE
        let event_index = 0;
        
        // Build payload: [pause_level: u8]
        let payload = vec![0]; // pause_level = 0 (Active)

        // Verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );


        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Clear pause
        cfg.pause_level = 0;
        cfg.pause_expiry = 0;

        msg!("Bridge UNPAUSED - Normal operation resumed");

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn update_rate_limit(
        ctx: Context<SetImplWithSigs>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        new_limit: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 7; // ACTION_UPDATE_RATE_LIMIT
        let event_index = 0;
        
        // Build payload: [new_limit: u64 BE]
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&new_limit.to_be_bytes());

        // Verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Update rate limit
        cfg.rate_limit_usd = new_limit;

        msg!("Rate limit updated to: {} cents (${:.2})", new_limit, new_limit as f64 / 100.0);

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn update_single_tx_limit(
        ctx: Context<SetImplWithSigs>,
        version: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        new_limit: u64,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 6; // ACTION_UPDATE_SINGLE_TX_LIMIT
        let event_index = 0;
        
        // Build payload: [new_limit: u64 BE]
        let mut payload = Vec::with_capacity(8);
        payload.extend_from_slice(&new_limit.to_be_bytes());

        // Verify guardian signatures
        let expected_hash = vaa_body_hash(
            version,
            ZERA_BRIDGE_GOV_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Update single transaction limit
        cfg.single_tx_limit_usd = new_limit;

        msg!("Single transaction limit updated to: {} cents (${:.2})", new_limit, new_limit as f64 / 100.0);

        // Mark used to prevent replay
        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn post_verified_transfer(
        ctx: Context<PostVerifiedTransfer>,
        version: u8,
        action: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
        event_index: u32,
        payload: Vec<u8>,
        payload_len: u16,
    ) -> Result<()> {
        let cfg = &mut ctx.accounts.router_cfg;
        let replay = &mut ctx.accounts.replay;

        // Check pause (incoming operation - block if level >= 1)
        check_pause(cfg, 1)?;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        require!(
            payload_len as usize == payload.len(),
            CoreErr::InvalidLength
        );

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

        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }

    pub fn post_verified_admin_action(
        ctx: Context<PostVerifiedTransfer>,
        version: u8,
        action: u8,
        timestamp: u64,
        expiry: u64,
        txn_hash: [u8; 32],
    ) -> Result<()> {
        let cfg = &ctx.accounts.router_cfg;
        let replay = &ctx.accounts.replay;

        // Optional freshness check
        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let payload = Vec::new();

        // Use nonce as event_index for admin actions (ensures uniqueness)
        let event_index = 0u32;

        let expected_hash = vaa_body_hash(
            version,
            ADMIN_ACTION_DOMAIN,
            action,
            timestamp,
            expiry,
            txn_hash,
            event_index,
            ctx.accounts.target_program.key(),
            &payload,
        );

        let ix_info = ctx.accounts.instructions.to_account_info();
        verify_guardian_sigs(&ix_info, &expected_hash, &cfg.guardians, cfg.guardian_threshold)?;

        // Create admin action marker with different seed
        let seeds: &[&[u8]] = &[VERIFIED_ADMIN_SEED, &expected_hash];
        let (used_pda, bump) = Pubkey::find_program_address(seeds, ctx.program_id);
        
        require_keys_eq!(
            used_pda,
            replay.used_marker.key(),
            CoreErr::BadReplayMarkerPda
        );
        require!(
            replay.used_marker.lamports() == 0,
            CoreErr::BadReplayMarkerLamports
        );

        // Create the marker account
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(0);
        let signer_seeds: &[&[u8]] = &[VERIFIED_ADMIN_SEED, &expected_hash, &[bump]];
        
        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                ctx.accounts.replay.system_program.to_account_info(),
                anchor_lang::system_program::CreateAccount {
                    from: replay.payer.to_account_info(),
                    to: replay.used_marker.to_account_info(),
                },
                &[signer_seeds],
            ),
            lamports,
            0,
            ctx.program_id,
        )?;

        msg!("✅ Admin action verified: action={}", action);

        Ok(())
    }

}

// Helper function to check pause state and auto-unpause if expired
fn check_pause(cfg: &RouterConfig, required_level: u8) -> Result<()> {
    let current_level = if cfg.pause_level > 0 && cfg.pause_expiry > 0 {
        // Check if timed pause has expired
        let current_time = Clock::get()?.unix_timestamp;
        if current_time >= cfg.pause_expiry {
            msg!("Timed pause expired, auto-unpausing");
            0 // Auto-unpause
        } else {
            cfg.pause_level
        }
    } else {
        cfg.pause_level
    };
    
    require!(current_level < required_level, CoreErr::BridgePaused);
    Ok(())
}

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
        // version(1) + domain(32) + action(1) + ts(8) + expiry(8) + txn_hash(32) + event_index(4) + target_program(32) + payload
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
    

    // Internal helpers
    fn create_used_marker<'info>(
        program_id: &Pubkey,
        payer_ai: AccountInfo<'info>,
        system_program_ai: AccountInfo<'info>,
        used_marker_ai: AccountInfo<'info>,
        expected_hash: &[u8; 32],
    ) -> Result<()> {
        let seeds: &[&[u8]] = &[VERIFIED_SEED, expected_hash];
        let (used_pda, bump) = Pubkey::find_program_address(seeds, program_id);

        require_keys_eq!(used_pda, *used_marker_ai.key, CoreErr::BadReplayMarkerPda);
        require!(used_marker_ai.lamports() == 0, CoreErr::BadReplayMarkerLamports);

        let lamports = Rent::get()?.minimum_balance(0);
        let signer_seeds: &[&[u8]] = &[VERIFIED_SEED, expected_hash, &[bump]];

        anchor_lang::system_program::create_account(
            CpiContext::new_with_signer(
                system_program_ai,
                anchor_lang::system_program::CreateAccount {
                    from: payer_ai,
                    to: used_marker_ai,
                },
                &[signer_seeds],
            ),
            lamports,
            0,
            program_id,
        )?;
        Ok(())
    }

    fn verify_guardian_sigs(
        ix_acc: &AccountInfo,
        expected_hash: &[u8],
        guardians: &[Pubkey],
        threshold: u8,
    ) -> Result<()> {
        msg!("🔍 Looking for {} guardian signatures", threshold);
        msg!("🔍 Configured guardians: {}", guardians.len());
        for (i, g) in guardians.iter().enumerate() {
            msg!("  Guardian {}: {}", i, g);
        }
        
        let mut uniq: std::collections::BTreeSet<Pubkey> = std::collections::BTreeSet::new();
        let mut idx = 0usize;
        let mut ed25519_ix_count = 0;
        loop {
            let loaded = sysvar_instructions::load_instruction_at_checked(idx, ix_acc);
            if loaded.is_err() {
                break;
            }
            let ix = loaded.unwrap();
            if ix.program_id == ed25519_program::id() {
                ed25519_ix_count += 1;
                msg!("🔍 Found ed25519 instruction #{}", ed25519_ix_count);
                let d = ix.data;
                if d.len() < 2 {
                    idx += 1;
                    continue;
                }
                let num = d[0] as usize;
                let mut c = 2usize;
                for s in 0..num {
                    if c + 14 > d.len() {
                        break;
                    }
                    let _sig_off = u16::from_le_bytes([d[c], d[c + 1]]) as usize;
                    c += 2;
                    let sig_ix = u16::from_le_bytes([d[c], d[c + 1]]);
                    c += 2;
                    let pk_off = u16::from_le_bytes([d[c], d[c + 1]]) as usize;
                    c += 2;
                    let pk_ix = u16::from_le_bytes([d[c], d[c + 1]]);
                    c += 2;
                    let msg_off = u16::from_le_bytes([d[c], d[c + 1]]) as usize;
                    c += 2;
                    let msg_sz = u16::from_le_bytes([d[c], d[c + 1]]) as usize;
                    c += 2;
                    let msg_ix = u16::from_le_bytes([d[c], d[c + 1]]);
                    c += 2;
                    if sig_ix != u16::MAX || pk_ix != u16::MAX || msg_ix != u16::MAX {
                        continue;
                    }
                    if pk_off + 32 > d.len() || msg_off + msg_sz > d.len() {
                        continue;
                    }
                    if msg_sz != expected_hash.len() {
                        continue;
                    }
                    let same = &d[msg_off..msg_off + msg_sz] == expected_hash;
                    if !same {
                        msg!("  ❌ Message hash mismatch");
                        continue;
                    }
                    let mut pk = [0u8; 32];
                    pk.copy_from_slice(&d[pk_off..pk_off + 32]);
                    let signer = Pubkey::new_from_array(pk);
                    msg!("  📝 Signature from: {}", signer);
                    let is_guardian = guardians.iter().any(|g| *g == signer);
                    if is_guardian {
                        msg!("  ✅ Valid guardian signature!");
                        uniq.insert(signer);
                    } else {
                        msg!("  ❌ Not a configured guardian");
                    }
                }
            }
            idx += 1;
        }
        msg!("🔍 Found {} ed25519 instructions total", ed25519_ix_count);
        msg!("🔍 Valid unique guardian signatures: {}/{}", uniq.len(), threshold);
        
        require!(
            (uniq.len() as u8) >= threshold,
            CoreErr::GuardianSignatures
        );
        Ok(())
    }

    fn to_hex(bytes: &[u8]) -> String {
        let mut s = String::with_capacity(bytes.len() * 2);
        for b in bytes {
            use core::fmt::Write;
            let _ = write!(&mut s, "{:02x}", b);
        }
        s
    }

//REPLAY ACCOUNTS
#[derive(Accounts)]
pub struct Replay<'info> {
    /// CHECK: PDA created by this program via create_used_marker with seeds [b"verified_transfer", expected_hash].
    /// Ownership and address are verified at runtime before creation/use.
    #[account(mut)]
    pub used_marker: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>, //
}

#[derive(Accounts)]
pub struct PostVerifiedTransfer<'info> {
    #[account(mut, seeds=[ROUTER_CFG_SEED], bump=router_cfg.cfg_bump)]
    pub router_cfg: Account<'info, RouterConfig>, // core program config
    /// CHECK: instructions sysvar required to read ed25519 verify instructions
    #[account(address = sysvar_instructions::id())]
    pub instructions: UncheckedAccount<'info>,
    pub replay: Replay<'info>,

    /// CHECK: target program PDA [b"zera_bridge_token"]
    #[account()]
    pub target_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct SetImplWithSigs<'info> {
    #[account(mut, seeds=[ROUTER_CFG_SEED], bump=router_cfg.cfg_bump)]
    pub router_cfg: Account<'info, RouterConfig>,
    /// CHECK: instructions sysvar required to read ed25519 verify instructions
    #[account(address = sysvar_instructions::id())]
    pub instructions: UncheckedAccount<'info>,

    pub replay: Replay<'info>,

    /// CHECK: target program PDA [b"zera_bridge_core"]
    #[account()]
    pub target_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpgradeTokenBridge<'info> {
    #[account(seeds=[ROUTER_CFG_SEED], bump=router_cfg.cfg_bump)]
    pub router_cfg: Account<'info, RouterConfig>,
    /// CHECK: instructions sysvar required to read ed25519 verify instructions
    #[account(address = sysvar_instructions::id())]
    pub instructions: UncheckedAccount<'info>,
    
    pub replay: Replay<'info>,

    /// CHECK: Governance PDA that acts as upgrade authority for token bridge
    #[account(mut, seeds=[b"governance"], bump)]
    pub governance_pda: UncheckedAccount<'info>,

    /// CHECK: Token bridge program to upgrade
    #[account(mut)]
    pub token_bridge_program: UncheckedAccount<'info>,

    /// CHECK: Token bridge program data account
    #[account(mut)]
    pub token_bridge_program_data: UncheckedAccount<'info>,

    /// CHECK: Buffer account containing new program code
    #[account(mut)]
    pub buffer: UncheckedAccount<'info>,

    /// CHECK: Spill account to receive refunded lamports
    #[account(mut)]
    pub spill: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct UpgradeSelf<'info> {
    #[account(seeds=[ROUTER_CFG_SEED], bump=router_cfg.cfg_bump)]
    pub router_cfg: Account<'info, RouterConfig>,
    /// CHECK: instructions sysvar required to read ed25519 verify instructions
    #[account(address = sysvar_instructions::id())]
    pub instructions: UncheckedAccount<'info>,
    
    pub replay: Replay<'info>,

    /// CHECK: Governance PDA that acts as upgrade authority for core bridge
    #[account(mut, seeds=[b"governance"], bump)]
    pub governance_pda: UncheckedAccount<'info>,

    /// CHECK: Core bridge program to upgrade (THIS program)
    #[account(mut, address = crate::id())]
    pub core_bridge_program: UncheckedAccount<'info>,

    /// CHECK: Core bridge program data account
    #[account(mut)]
    pub core_bridge_program_data: UncheckedAccount<'info>,

    /// CHECK: Buffer account containing new program code
    #[account(mut)]
    pub buffer: UncheckedAccount<'info>,

    /// CHECK: Spill account to receive refunded lamports
    #[account(mut)]
    pub spill: UncheckedAccount<'info>,

    pub rent: Sysvar<'info, Rent>,
    pub clock: Sysvar<'info, Clock>,
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(init, payer = payer, space = 8 + RouterConfig::SIZE, seeds=[ROUTER_CFG_SEED], bump)]
    pub router_cfg: Account<'info, RouterConfig>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct SetImpl<'info> {
    #[account(mut, seeds=[ROUTER_CFG_SEED], bump=router_cfg.cfg_bump)]
    pub router_cfg: Account<'info, RouterConfig>,
    // remaining_accounts: include at least `guardian_threshold` guardian signers
}

#[account]
pub struct RouterConfig {
    pub guardians: Vec<Pubkey>,
    pub guardian_threshold: u8,
    pub version: u32,
    pub cfg_bump: u8,
    pub signer_bump: u8,
    pub pause_level: u8,      // 0=Active, 1=IncomingOnly, 2=Complete
    pub pause_expiry: i64,    // Unix timestamp, 0=indefinite
    pub rate_limit_usd: u64,  // 24-hour rate limit in cents (e.g., 1000000000 = $10M)
    pub single_tx_limit_usd: u64, // Per-transaction limit in cents (e.g., 100000000 = $1M)
}

impl RouterConfig {
    pub const MAX_GUARDIANS: usize = 20;
    // + 4 (guardians vec length) + 32 * MAX_GUARDIANS (max elements)
    // + 1 (guardian_threshold) + 8 (last_nonce) + 4 (version) + 1 (cfg_bump) + 1 (signer_bump)
    // + 1 (pause_level) + 8 (pause_expiry) + 8 (rate_limit_usd) + 8 (single_tx_limit_usd)
    pub const SIZE: usize =  4 + (32 * Self::MAX_GUARDIANS) + 1 + 8 + 4 + 1 + 1 + 1 + 8 + 8 + 8;
}

#[error_code]
pub enum CoreErr {
    #[msg("Too many guardians")]
    TooManyGuardians,
    #[msg("Bad used marker")]
    BadReplayMarkerPda,
    #[msg("Bad used marker lamports")]
    BadReplayMarkerLamports,
    #[msg("Guardian signatures")]
    GuardianSignatures,
    #[msg("Not authorized in CPI")]
    NotAuthorizedInCPI,
    #[msg("Expired")]
    Expired,
    #[msg("Invalid length")]
    InvalidLength,
    #[msg("Invalid guardian")]
    InvalidGuardian,
    #[msg("Invalid upgrade buffer address")]
    InvalidUpgradeBuffer,
    #[msg("Invalid spill account address")]
    InvalidSpillAccount,
    #[msg("Bridge is paused")]
    BridgePaused,
}
