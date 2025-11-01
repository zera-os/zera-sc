# Circulating Supply Management Contracts

## Overview

These contracts manage a whitelist of wallet addresses that are excluded from circulating supply calculations. This is essential for accurate market cap reporting and excludes wallets like treasury, staking contracts, and burn addresses from circulating supply metrics.

## Contracts

### 1. circulating_whitelist_v1 (Implementation Contract)

**Location**: `circulating_supply/circulating_whitelist_v1/src/lib.rs`

**Purpose**: Maintains a whitelist of addresses excluded from circulating supply calculations.

**Key Functions**:

- `init()`: Initializes the whitelist with default excluded addresses
- `add_wallet(wallet: String)`: Adds a wallet to the whitelist (restricted to proxy)
- `remove_wallet(wallet: String)`: Removes a wallet from the whitelist (restricted to proxy)

**State Storage**:
- `WhiteList`: HashSet of wallet addresses excluded from circulating supply
- `WHITE_LIST_NETWORK`: CSV string of all whitelisted addresses for network-wide queries

**Default Whitelist** (initialized in `init()`):
- `W5jE3KNH` - :fire: encoded (burn address)
- `AZfFcttA3nwqmEYzAtsmufops7PaxLYavvkkDRsxTX5j` - Early backers proxy
- `AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8` - Staking proxy
- `8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko` - Principle proxy
- `4KjrhiQMoK999KxK3yjmuGw8LypoJDh1JzqcdmErG2NX` - IIT gov wallet
- `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH` - Treasury SC
- `3yygVMvY5DdRENZuM4J7NUXwiMhyfZE1nBfjnnodeHve` - ZMT gov wallet

**Authorization**:
- Only calls from the proxy wallet (`EMK16opdneub97v9qC4NdSkMTsnvHsRf4LrdZ2KH3cky`) can modify the whitelist

**Initialization Fee**: 0.5 USD equivalent (converted to ZRA and held on initialization)

---

### 2. circulating_supply_proxy (Proxy Contract)

**Location**: `circulating_supply/circulating_supply_proxy/src/lib.rs`

**Purpose**: Provides an upgradeable proxy pattern for whitelist management with governance controls.

**Key Functions**:

- `init()`: Initializes proxy with default implementation (`circulating_whitelist_v1` instance 1)
- `execute(function: String, parameters: String)`: Delegates function calls to the implementation contract
- `update(smart_contract: String, instance: String)`: Upgrades to a new implementation version
- `update_update_key(update_key: String)`: Changes the governance key for upgrades
- `update_send_all_key(send_all_key: String)`: Changes the key for fund withdrawals
- `send_all(wallet: String)`: Transfers all contract funds to specified wallet

**State Management**:
- `SmartContractState`: Tracks current implementation contract and instance
- `GovKeys`: Stores `update_key` and `send_all_key` for governance

**Authorization**:
- Update operations: Requires `update_key` (default: `gov_$ZRA+0000`)
- Send all operations: Requires `send_all_key` (default: `gov_$ZRA+0000`)

**Initialization Fee**: 0.5 USD equivalent (converted to ZRA and held on initialization)

**Network State**: Stores implementation version as `WHITELIST_SC` = "circulating_whitelist_v1_1"

## Architecture

```
Governance → circulating_supply_proxy → circulating_whitelist_v1
                                     ↓
                              Network queries whitelist
                              for supply calculations
```

## Data Structure

### WhiteList
```rust
pub struct WhiteList {
    pub wallets: HashSet<String>
}
```

Stored as:
- **Internal**: Base64-encoded postcard serialization
- **Network**: CSV string for easy querying

## Use Cases

### 1. Market Cap Calculation
```
Circulating Supply = Total Supply - Sum(Whitelisted Wallet Balances)
Market Cap = Circulating Supply × Price
```

### 2. Excluded Categories
- **Staking Contracts**: Tokens locked in staking
- **Treasury**: Government-controlled funds
- **Burn Address**: Permanently removed tokens
- **Team/Vesting**: Locked allocations

### 3. Reporting
Provides accurate circulating supply for:
- Exchange listings
- Market data aggregators
- Analytics platforms
- Governance transparency

## Operations

### Adding a Wallet
1. Governance calls `execute()` on proxy
2. Proxy delegates to `add_wallet()` in implementation
3. Wallet added to HashSet
4. Network state updated with new CSV list

### Removing a Wallet
1. Governance calls `execute()` on proxy
2. Proxy delegates to `remove_wallet()` in implementation
3. Wallet removed from HashSet
4. Network state updated with new CSV list

### Querying the Whitelist
External systems can query:
- Contract state: `WHITE_LIST` key (serialized HashSet)
- Network state: `WHITE_LIST_NETWORK` key (CSV string)

## Security Features

1. **Proxy Pattern**: Allows upgrades without data migration
2. **Governance Control**: Only governance can modify whitelist
3. **Authorization Checks**: Implementation verifies caller is proxy
4. **Dual Storage**: Internal HashSet + external CSV for redundancy
5. **Immutable Defaults**: Critical addresses hardcoded at initialization

## Integration Example

Network calculates circulating supply:
```rust
// Get total supply
let total_supply = smart_contracts::supply_data(ZRA_CONTRACT);

// Get whitelisted addresses
let whitelist_csv = smart_contracts::retrieve_state("WHITE_LIST_NETWORK");
let whitelist_wallets = whitelist_csv.split(',');

// Calculate locked supply
let mut locked_supply = U256::zero();
for wallet in whitelist_wallets {
    let balance = smart_contracts::wallet_balance(ZRA_CONTRACT, wallet);
    locked_supply += balance;
}

// Circulating = Total - Locked
let circulating_supply = total_supply - locked_supply;
```

## Upgrade Process

1. Deploy new whitelist implementation (e.g., `circulating_whitelist_v2`)
2. Call `update()` on proxy with new contract name and instance
3. Proxy updates routing to new implementation
4. Old whitelist data migrates if needed
5. Network state key updates to new version

## Initial Configuration

- **Default Implementation**: circulating_whitelist_v1, instance 1
- **Governance Keys**: Both set to `gov_$ZRA+0000`
- **ZRA Contract**: `$ZRA+0000`
- **Initial Whitelist**: 7 addresses (see above)

## Notes

- Whitelist changes take effect immediately
- Both internal and network state are updated synchronously
- CSV format allows easy external queries
- HashSet provides O(1) lookup performance
- Proxy address is hardcoded in implementation for security

