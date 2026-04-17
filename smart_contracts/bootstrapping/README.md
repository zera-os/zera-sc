# Bootstrapping Contracts

## Overview

The Zera Network bootstrapping system incentivizes early liquidity provisioning by allowing users to stake DEX LP tokens and earn ZRA rewards. Rewards are distributed based on a weight-proportional system where longer lock-up terms and specific LP token types receive higher multipliers. The total daily ZRA release decreases over 10 phases spanning approximately 11.5 years, rewarding early participants the most.

## Contract Architecture

```
┌─────────────────────────────────────────────────────┐
│              Bootstrapping System                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────────┐    ┌──────────────────┐       │
│  │  Bootstrapping   │◄───┤  Bootstrapping   │       │
│  │  Proxy           │    │  V1              │       │
│  │(bootstrapping_   │    │(bootstrapping)   │       │
│  │ proxy)           │    │                  │       │
│  └──────┬───────────┘    └────────┬─────────┘       │
│         │                         │                 │
│         │ delegates               │ derived_send    │
│         │                         ▼                 │
│         │                ┌──────────────────┐       │
│         │                │  Derived Wallet  │       │
│         │                │  ("principle")   │       │
│         │                └──────────────────┘       │
│         │                (holds staked LP tokens)   │
│         │                                           │
└─────────┴───────────────────────────────────────────┘
```

## Contracts

### 1. bootstrapping (bootstrapping_v1) - Implementation

**Location**: `bootstrapping/bootstrapping/src/lib.rs`

**Purpose**: Core logic for LP token staking, weight-based ZRA reward distribution across 10 decreasing-reward phases, and principal (LP token) release on term completion.

#### Accepted LP Tokens

- **Zera DEX LP Token**: `$dex-ZRA25sol-USDC+0000000000` (ZRA/USDC pool, 25 bps tier)
- **Solana LP Token**: Configurable via `update_sol_token()`, initially `$sol-8miyE+000000`

#### Term Options & Boosters

Longer lock-up periods receive a higher weight multiplier (booster), meaning a larger share of daily rewards:

| Term | Days | Booster | Effective Multiplier |
|------|------|---------|---------------------|
| 30 days | 30 | 100 | 1.00x |
| 90 days | 90 | 116 | 1.16x |
| 6 months | 182 | 140 | 1.40x |
| 1 year | 365 | 167 | 1.67x |
| 2 years | 730 | 201 | 2.01x |
| 3 years | 1,095 | 241 | 2.41x |
| 4 years | 1,460 | 289 | 2.89x |
| 5 years | 1,825 | 347 | 3.47x |
| 6 years | 2,190 | 417 | 4.17x |
| 7 years | 2,555 | 500 | 5.00x |

**Solana LP Bonus**: Solana LP token stakes receive an additional ~31.623x multiplier (`sol_multi / sol_multi_scale = 31623 / 1000`) on top of the term booster.

#### Weight Calculation

```
weight = (booster × principle) / 100

If staking Solana LP tokens:
    weight = (weight × 31623) / 1000
```

A staker's daily reward share is proportional to their weight relative to the total weight of all active stakers on that day:

```
daily_reward = (staker_weight / total_weight_for_day) × daily_release_for_phase
```

#### Reward Release Schedule (10 Phases)

ZRA rewards are released daily and decrease over 10 phases. Phase dates are anchored to the timestamp of the first stake:

| Phase | Start Offset | Approx. Date | Daily Release (per unit weight) |
|-------|-------------|---------------|-------------------------------|
| 1 | Launch | 2026-02-01 | 23,333,333,333,333 |
| 2 | +30 days | 2026-03-03 | 15,217,391,304,348 |
| 3 | +76 days | 2026-04-18 | 9,859,154,929,577 |
| 4 | +147 days | 2026-06-28 | 6,363,636,363,636 |
| 5 | +257 days | 2026-10-16 | 4,117,647,058,824 |
| 6 | +427 days | 2027-04-04 | 2,661,596,958,175 |
| 7 | +690 days | 2027-12-23 | 1,719,901,719,902 |
| 8 | +1,097 days | 2029-02-02 | 1,111,111,111,111 |
| 9 | +1,727 days | 2030-10-25 | 717,213,114,754 |
| 10 | +2,703 days | 2033-06-27 | 462,962,962,963 |
| End | +4,216 days | 2037-08-17 | — (no more releases) |

Each phase's daily release is the total ZRA distributed across all stakers, split by weight.

#### Key Functions

**Initialization**:
- `init()`: Empty (proxy initializes the bootstrapping manager)

**Staking**:
- `stake(amount: String, term: String, contract_id: String)`
  - Validates the LP token (`contract_id`) is an accepted type (Zera DEX LP or Solana LP)
  - Checks the user has sufficient balance
  - Calculates weight from term booster and principle (with Solana LP bonus if applicable)
  - Transfers LP tokens to the derived wallet
  - Records the stake with weight, term end, and LP token type
  - Initializes the bootstrapping manager on the first stake

**Reward Processing**:
- `process_rewards()`
  - Calculates days elapsed since last processing
  - Computes total weight per day across all stakers (only counting active stakes for each day)
  - For each staker's stake, calculates their proportional share of each day's release
  - Releases ZRA rewards via multi-send
  - Releases LP token principal from derived wallet when terms expire
  - Validates against exploit limits (sum of daily releases for elapsed days)

**Wallet Management**:
- `update_wallet(wallet_address: String, bump_id: String)`
  - Transfers a specific stake to a new wallet
  - Validates wallet address (base58 format, 32-byte public key)
  - Removes old wallet from staker index if no stakes remain

**Governance**:
- `update_sol_token(sol_token: String)`
  - Updates the accepted Solana LP token contract ID
  - Restricted to governance `update_key`

**One-Time Release**:
- `one_time_release()`
  - Special function to return Solana LP tokens to specific early staker wallets
  - Releases LP principal from derived wallet back to designated addresses
  - Can only be executed once (`ONE_TIME_RELEASE` flag)

#### State Management

**Wallet Stakes** (`WALLET_STAKES_{address}`):
```rust
pub struct AllWalletStakes {
    pub wallet_stakes: HashMap<String, WalletStake>,
}

pub struct WalletStake {
    pub principle: String,      // LP token amount staked (U256 as string)
    pub weight: String,         // Calculated weight (U256 as string)
    pub term: String,           // Lock-up term (e.g., "1_year")
    pub term_end: u64,          // Day number when term expires
    pub last_reward_day: u64,   // Last day rewards were calculated
    pub lp_token: String,       // LP token contract ID
}
```

**All Stakers Index** (`ALL_STAKERS_`):
```rust
pub struct AllStakers {
    pub staker_states: HashMap<String, u8>,  // wallet → marker
}
```

**Bootstrapping Manager** (`BOOT_MANAGER`):
```rust
pub struct BootstrappingManager {
    pub last_reward_day: u64,
    pub exploit: bool,
}
```

**Release Days** (`RELEASE_DAYS`):
```rust
pub struct ReleaseDays {
    date1: u64,   // Phase 1 start (first stake timestamp)
    date2: u64,   // Phase 2 start
    // ... through date10
    end_date: u64, // End of all rewards
}
```

**ID Bump** (`ID_BUMP_`):
```rust
pub struct IdBumpState {
    pub id: u64,  // Global incrementing stake ID
}
```

---

### 2. proxy (bootstrapping_proxy) - Proxy

**Location**: `bootstrapping/proxy/src/lib.rs`

**Purpose**: Upgradeable proxy for the bootstrapping implementation.

**Key Functions**:
- `init()`: Initializes governance keys and bootstrapping manager state
- `execute(function: String, parameters: String)`: Delegates to implementation
- `update(smart_contract: String, instance: String)`: Upgrades implementation
- `update_key(key: String)`: Changes the governance update key
- `update_send_all_key(send_all_key: String)`: Changes the fund control key
- `send_all()`: Transfers all funds to treasury

**Initial Configuration**:
- Implementation: `bootstrapping`, instance `0`
- Update key: Specific key holder address
- Send all key: `gov_$ZRA+0000`

## Reward Processing Flow

```
process_rewards() called daily
    ↓
Verify proxy authorization
    ↓
Calculate days elapsed since last processing
    ↓
Calculate total weight per day
├── For each day in range:
│   └── Sum weights of all active stakes (not expired, not already rewarded)
    ↓
Process all wallet stakes
├── For each staker:
│   ├── For each stake:
│   │   ├── For each elapsed day:
│   │   │   ├── Look up phase → daily release amount
│   │   │   ├── reward = (stake_weight / total_weight_for_day) × daily_release
│   │   │   └── Accumulate reward
│   │   ├── If term expired: mark principal for release
│   │   └── Otherwise: update last_reward_day
│   └── Add to release map
    ↓
Validate exploit limits
├── Sum theoretical max releases for elapsed days
└── If actual > max: set exploit flag, abort
    ↓
Send ZRA rewards via multi-send
    ↓
Release LP token principal from derived wallet (for expired terms)
    ↓
Update state
├── Save updated wallet stakes
├── Remove completed stakers
├── Update bootstrapping manager
└── Success!
```

## Integration Examples

### Staking LP Tokens
```rust
// User stakes Zera DEX LP tokens for 1 year
bootstrapping_proxy.execute(
    "stake",
    "1000000000000000000,1_year,$dex-ZRA25sol-USDC+0000000000"
);

// Weight = (167 × 1000000000000000000) / 100 = 1.67x the principal
// Staker earns proportional share of daily ZRA release
```

### Staking Solana LP Tokens
```rust
// User stakes Solana LP tokens for 2 years
bootstrapping_proxy.execute(
    "stake",
    "500000000000000000,2_years,$sol-8miyE+000000"
);

// Weight = ((201 × 500000000000000000) / 100) × 31623 / 1000
// Gets ~31.6x additional multiplier for Solana LP
```

### Processing Rewards
```rust
// Called daily by automated system
bootstrapping_proxy.execute("process_rewards", "");

// Distributes ZRA rewards to all stakers based on weight
// Releases LP token principal for expired terms
```

### Updating Wallet
```rust
// Transfer a specific stake to a new wallet
bootstrapping_proxy.execute(
    "update_wallet",
    "new_wallet_address,bump_id"
);
```

## Security Features

1. **Exploit Detection**: Daily release amounts validated against theoretical maximums per phase
2. **Proxy Authorization**: All operations must come through the bootstrapping proxy
3. **LP Token Validation**: Only accepted LP token contract IDs can be staked
4. **Weight Integrity**: Weights calculated deterministically from booster and principal
5. **Derived Wallet**: LP token principal held in a derived wallet owned by the proxy
6. **One-Time Release Guard**: `one_time_release()` can only execute once
7. **Wallet Validation**: Wallet updates validate base58 addresses (32-byte public keys)
8. **Governance Control**: LP token updates and contract upgrades restricted to governance keys

## Key Addresses

- Bootstrapping Proxy: `2nuEvMULK77BCZPyLLThtUn9kvkJkjsSyky7Nb67FMC1`
- Principal Derived Wallet: `derive_wallet("principle")` (computed at runtime, owned by the proxy)
- Treasury: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

## Notes

- All amounts use U256 precision (stored as strings in state)
- Rewards are paid in ZRA; principal is returned as the original LP token
- Phase dates are anchored to the contract's first activity timestamp, not a hardcoded calendar date
- The Solana LP multiplier (~31.623x) provides additional incentive for cross-chain liquidity
- State is stored on the proxy contract via `delegate_store_state` / `delegate_retrieve_state`
- Stake IDs are globally unique and incrementing
- Rewards cannot be processed more than once per day
- After the end date (~2037), no further ZRA rewards are distributed, but LP principal is still returned on term expiry
