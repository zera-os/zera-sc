# Staking System Contracts

## Overview

The Zera Network staking system is a comprehensive multi-contract architecture that manages token staking, reward distribution, early backer allocations, liquid staking, and instant staking. It includes time-locked staking with fixed rewards, flexible liquid staking, and instant staking where users receive 2/3 of their rewards upfront. The system has been upgraded from v1 to v2, migrating principal storage from a separate principle proxy contract to a program-derived wallet and adding instant staking support.

## Contract Architecture

```
┌─────────────────────────────────────────────────────┐
│              Staking System (v2)                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐        ┌──────────────┐           │
│  │   Staking    │◄───────┤   Staking    │           │
│  │   Proxy      │        │   V2         │           │
│  │ (normal_proxy)        │ (normal_stake)           │
│  └──────┬───────┘        └──────────────┘           │
│         │                        │                  │
│         │ delegates              │ derived_send     │
│         │                        ▼                  │
│         │                ┌──────────────┐           │
│         │                │  Derived     │           │
│         │                │  Wallet      │           │
│         │                │ ("principle")│           │
│         │                └──────────────┘           │
│         │                 (holds all principal)     │
│         │                                           │
│  ┌──────────────┐        ┌──────────────┐           │
│  │  Principle   │        │  Principle   │           │
│  │  Proxy       │◄───────┤  V1          │           │
│  │  (legacy)    │        │  (legacy)    │           │
│  └──────────────┘        └──────────────┘           │
│   (funds migrated to derived wallet in v2)          │
│                                                     │
│  ┌──────────────┐        ┌──────────────┐           │
│  │ Early Backer │◄───────┤   Release    │           │
│  │   Proxy      │        │   V1         │           │
│  │(early_proxy) │        │(early_backers)│          │
│  └──────────────┘        └──────────────┘           │
│                                                     │
└─────────────────────────────────────────────────────┘
```

## Contracts

### 1. normal_stake (staking_v2) - Main Staking Implementation

**Location**: `staking/normal_stake/src/lib.rs`

**Purpose**: Manages time-locked staking with fixed APY rewards, liquid staking with flexible withdrawal, and instant staking with upfront rewards. Upgraded from v1 to v2 with derived wallet principal storage and instant staking support.

#### Staking Options

**Fixed-Term Staking (Daily Rewards)**:
- **6 Months**: 2% APY (1% total), daily release over 182 days
- **1 Year**: 6% APY, daily release over 365 days
- **2 Years**: 8% APY, daily release over 730 days
- **3 Years**: 7% APY, daily release over 1,095 days
- **4 Years**: 7% APY, daily release over 1,460 days
- **5 Years**: 7% APY, daily release over 1,825 days

**Instant Staking (Upfront Rewards)**:
- Same term options as fixed-term staking (6 months through 5 years) — **liquid not eligible**
- Rewards paid **immediately** at the time of staking
- Receives **2/3 (66.67%)** of the normal total reward
- Principal locked until the term expires (same durations as regular staking)
- **No daily reward processing** — rewards are fully distributed upfront
- Principal released in batch via `release_instant()` after term ends

**Liquid Staking**:
- **0.1% APY**: Daily rewards, 14-day withdrawal period after unstaking request
- No fixed end date
- Flexible principal amount

#### Key Functions

**Initialization**:
- `init()`: Empty in v2 (legacy v1 initialization moved to `init_v2`)
- `init_v2()`: Migration function from v1 to v2
  - Moves all principal from the old `PRINCIPLE_WALLET` to a program-derived wallet (`derive_wallet("principle")`)
  - Calls principle proxy to release all funds from the legacy wallet
  - Copies all state from staking_v1 instance 1 to the proxy contract via `delegate_store_state`
  - Migrates old `OldWalletStake` (without `term` field) to new `WalletStake` (with `term` field, defaulting to `"5_years"`)
  - Sets `MIGRATED` flag to prevent re-execution
  - All subsequent operations require migration to have completed

**Staking Operations**:
- `stake(amount: String, wallet_address: String, term: String)`
  - Transfers principal to derived wallet
  - Creates stake record with reward calculations
  - Returns unique stake ID
  - Validates sufficient supply available
  - For fixed-term: allocates total reward from supply
  - For liquid: no supply allocation (rewards from pool)

**Instant Staking Operations**:
- `instant_stake(amount: String, term: String)`
  - Works like regular stake but rewards are sent immediately
  - Calculates 2/3 of the normal total reward: `reward = (total_reward × 66,666,666,666) / 100,000,000,000`
  - Transfers principal to derived wallet
  - Sends computed reward directly to the staker's wallet
  - Records instant stake with `release_day` for future principal release
  - Deducts reward from available supply
  - Liquid staking not allowed for instant stakes
- `release_instant()`
  - Batch releases principal for all expired instant stakes
  - Checks `earliest_release_day` optimization to skip processing if no stakes ready
  - Iterates all instant stakers, releasing principal for stakes past their `release_day`
  - Sends principal back from derived wallet via `derived_send_multi`
  - Removes completed stakes, clears state for fully released wallets

**Reward Processing**:
- `process_rewards()`
  - Calculates rewards for all regular (non-instant) stakers
  - Processes early backer releases
  - Handles completed stakes (releases principal from derived wallet via `derived_send`)
  - Distributes rewards via multi-send
  - Includes exploit detection (max 50K ZRA per day)
  - **Note**: Instant stakes are not processed here — they have no daily rewards

**Liquid Staking**:
- `release_liquid_stake()`
  - Initiates 14-day withdrawal period
  - Marks unstake day for processing
  - Principal released after period + remaining rewards

**Wallet Management**:
- `update_wallet(wallet_address: String, bump_id: String)`
  - Transfers a regular or liquid stake to a new wallet
  - Validates uniqueness
  - Handles early backer, fixed-term, and liquid stake transfers
  - Maintains stake history
- `update_instant_wallet(wallet_address: String, bump_id: String)`
  - Transfers an instant stake to a new wallet
  - Validates wallet address (base58 format, 32-byte public key)
  - Moves the specific stake (by bump_id) from old wallet to new wallet
  - Updates `AllInstantStakers` tracker
  - Clears old wallet state if no stakes remain

#### V1 → V2 Migration

The `init_v2()` migration performs the following one-time operations:

1. **Verify** proxy authorization and that migration hasn't already run
2. **Release principal** from legacy principle proxy wallet
3. **Transfer** all ZRA from the old `PRINCIPLE_WALLET` to the new derived wallet
4. **Copy all state** from staking_v1 to the proxy contract's state space
5. **Migrate stake structs** from `OldWalletStake` (no term) to `WalletStake` (with term defaulting to `"5_years"`)
6. **Set MIGRATED flag** to prevent re-execution

After migration, `check_auth()` verifies both proxy authorization **and** that the `MIGRATED` flag is set.

#### State Management

**Early Backer State** (`EARLY_STAKER_STATE_`):
```rust
pub struct EarlyStakerState {
    pub bump_id: u64,
    pub staker_address: String,
    pub total_reward: u64,
    pub daily_release: u64,
    pub total_released: u64,
}
```

**Wallet Stakes** (`WALLET_STAKES_{address}`):
```rust
pub struct AllWalletStakes {
    pub staker_states: HashMap<String, WalletStake>,
    pub liquid_stake: LiquidStake,
}

pub struct WalletStake {
    pub principle: u64,
    pub total_reward: u64,
    pub daily_release: u64,
    pub total_released: u64,
    pub last_reward_day: u64,
    pub term: String,           // NEW in v2: "6_months", "1_year", etc.
}

pub struct LiquidStake {
    pub bump_id: u64,
    pub principle: u64,
    pub last_reward_day: u64,
    pub daily_release: u64,
    pub unstake_day: u64,  // u64::MAX if not unstaking
}
```

**Instant Staking State** (`INSTANT_STAKES_{address}`):
```rust
pub struct AllWalletInstantStakes {
    pub staker_states: HashMap<String, InstantStake>,
}

pub struct InstantStake {
    pub principle: u64,
    pub total_reward: u64,
    pub release_day: u64,
    pub term: String,
}
```

**All Instant Stakers Index** (`ALL_INSTANT_STAKERS`):
```rust
pub struct AllInstantStakers {
    pub staker_states: HashMap<String, u8>,  // wallet → marker
    pub earliest_release_day: u64,           // optimization: skip processing if too early
}
```

**Reward Manager** (`REWARD_MANAGER_STATE_`):
```rust
pub struct RewardManagerState {
    pub total_supply: u64,      // 40M ZRA
    pub used_supply: u64,        // Currently allocated
    pub last_reward_day: u64,    // Last processing day
    pub exploit: bool,           // Exploit detection flag
}
```

**All Stakers Index** (`ALL_STAKERS_`):
```rust
pub struct AllStakers {
    pub staker_states: HashMap<String, u8>,  // wallet → marker
}
```

#### Security Features

1. **Exploit Detection**: Max 50K ZRA rewards per day
2. **Supply Tracking**: Cannot exceed 40M total allocation
3. **Atomic Operations**: Multi-send with rollback on failure
4. **Principal Protection**: Derived wallet owned by the staking proxy (migrated from separate principle proxy)
5. **Daily Processing**: Rewards calculated per day, not per block
6. **Overflow Protection**: Uses saturating arithmetic
7. **Migration Guard**: All operations require `MIGRATED` flag to be set
8. **Wallet Validation**: Instant stake wallet updates validate base58 addresses (32-byte public keys)

---

### 2. normal_proxy (staking_proxy) - Staking Proxy

**Location**: `staking/normal_proxy/src/lib.rs`

**Purpose**: Upgradeable proxy for main staking contract.

**Key Functions**:
- `init()`: Initializes with staking_v1
- `execute(function: String, parameters: String)`: Delegates to implementation
- `update(smart_contract: String, instance: String)`: Upgrades implementation
- `update_update_key()`: Changes governance key
- `update_send_all_key()`: Changes fund control key
- `send_all()`: Transfers all funds to treasury

**Initialization Fee**: 100 USD equivalent (converted to ZRA and held on initialization)

---

### 3. principle (staking_principle_v1) - Principal Management (Legacy)

**Location**: `staking/principle/src/lib.rs`

**Purpose**: Previously held staked principal amounts. In v2, all principal has been migrated to a program-derived wallet. This contract is now legacy — used only during the v1 → v2 migration to release remaining funds.

**Key Functions**:
- `init()`: Simple initialization
- `release_principle(amount: String)`: Transfers principal back to staking proxy

**Authorization**:
- Only calls from principle proxy (`8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko`)
- Delegates send to staking proxy (`AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8`)
- Validates against governance keys

**v2 Note**: During `init_v2()`, all funds are moved from this wallet to `derive_wallet("principle")`. The derived wallet is now the sole holder of staked principal, owned by the staking proxy.

---

### 4. principle_proxy (staking_principle_proxy) - Principle Proxy (Legacy)

**Location**: `staking/principle_proxy/src/lib.rs`

**Purpose**: Upgradeable proxy for principle management. Legacy in v2 — principal now held in a derived wallet.

**Key Functions**:
- `init()`: Initializes with staking_principle_v1
- `execute()`: Delegates to implementation (restricted to staking proxy)
- `update()`: Upgrades implementation
- Governance key management

**Authorization Check**:
```rust
let sc_wallet = smart_contracts::smart_contract_wallet();
if sc_wallet != STAKING_PROXY_WALLET {
    return false;  // Only staking proxy can call
}
```

**Initialization Fee**: 1 USD equivalent (converted to ZRA and held on initialization)

---

### 5. early_backers (release_v1) - Early Backer Releases

**Location**: `staking/early_backers/src/lib.rs`

**Purpose**: Manages token release schedule for 25 early backers over time.

**Key Functions**:
- `init()`: Initializes 25 early backer allocations with daily release rates
- `process_rewards()`: Releases daily allocations to early backers
- `update_wallet(wallet_address: String)`: Allows early backers to change their wallet

#### Early Backer Allocations

25 early backers with varying allocations:
- 1x 563 ZRA (test allocation)
- 1x 999,999.437 ZRA
- 11x 500,000 ZRA each
- 1x 1,000,000 ZRA each (×6 backers)
- 2x 2,000,000 ZRA each (×9 backers)
- 2x 5,000,000 ZRA each
- 1x 10,000,000 ZRA

**Total**: ~50M ZRA for early backers

**Release Schedule**: Daily over 3,650 days (~10 years)

**Exploit Protection**: Max 14K ZRA per day

**State Structure**:
```rust
pub struct StakerState {
    pub staker_address: String,
    pub principle: u64,       // Total allocation
    pub daily_release: u64,   // Amount per day
    pub total_released: u64,  // Released so far
}
```

---

### 6. early_proxy (release_proxy) - Early Backer Proxy

**Location**: `staking/early_proxy/src/lib.rs`

**Purpose**: Upgradeable proxy for early backer releases.

**Key Functions**:
- `init()`: Initializes with release_v1
- `execute()`: Delegates to implementation (public access for processing)
- `update()`: Upgrades implementation
- Governance key management

**Initialization Fee**: 100 USD equivalent (converted to ZRA and held on initialization)

## Staking Economics

### Fixed-Term Staking (Daily Rewards)

**Reward Calculation**:
```rust
fn get_reward(term: String, principle: u64) -> (u64, u64) {
    match term {
        "1_year" => {
            let total_reward = (principle * 6) / 100;  // 6% APY
            let daily_release = total_reward / 365;
            (total_reward, daily_release)
        }
        // ... other periods
    }
}
```

**Total Allocation**: 40M ZRA
- 25M initially allocated to early backers
- 15M available for public staking
- Dynamic supply tracking

### Instant Staking (Upfront Rewards)

**Reward Calculation**:
```
instant_reward = (normal_total_reward × 66,666,666,666) / 100,000,000,000
```

This gives exactly **2/3 (66.67%)** of the normal reward. For example:
- **1 Year, 1,000 ZRA principal**: Normal = 60 ZRA → Instant = 40 ZRA (paid immediately)
- **5 Years, 1,000 ZRA principal**: Normal = 350 ZRA → Instant = 233.33 ZRA (paid immediately)

**Characteristics**:
- Same term options as fixed-term (6 months through 5 years)
- Liquid staking **not eligible** for instant staking
- Rewards paid in full at the time of staking
- Principal locked until term expires
- No daily reward processing needed
- Supply deducted at stake time

### Liquid Staking

**Characteristics**:
- 0.1% APY (365 days basis)
- No fixed end date
- 14-day withdrawal period after unstaking request
- Rewards paid daily
- Principal separate from rewards

**Withdrawal Process**:
1. User calls `release_liquid_stake()`
2. Contract sets `unstake_day = current_day + 14`
3. During processing, if `current_day >= unstake_day`:
   - Final rewards calculated
   - Principal released
   - Stake removed

## Reward Processing Flow

### Daily Rewards (process_rewards)

```
process_rewards() called daily
    ↓
Verify authorization + migration
    ↓
Calculate days elapsed since last processing
    ↓
Process Early Backers
├── Calculate daily_release × days_elapsed
├── Check against total_reward cap
└── Add to release map
    ↓
Process Normal Stakes (fixed-term + liquid only, NOT instant)
├── For each wallet with stakes
├── For each stake in wallet
│   ├── Calculate rewards
│   ├── Check if stake completed
│   ├── If completed: release principal
│   └── Otherwise: update total_released
└── Add to release map
    ↓
Process Liquid Stakes
├── Check against available supply
├── Calculate rewards
├── Check if unstake_day reached
└── If yes: mark for principal release
    ↓
Validate Exploit Limits
├── Early: max 14K ZRA/day
├── Normal: max 50K ZRA/day
└── If exceeded: set exploit flag, abort
    ↓
Release Principal (if needed)
├── derived_send from derived wallet to proxy
└── Amount = total principal being released
    ↓
Execute Multi-Send
├── Send rewards + principal to all wallets
└── If fails: panic (rollback)
    ↓
Update State
├── Save new wallet stakes
├── Save new early backer states
├── Update reward manager
├── Remove completed stakes
└── Success!
```

### Instant Staking Flow

```
instant_stake(amount, term) called by user
    ↓
Verify authorization + migration + no exploit
    ↓
Calculate total_reward for the term
    ↓
Apply 2/3 multiplier: reward = total_reward × 66.67%
    ↓
Check supply: used_supply + reward ≤ total_supply
    ↓
Transfer principal to derived wallet
    ↓
Send reward immediately to staker's wallet
    ↓
Record InstantStake { principle, total_reward, release_day, term }
    ↓
Update AllInstantStakers (track earliest_release_day)
    ↓
Save state + emit events
```

### Instant Stake Release Flow

```
release_instant() called
    ↓
Verify authorization + no exploit
    ↓
Check: current_day ≥ earliest_release_day (skip if not)
    ↓
For each instant staker:
├── For each stake in wallet:
│   ├── If release_day ≤ current_day:
│   │   └── Add to release list (principal)
│   └── Else: keep in new state
├── If wallet has no remaining stakes:
│   └── Clear wallet's instant stake state
└── Track new earliest_release_day
    ↓
derived_send_multi: send all principal back from derived wallet
    ↓
Update AllInstantStakers state
    ↓
Emit events with total released
```

## Integration Examples

### Staking Tokens (Regular)
```rust
// User calls through staking proxy
let stake_id = staking_proxy.execute(
    "stake",
    "1000000000000000000,wallet_address,1_year"
);

// Contract:
// 1. Validates amount and term
// 2. Checks available supply
// 3. Calculates rewards (6% for 1 year)
// 4. Transfers principal to derived wallet
// 5. Creates stake record with term
// 6. Returns stake ID
```

### Instant Staking
```rust
// User calls through staking proxy
staking_proxy.execute(
    "instant_stake",
    "1000000000000000000,1_year"
);

// Contract:
// 1. Calculates normal reward (6% of principal)
// 2. Applies 2/3 multiplier → 4% effective reward
// 3. Sends reward immediately to user's wallet
// 4. Locks principal in derived wallet until term ends
// 5. Records instant stake with release_day
```

### Processing Rewards
```rust
// Called daily by automated system
staking_proxy.execute("process_rewards", "");

// Contract processes regular stakes and early backers
// Does NOT process instant stakes (they have no daily rewards)
// Sends rewards via multi-send
// Updates all state atomically
```

### Releasing Instant Stakes
```rust
// Called periodically to release matured instant stakes
staking_proxy.execute("release_instant", "");

// Checks if any instant stakes have reached their release_day
// Batch releases all matured principal back to stakers
// Uses derived_send_multi from derived wallet
```

### Unstaking (Liquid)
```rust
// User initiates withdrawal
staking_proxy.execute("release_liquid_stake", "");

// Sets 14-day timer
// After 14 days, process_rewards() releases principal
```

### Updating Instant Stake Wallet
```rust
// User transfers an instant stake to a new wallet
staking_proxy.execute(
    "update_instant_wallet",
    "new_wallet_address,bump_id"
);

// Validates new address (base58 format)
// Moves stake from old wallet to new wallet
// Updates AllInstantStakers tracker
```

## Security Considerations

1. **Exploit Detection**: Daily limits prevent reward manipulation (50K ZRA/day for normal, 14K ZRA/day for early backers)
2. **Supply Cap**: Cannot exceed allocated 40M ZRA
3. **Atomic Processing**: All-or-nothing reward distribution
4. **Derived Wallet Principal**: Staked funds held in a program-derived wallet owned by the staking proxy (migrated from separate principle proxy in v2)
5. **Governance Control**: Proxies upgradeable only by governance
6. **Authorization Checks**: Multi-layer access control (proxy + migration guard)
7. **Overflow Protection**: Saturating arithmetic throughout
8. **Migration Guard**: All v2 operations require `MIGRATED` flag
9. **Wallet Validation**: Instant stake wallet updates validate base58 addresses
10. **Instant Stake Supply Check**: Upfront rewards deducted from supply at stake time, preventing over-allocation

## Upgrade Process

1. **Test New Implementation**: Deploy and test new version
2. **Governance Proposal**: Submit upgrade proposal
3. **Community Vote**: Approve upgrade
4. **Execute Upgrade**: Call `update()` on respective proxy
5. **State Migration**: If needed, migrate data to new format (e.g., `init_v2()` for v1 → v2)
6. **Verify**: Confirm all stakes and rewards intact

## Initial Configuration

**Staking Proxy**:
- Implementation: staking_v2 (upgraded from staking_v1 via `init_v2()`)
- Governance: `gov_$ZRA+0000`
- Initialization: 100 USD equivalent
- State stored on proxy via `delegate_store_state`

**Principle Proxy** (legacy):
- Implementation: staking_principle_v1, instance 1
- Governance: `gov_$ZRA+0000`
- Authorization: Only staking proxy
- Initialization: 1 USD equivalent
- Funds migrated to derived wallet in v2

**Early Proxy**:
- Implementation: release_v1, instance 1
- Governance: `gov_$ZRA+0000`
- Initialization: 100 USD equivalent

## Key Addresses

- Staking Proxy: `AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8`
- Principle Proxy (legacy): `8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko`
- Principal Derived Wallet: `derive_wallet("principle")` (computed at runtime, owned by the staking proxy)
- Early Backer Proxy: `AZfFcttA3nwqmEYzAtsmufops7PaxLYavvkkDRsxTX5j`
- Treasury: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

## Notes

- All amounts in smallest denomination (18 decimals)
- Rewards processed daily based on block timestamps
- Principal held in derived wallet (migrated from separate principle proxy in v2)
- Early backers have priority in processing
- Liquid staking rewards capped by available supply
- Failed multi-sends cause panic (rollback)
- Stake IDs are globally unique and incrementing
- Instant stakes receive 2/3 of normal rewards upfront, with principal locked until term ends
- Instant stake principal is released in batches via `release_instant()`, not via `process_rewards()`
- State is stored on the proxy contract via `delegate_store_state` / `delegate_retrieve_state`

