# DEX (Decentralized Exchange) Contracts

## Overview

The Zera DEX is an on-chain automated market maker (AMM) built on the constant product formula (`x * y = k`). It enables permissionless token swaps, liquidity pool creation, and liquidity provision with configurable fee tiers. The DEX also feeds price data back into the ACE (Automated Currency Exchange) system for tokens paired with ZRA.

## Contracts

### 1. dex_factory (zera_dex) - DEX Implementation

**Location**: `dex/dex_factory/src/lib.rs`

**Purpose**: Core DEX logic handling pool creation, liquidity management, token swaps, and price calculations.

**Key Functions**:

#### Initialization
- `init()`: Derives the lock wallet used for time-locked LP tokens

#### Pool Creation

**`create_liquidity_pool()`**: Creates a new liquidity pool for a token pair
- **Parameters**:
  - `token1`, `token2`: Token contract IDs for the pair
  - `token1_volume`, `token2_volume`: Initial liquidity amounts
  - `fee_bps`: Fee tier in basis points
  - `lock_timestamp`: Optional LP token lock-up time (Unix timestamp)
- **Behavior**:
  - Validates tokens are different
  - Auto-orders pairs so ZRA is always token1 (if present)
  - Validates volumes > 0 and valid U256
  - Checks user has sufficient balances
  - Prevents duplicate pools (checks both token orderings)
  - Transfers tokens to a derived wallet for the pool
  - Calculates initial LP tokens: `sqrt(token1_scaled * token2_scaled)`
  - Creates LP token contract via `instrument_contract_dex()`
  - Optionally locks LP tokens until `lock_timestamp`
  - Updates ACE price data for ZRA-paired pools (25 bps tier)
- **Events**: `LIQUIDITY_CREATED` with pool details

#### Liquidity Management

**`add_liquidity()`**: Add liquidity to an existing pool
- **Parameters**: Same as `create_liquidity_pool()`
- **Behavior**:
  - Loads existing pool (tries both token orderings)
  - Validates volumes and balances
  - Calculates LP tokens to mint proportionally:
    - Uses `min(lp_from_token1, lp_from_token2)` to maintain ratio
    - Adjusts the non-limiting token to match the ratio
  - Transfers tokens to pool's derived wallet
  - Mints LP tokens to user (or lock wallet if locked)
  - Updates pool reserves and circulating LP supply
- **Events**: `LIQUIDITY_ADDED` with updated reserves

**`remove_liquidity()`**: Remove liquidity by burning LP tokens
- **Parameters**:
  - `token1`, `token2`: Token pair
  - `lp_tokens`: Amount of LP tokens to redeem
  - `fee_bps`: Fee tier
- **Behavior**:
  - Validates LP token balance
  - Calculates proportional token amounts:
    - `token1_out = (lp_amount * reserve_token1) / total_lp_supply`
    - `token2_out = (lp_amount * reserve_token2) / total_lp_supply`
  - Burns LP tokens (sends to `DeXBurnDexBurnDexBurnDexBurnDexBurnDex`)
  - Sends tokens back to user from derived wallet
  - Updates pool reserves
  - If all LP tokens redeemed: marks pool as inactive and clears ACE data
- **Events**: `LIQUIDITY_REMOVED` with amounts returned

**`unlock_liquidity_pool_tokens()`**: Unlock time-locked LP tokens
- **Parameters**: `token1`, `token2`, `fee_bps`
- **Behavior**:
  - Validates pool exists
  - Checks lock timestamp has passed
  - Transfers LP tokens from lock wallet to user
  - Clears lock state
- **Events**: `LIQUIDITY_UNLOCKED` with details

#### Token Swaps

**`swap()`**: Swap one token for another
- **Parameters**:
  - `token1`: Input token
  - `token2`: Output token
  - `token1_volume`: Amount of input token
  - `fee_bps`: Fee tier of the pool
  - `platform_bps`: Optional platform fee (max 500 bps / 5%)
  - `platform_wallet`: Optional wallet to receive platform fee
- **Behavior**:
  - Validates input amount, fee tier, and platform fee
  - Loads pool (tries both token orderings)
  - Checks user balance
  - Calculates swap using constant product formula with fees
  - Splits input: treasury fee sent to treasury, remainder to pool
  - Sends output tokens to user (minus optional platform fee)
  - Updates pool reserves
  - Updates ACE price for ZRA-paired pools (25 bps tier)
- **Events**: `SWAP_EXECUTED` with amounts, fees, and updated reserves

#### Governance

**`update_fees()`**: Update treasury fee percentage and wallet
- Requires proxy wallet authorization AND governance key (`gov_$ACE+0000`)
- Treasury fee max: 1000 (100% of LP fee, effectively 10% max swap fee at highest tier)
- Validates wallet address format

### Swap Formula

The DEX uses the **constant product AMM formula** with a split fee model:

```
Fee Calculation:
  lp_fee_calculated = fee_bps * 100  (convert to parts per million)
  treasury_fee_calculated = (lp_fee_calculated * treasury_fee_bps) / 1000
  reward_fee_calculated = lp_fee_calculated - treasury_fee_calculated

Fee Application:
  treasury_fee = (amount_in * treasury_fee_calculated) / 1,000,000
  amount_to_pool = amount_in - treasury_fee

Constant Product Swap:
  amount_out = (reserve_out * amount_to_pool * (1,000,000 - reward_fee_calculated))
               / ((reserve_in + amount_to_pool) * 1,000,000)
```

**Fee Breakdown**:
- **Treasury Fee**: Portion of LP fee sent to treasury wallet (default: 12.5% of LP fee)
- **Reward Fee**: Remaining portion stays in pool as LP rewards
- **Platform Fee**: Optional, taken from output (max 5%), sent to platform wallet

### Fee Tiers

Pools can be created with one of seven fee tiers (in basis points):

| Fee Tier | Percentage | Typical Use Case                |
|----------|-----------|----------------------------------|
| 10 bps   | 0.10%     | Stablecoin pairs                 |
| 25 bps   | 0.25%     | Standard pairs (price feed tier) |
| 50 bps   | 0.50%     | Standard pairs                   |
| 100 bps  | 1.00%     | Low-volume pairs                 |
| 200 bps  | 2.00%     | Volatile pairs                   |
| 400 bps  | 4.00%     | High-risk pairs                  |
| 800 bps  | 8.00%     | Exotic pairs                     |

**Note**: The **25 bps** tier for ZRA-paired pools is special -- it automatically updates the ACE price oracle.

### LP Token Calculation

**Initial Liquidity**:
```
lp_tokens = sqrt(token1_scaled * token2_scaled)
```

**Adding Liquidity** (proportional):
```
lp_from_token1 = (amount_token1 * total_lp_supply) / reserve_token1
lp_from_token2 = (amount_token2 * total_lp_supply) / reserve_token2
lp_tokens_minted = min(lp_from_token1, lp_from_token2)
```

**Removing Liquidity**:
```
token1_out = (lp_amount * reserve_token1) / total_lp_supply
token2_out = (lp_amount * reserve_token2) / total_lp_supply
```

### Scaling

All calculations use a normalized scale to handle tokens with different denominations:

```
SCALE = 1,000,000,000 (10^9)
token_scaled = (token_amount * SCALE) / token_denomination
token_raw = (token_scaled * token_denomination) / SCALE
```

This ensures consistent math across tokens with varying decimal places.

### ACE Price Feed Integration

When a swap or pool creation occurs on a **ZRA-paired pool at 25 bps**, the DEX calculates and stores:

1. **Token → USD rate**: Based on ZRA reserves and token reserves
2. **ZRA → Token rate**: Inverse calculation

```rust
ace_value = (zra_volume * token_denom * ONE_DOLLAR) / (token_volume * 10^9)
```

These values are stored under `ACE_{contract_id}` keys on the proxy contract, enabling other contracts and the network to query token prices.

**State Storage**:

```rust
LiquidityPool {
    token1: String,                    // First token contract ID
    token2: String,                    // Second token contract ID
    lp_token_id: String,               // LP token contract ID
    token1_volume: String,             // Current token1 reserves
    token2_volume: String,             // Current token2 reserves
    circulating_lp_tokens: String,     // Active LP token supply
    active: bool,                      // Pool active status
    derived_wallet: String,            // Pool's derived wallet holding funds
    redeemed_lp_tokens: String,        // Total LP tokens ever redeemed
    fee_bps: u64,                      // Pool fee tier
}

Fees {
    treasury_fee: u64,         // Treasury fee share (bps of LP fee)
    treasury_wallet: String,   // Treasury wallet address
}

LockedLiquidityPool {
    token1: String,            // First token
    token2: String,            // Second token
    lp_tokens: String,         // Amount of locked LP tokens
    lock_timestamp: u64,       // Unix timestamp when unlock is available
}

SymbolConfig {
    suffix_count: u64,         // Counter for LP token suffixes
}
```

**Authorization**:
- All functions require calls from the proxy wallet (`3uct7y6rcxW3KA8o8b2gqtaygw7hA39P3SyjV466fXWP`)
- Fee updates additionally require governance key (`gov_$ACE+0000`)
- State is stored via `delegate_store_state` on the proxy contract

---

### 2. proxy (zera_dex_proxy) - DEX Proxy

**Location**: `dex/proxy/src/lib.rs`

**Wallet Address**: `3uct7y6rcxW3KA8o8b2gqtaygw7hA39P3SyjV466fXWP`

**Purpose**: Upgradeable proxy for the DEX with governance controls and fee configuration.

**Key Functions**:

- `init()`: Initializes proxy with default configuration
  - Implementation: `zera_dex`, instance 1
  - Governance: `gov_$ACE+0000`
  - Treasury fee: 125 (12.5% of LP fee)
  - Treasury wallet: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

- `execute(function: String, parameters: String)`: Delegates calls to DEX implementation
  - Public entry point for all DEX operations
  - Parameters are comma-separated

- `update(smart_contract: String, instance: String)`: Upgrades DEX implementation
  - Restricted to `update_key`

- `update_key(key: String)`: Changes governance key
  - Restricted to current `update_key`

- `update_send_all_key(send_all_key: String)`: Changes fund withdrawal key
  - Restricted to current `send_all_key`

- `send_all(wallet: String)`: Transfers all proxy funds to specified wallet
  - Restricted to `send_all_key`

**State Storage**:
```rust
SmartContractState {
    smart_contract: String,  // "zera_dex"
    instance: String,        // "1"
}

GovKeys {
    update_key: String,      // "gov_$ACE+0000"
    send_all_key: String,    // "gov_$ACE+0000"
}

Fees {
    treasury_fee: u64,       // 125 (12.5%)
    treasury_wallet: String, // Treasury address
}
```

**Note**: No initialization fee for the proxy.

---

## Architecture

```
User → zera_dex_proxy (execute) → zera_dex (implementation)
                                       │
                         ┌─────────────┼─────────────┐
                         │             │             │
                    Derived Wallets   LP Tokens    ACE Prices
                    (hold pool funds) (minted/burned) (price feed)
```

### Derived Wallets

Each pool has a unique **derived wallet** that holds the pool's funds:

```rust
derived = smart_contracts::derive_wallet(
    format!("{}{}{}", token1, token2, fee_bps)
);
```

This ensures:
- Pool funds are isolated from each other
- Only the DEX contract can move funds via `derived_send()`
- Each token pair + fee tier combination has unique storage

### LP Token Locking

LP tokens can be optionally **time-locked** on creation or when adding liquidity:

1. If `lock_timestamp` is in the future:
   - LP tokens are sent to a global lock-derived wallet
   - A `LockedLiquidityPool` record is saved per user + pool
   - User cannot access LP tokens until timestamp passes
2. When unlocking:
   - Validates current time >= lock timestamp
   - Transfers LP tokens from lock wallet to user
   - Clears lock record

---

## Use Cases

### 1. Creating a Pool

```rust
// Create a ZRA/USDC pool at 0.25% fee tier with 30-day lock
execute("create_liquidity_pool", 
    "$ZRA+0000,$USDC+0000,1000000000000,500000000,25,1707868800")
// token1=ZRA, token2=USDC, 1000 ZRA, 500 USDC, 25bps, lock until Feb 14
```

### 2. Swapping Tokens

```rust
// Swap 100 ZRA for USDC on the 25bps pool, no platform fee
execute("swap", "$ZRA+0000,$USDC+0000,100000000000,25,0,")

// Swap with 1% platform fee going to aggregator
execute("swap", "$ZRA+0000,$USDC+0000,100000000000,25,100,aggregator_wallet")
```

### 3. Removing Liquidity

```rust
// Redeem 500 LP tokens from ZRA/USDC 25bps pool
execute("remove_liquidity", "$ZRA+0000,$USDC+0000,500000000000,25")
```

---

## Fee Economics

### Default Configuration

- **Treasury Fee**: 12.5% of the LP fee (125 out of 1000)
- **LP Reward Fee**: 87.5% of the LP fee (stays in pool)
- **Platform Fee**: 0-5% of output (set per-swap by caller)

### Fee Flow Example (25 bps pool, default treasury)

For a 1000 ZRA swap:
```
Total LP Fee:    2.5 ZRA (0.25%)
  ├─ Treasury:   0.3125 ZRA (12.5% of 2.5)
  └─ LP Reward:  2.1875 ZRA (87.5% of 2.5, stays in pool)
  
Output: ~997.5 ZRA equivalent in token2 (minus slippage)
  ├─ Platform:   Optional (0-5% of output)
  └─ User:       Remainder after platform fee
```

### Fee Distribution

```
Swap Input
  ├── Treasury Fee → Treasury Wallet (gov-configurable)
  ├── LP Reward → Stays in Pool (accrues to LP holders)
  └── Swap Amount → Constant Product Calculation
                     └── Output Tokens
                          ├── Platform Fee → Platform Wallet (optional)
                          └── User Receives remainder
```

---

## Security Features

1. **Proxy Authorization**: All operations must come through the proxy wallet
2. **Governance Control**: Fee updates require governance key
3. **Balance Validation**: Checks user balances before all operations
4. **Duplicate Prevention**: Cannot create duplicate pools
5. **Wallet Validation**: Base58 address validation for treasury and platform wallets
6. **Fee Caps**: Treasury fee max 1000, platform fee max 500 bps (5%)
7. **LP Token Locking**: Optional time-locks prevent rug pulls
8. **Derived Wallets**: Pool funds isolated in program-derived wallets
9. **Pool Deactivation**: Pools marked inactive when fully drained
10. **Denomination Handling**: Validates token denominations are powers of 10

---

## Integration

### ACE Price Oracle

The DEX serves as a **price oracle** for the ACE system:

- ZRA-paired pools at 25 bps automatically update on-chain prices
- Other contracts query these prices for USD equivalent calculations
- Price cleared when pool is fully drained (deactivated)

### Network Fee Calculations

Prices from DEX pools feed into:
- Bridge rate limiting (USD value calculations)
- Staking reward valuations
- Network fee USD equivalent computations

---

## Key Addresses

- **DEX Proxy**: `3uct7y6rcxW3KA8o8b2gqtaygw7hA39P3SyjV466fXWP`
- **Governance Key**: `gov_$ACE+0000`
- **Treasury Wallet**: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`
- **LP Burn Address**: `DeXBurnDexBurnDexBurnDexBurnDexBurnDex`
- **ZRA Contract**: `$ZRA+0000`

## Initial Configuration

- **Default Implementation**: zera_dex, instance 1
- **Governance Keys**: `gov_$ACE+0000`
- **Treasury Fee**: 125 (12.5% of LP fee)
- **Treasury Wallet**: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

## Notes

- All amounts use their respective token denominations (variable decimals)
- Scaling to 10^9 normalizes calculations across different token decimals
- LP tokens are a separate on-chain token created per pool
- Pool state is stored on the proxy contract via `delegate_store_state()`
- Token pair ordering is handled automatically (ZRA always first if present)
- Only one pool per token pair per fee tier is allowed
- Proxy address is hardcoded in implementation for security
