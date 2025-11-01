# Staking System Contracts

## Overview

The Zera Network staking system is a comprehensive multi-contract architecture that manages token staking, reward distribution, early backer allocations, and liquid staking. It includes both time-locked staking with fixed rewards and flexible liquid staking options.

## Contract Architecture

```
┌─────────────────────────────────────────────────────┐
│              Staking System                         │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ┌──────────────┐        ┌──────────────┐           │
│  │   Staking    │◄───────┤   Staking    │           │
│  │   Proxy      │        │   V1         │           │
│  │ (normal_proxy)        │ (normal_stake)           │
│  └──────┬───────┘        └──────────────┘           │
│         │                                           │
│         │ calls                                     │
│         ▼                                           │
│  ┌──────────────┐        ┌──────────────┐           │
│  │  Principle   │◄───────┤  Principle   │           │
│  │  Proxy       │        │  V1          │           │
│  └──────────────┘        └──────────────┘           │
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

### 1. normal_stake (staking_v1) - Main Staking Implementation

**Location**: `staking/normal_stake/src/lib.rs`

**Purpose**: Manages time-locked staking with fixed APY rewards and liquid staking with flexible withdrawal.

#### Staking Options

**Fixed-Term Staking**:
- **6 Months**: 2% APY (1% total), daily release over 182 days
- **1 Year**: 6% APY, daily release over 365 days
- **2 Years**: 8% APY, daily release over 730 days
- **3 Years**: 7% APY, daily release over 1,095 days
- **4 Years**: 7% APY, daily release over 1,460 days
- **5 Years**: 7% APY, daily release over 1,825 days

**Liquid Staking**:
- **0.1% APY**: Daily rewards, 14-day withdrawal period after unstaking request
- No fixed end date
- Flexible principal amount

#### Key Functions

**Initialization**:
- `init()`: Sets up early backer allocations and reward tracking
- Initializes 25 early backer addresses with predetermined allocations
- Total early backer supply: 25M ZRA
- Total staking supply: 40M ZRA (25M used initially for early backers)

**Staking Operations**:
- `stake(amount: String, wallet_address: String, staking_type: String)`
  - Locks tokens in principle proxy
  - Creates stake record with reward calculations
  - Returns unique stake ID
  - Validates sufficient supply available

**Reward Processing**:
- `process_rewards()`
  - Calculates rewards for all stakers
  - Processes early backer releases
  - Handles completed stakes (releases principal)
  - Distributes rewards via multi-send
  - Includes exploit detection (max 50K ZRA per day)

**Liquid Staking**:
- `release_liquid_stake()`
  - Initiates 14-day withdrawal period
  - Marks unstake day for processing
  - Principal released after period + remaining rewards

**Wallet Management**:
- `update_wallet(wallet_address: String, bump_id: String)`
  - Transfers stake to new wallet
  - Validates uniqueness
  - Maintains stake history

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
}

pub struct LiquidStake {
    pub bump_id: u64,
    pub principle: u64,
    pub last_reward_day: u64,
    pub daily_release: u64,
    pub unstake_day: u64,  // u64::MAX if not unstaking
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
4. **Principal Protection**: Separate principle proxy wallet
5. **Daily Processing**: Rewards calculated per day, not per block
6. **Overflow Protection**: Uses saturating arithmetic

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

### 3. principle (staking_principle_v1) - Principal Management

**Location**: `staking/principle/src/lib.rs`

**Purpose**: Holds staked principal amounts and releases them back to staking proxy when stakes complete.

**Key Functions**:
- `init()`: Simple initialization
- `release_principle(amount: String)`: Transfers principal back to staking proxy

**Authorization**:
- Only calls from principle proxy (`8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko`)
- Delegates send to staking proxy (`AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8`)
- Validates against governance keys

**Purpose**: Separation of concerns - rewards come from staking contract, principal held separately for security

---

### 4. principle_proxy (staking_principle_proxy) - Principle Proxy

**Location**: `staking/principle_proxy/src/lib.rs`

**Purpose**: Upgradeable proxy for principle management.

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

### Fixed-Term Staking

**Reward Calculation**:
```rust
fn get_reward(staking_type: String, principle: u64) -> (u64, u64) {
    match staking_type {
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

```
process_rewards() called daily
    ↓
Calculate days elapsed since last processing
    ↓
Process Early Backers
├── Calculate daily_release × days_elapsed
├── Check against total_reward cap
└── Add to release map
    ↓
Process Normal Stakes
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
Request Principal Release (if needed)
├── Call principle_proxy.execute()
└── Parameters: "release_principle", amount
    ↓
Execute Multi-Send
├── Send rewards + principal to all wallets
└── If fails: refund principal, abort
    ↓
Update State
├── Save new wallet stakes
├── Save new early backer states
├── Update reward manager
├── Remove completed stakes
└── Success!
```

## Integration Examples

### Staking Tokens
```rust
// User calls through staking proxy
let stake_id = staking_proxy.execute(
    "stake",
    "1000000000000000000,wallet_address,1_year"
);

// Contract:
// 1. Validates amount and staking type
// 2. Checks available supply
// 3. Calculates rewards
// 4. Transfers tokens to principle proxy
// 5. Creates stake record
// 6. Returns stake ID
```

### Processing Rewards
```rust
// Called daily by automated system
staking_proxy.execute("process_rewards", "");

// Contract processes all stakes and early backers
// Sends rewards via multi-send
// Updates all state atomically
```

### Unstaking (Liquid)
```rust
// User initiates withdrawal
staking_proxy.execute("release_liquid_stake", "");

// Sets 14-day timer
// After 14 days, process_rewards() releases principal
```

## Security Considerations

1. **Exploit Detection**: Daily limits prevent reward manipulation
2. **Supply Cap**: Cannot exceed allocated 40M ZRA
3. **Atomic Processing**: All-or-nothing reward distribution
4. **Principal Separation**: Staked funds held in separate contract
5. **Governance Control**: Proxies upgradeable only by governance
6. **Authorization Checks**: Multi-layer access control
7. **Overflow Protection**: Saturating arithmetic throughout

## Upgrade Process

1. **Test New Implementation**: Deploy and test new version
2. **Governance Proposal**: Submit upgrade proposal
3. **Community Vote**: Approve upgrade
4. **Execute Upgrade**: Call `update()` on respective proxy
5. **State Migration**: If needed, migrate data to new format
6. **Verify**: Confirm all stakes and rewards intact

## Initial Configuration

**Staking Proxy**:
- Implementation: staking_v1, instance 1
- Governance: `gov_$ZRA+0000`
- Initialization: 100 USD equivalent

**Principle Proxy**:
- Implementation: staking_principle_v1, instance 1
- Governance: `gov_$ZRA+0000`
- Authorization: Only staking proxy
- Initialization: 1 USD equivalent

**Early Proxy**:
- Implementation: release_v1, instance 1
- Governance: `gov_$ZRA+0000`
- Initialization: 100 USD equivalent

## Key Addresses

- Staking Proxy: `AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8`
- Principle Proxy: `8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko`
- Early Backer Proxy: `AZfFcttA3nwqmEYzAtsmufops7PaxLYavvkkDRsxTX5j`
- Treasury: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

## Notes

- All amounts in smallest denomination (18 decimals)
- Rewards processed daily based on block timestamps
- Principal held separately from reward pool
- Early backers have priority in processing
- Liquid staking rewards capped by available supply
- Failed multi-sends refund principal automatically
- Stake IDs are globally unique and incrementing

