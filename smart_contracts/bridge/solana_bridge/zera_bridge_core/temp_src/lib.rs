
use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    bpf_loader_upgradeable,
    ed25519_program,
    hash::hash,
    program::invoke_signed,
    sysvar::instructions as sysvar_instructions,
};

declare_id!("zera3giq7oM9QJaD6mY1ajGmakv9TZcax5Giky99HD8");

const ROUTER_CFG_SEED: &[u8] = b"router_cfg";
const VERIFIED_SEED: &[u8] = b"verified_transfer";

pub const ZERA_BRIDGE_GOV_DOMAIN: &[u8] = b"SOLANA_BRIDGE_GOV";

const TEMP_AUTHORITY: Pubkey = pubkey!("s1ugzkyk8cxqTJ5jvaz9uzBbB4J1DnVwgXn9KDFX6bz");

#[program]
pub mod zera_bridge_core {
    use super::*;

    pub fn transfer_authority(ctx: Context<TransferAuthority>) -> Result<()> {
        require_keys_eq!(ctx.accounts.payer.key(), TEMP_AUTHORITY, CoreErr::NotAuthorizedInCPI);

        let governance_seeds: &[&[u8]] = &[
            b"governance",
            &[ctx.bumps.governance_pda],
        ];

        let core_set_auth = bpf_loader_upgradeable::set_upgrade_authority(
            &ctx.accounts.core_program.key(),
            &ctx.accounts.governance_pda.key(),
            Some(&TEMP_AUTHORITY),
        );

        invoke_signed(
            &core_set_auth,
            &[
                ctx.accounts.core_program_data.to_account_info(),
                ctx.accounts.governance_pda.to_account_info(),
                ctx.accounts.payer.to_account_info(),
            ],
            &[governance_seeds],
        )?;

        let token_set_auth = bpf_loader_upgradeable::set_upgrade_authority(
            &ctx.accounts.token_bridge_program.key(),
            &ctx.accounts.governance_pda.key(),
            Some(&TEMP_AUTHORITY),
        );

        invoke_signed(
            &token_set_auth,
            &[
                ctx.accounts.token_bridge_program_data.to_account_info(),
                ctx.accounts.governance_pda.to_account_info(),
                ctx.accounts.payer.to_account_info(),
            ],
            &[governance_seeds],
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

        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 1u8;
        let event_index = 0u32;

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(ctx.accounts.buffer.key().as_ref());
        payload.extend_from_slice(ctx.accounts.spill.key().as_ref());

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

        if expiry != 0 {
            require!(Clock::get()?.unix_timestamp <= expiry as i64, CoreErr::Expired);
        }

        let action = 5u8;
        let event_index = 0u32;

        let mut payload = Vec::with_capacity(64);
        payload.extend_from_slice(ctx.accounts.buffer.key().as_ref());
        payload.extend_from_slice(ctx.accounts.spill.key().as_ref());

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

        let upgrade_instruction = anchor_lang::solana_program::bpf_loader_upgradeable::upgrade(
            &crate::id(),
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

        create_used_marker(
            ctx.program_id,
            replay.payer.to_account_info(),
            replay.system_program.to_account_info(),
            replay.used_marker.to_account_info(),
            &expected_hash,
        )?;

        Ok(())
    }
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
    let mut uniq: std::collections::BTreeSet<Pubkey> = std::collections::BTreeSet::new();
    let mut idx = 0usize;
    loop {
        let loaded = sysvar_instructions::load_instruction_at_checked(idx, ix_acc);
        if loaded.is_err() {
            break;
        }
        let ix = loaded.unwrap();
        if ix.program_id == ed25519_program::id() {
            let d = ix.data;
            if d.len() < 2 {
                idx += 1;
                continue;
            }
            let num = d[0] as usize;
            let mut c = 2usize;
            for _ in 0..num {
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
                if &d[msg_off..msg_off + msg_sz] != expected_hash {
                    continue;
                }
                let mut pk = [0u8; 32];
                pk.copy_from_slice(&d[pk_off..pk_off + 32]);
                let signer = Pubkey::new_from_array(pk);
                if guardians.iter().any(|g| *g == signer) {
                    uniq.insert(signer);
                }
            }
        }
        idx += 1;
    }
    require!(
        (uniq.len() as u8) >= threshold,
        CoreErr::GuardianSignatures
    );
    Ok(())
}

#[derive(Accounts)]
pub struct Replay<'info> {
    /// CHECK: PDA verified at runtime via create_used_marker
    #[account(mut)]
    pub used_marker: UncheckedAccount<'info>,
    #[account(mut)]
    pub payer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TransferAuthority<'info> {
    #[account(mut, address = TEMP_AUTHORITY)]
    pub payer: Signer<'info>,

    /// CHECK: Governance PDA (current upgrade authority for both programs)
    #[account(seeds=[b"governance"], bump)]
    pub governance_pda: UncheckedAccount<'info>,

    /// CHECK: Core bridge program (THIS program)
    #[account(address = crate::id())]
    pub core_program: UncheckedAccount<'info>,

    /// CHECK: Core bridge ProgramData account
    #[account(mut)]
    pub core_program_data: UncheckedAccount<'info>,

    /// CHECK: Token bridge program
    pub token_bridge_program: UncheckedAccount<'info>,

    /// CHECK: Token bridge ProgramData account
    #[account(mut)]
    pub token_bridge_program_data: UncheckedAccount<'info>,

    /// CHECK: BPF Loader Upgradeable program (needed for CPI)
    #[account(address = bpf_loader_upgradeable::id())]
    pub bpf_loader_program: UncheckedAccount<'info>,
}

#[derive(Accounts)]
pub struct UpgradeTokenBridge<'info> {
    #[account(seeds=[ROUTER_CFG_SEED], bump=router_cfg.cfg_bump)]
    pub router_cfg: Account<'info, RouterConfig>,
    /// CHECK: instructions sysvar
    #[account(address = sysvar_instructions::id())]
    pub instructions: UncheckedAccount<'info>,

    pub replay: Replay<'info>,

    /// CHECK: Governance PDA
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
    /// CHECK: instructions sysvar
    #[account(address = sysvar_instructions::id())]
    pub instructions: UncheckedAccount<'info>,

    pub replay: Replay<'info>,

    /// CHECK: Governance PDA
    #[account(mut, seeds=[b"governance"], bump)]
    pub governance_pda: UncheckedAccount<'info>,

    /// CHECK: Core bridge program (THIS program)
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

#[account]
pub struct RouterConfig {
    pub guardians: Vec<Pubkey>,
    pub guardian_threshold: u8,
    pub version: u32,
    pub cfg_bump: u8,
    pub signer_bump: u8,
    pub pause_level: u8,
    pub pause_expiry: i64,
    pub rate_limit_usd: u64,
    pub single_tx_limit_usd: u64,
}

impl RouterConfig {
    pub const MAX_GUARDIANS: usize = 20;
    pub const SIZE: usize = 4 + (32 * Self::MAX_GUARDIANS) + 1 + 8 + 4 + 1 + 1 + 1 + 8 + 8 + 8;
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
    #[msg("Migration already completed")]
    AlreadyMigrated,
    #[msg("Message data required on first submission")]
    MessageDataRequired,
    #[msg("VAA hash mismatch")]
    VaaHashMismatch,
    #[msg("Hash does not match message data")]
    HashMismatch,
    #[msg("Guardian set changed during verification")]
    GuardianSetChanged,
    #[msg("Verification session expired")]
    VerificationExpired,
    #[msg("Insufficient signatures to meet threshold")]
    InsufficientSignatures,
}
