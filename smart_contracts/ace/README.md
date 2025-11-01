# ACE (Automated Currency Exchange) Contracts

## Overview

The ACE smart contracts manage exchange rate data for the $ZRA token on the Zera Network. These contracts store and update price information that other contracts and network components use for calculations.

## Contracts

### 1. zra_v1 (Implementation Contract)

**Location**: `ace/zra_v1/src/lib.rs`

**Purpose**: Stores and manages the exchange rate for the ZRA token.

**Key Functions**:

- `init()`: Initializes the contract with a default exchange rate of $1 (1000000000000000000 in 18 decimals)
- `update_rate(token: String, price: String)`: Updates the exchange rate for a given token (restricted to proxy wallet)

**State Storage**:
- Stores token exchange rates as string values
- Default ZRA rate: "1000000000000000000" ($1.00)

**Authorization**:
- Only calls from the ACE proxy wallet (`6cifeAScHLGvxARJSdxS6QPdTJLwqgXMmHzXWyFU9tHC`) can update rates
- Uses `called_smart_contract_wallet()` for authorization checks

**Initialization Fee**: 0.1 USD equivalent (converted to ZRA and held on initialization)

---

### 2. ace_proxy (Proxy Contract)

**Location**: `ace/ace_proxy/src/lib.rs`

**Purpose**: Provides an upgradeable proxy pattern for ACE functionality with governance controls.

**Key Functions**:

- `init()`: Initializes proxy with default implementation (`zra_v1` instance 1)
- `execute(function: String, parameters: String)`: Delegates function calls to the implementation contract
- `update(smart_contract: String, instance: String)`: Upgrades to a new implementation version
- `update_update_key(update_key: String)`: Changes the governance key that can perform upgrades
- `update_send_all_key(send_all_key: String)`: Changes the key that can withdraw all funds
- `send_all(wallet: String)`: Transfers all contract funds to specified wallet

**State Management**:
- `SmartContractState`: Tracks current implementation contract and instance
- `GovKeys`: Stores `update_key` and `send_all_key` for governance

**Authorization**:
- Update operations: Requires `update_key` (default: `gov_$ZRA+0000`)
- Send all operations: Requires `send_all_key` (default: `gov_$ZRA+0000`)

**Initialization Fee**: 0.5 USD equivalent (converted to ZRA and held on initialization)

**Network State**: Stores implementation version as `ACE_SC` = "zra_v1_1"

## Architecture

```
User/Governance → ace_proxy → zra_v1 (current implementation)
                           ↓
                    Future versions (zra_v2, etc.)
```

## Integration

Other contracts query ACE data using:
```rust
let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
```

This returns:
- `authorized`: Boolean indicating if the rate is authorized
- `rate`: U256 value representing the current exchange rate

## Security Features

1. **Proxy Pattern**: Allows upgrades without changing the proxy address
2. **Governance Control**: All privileged operations require governance keys
3. **Authorization Checks**: Only proxy can update rates in implementation
4. **State Isolation**: Implementation and proxy maintain separate state

## Use Cases

1. **Price Oracles**: Provides exchange rate data for DeFi operations
2. **Fee Calculations**: Network uses ACE data to calculate fee equivalents in different tokens
3. **Staking Rewards**: Converts reward amounts based on current rates
4. **Treasury Management**: Tracks value of treasury holdings

## Upgrade Process

1. Deploy new implementation contract (e.g., `zra_v2`)
2. Call `update()` on proxy with new contract name and instance
3. Proxy updates `SMART_CONTRACT_KEY` and `ACE_SC` network state
4. All subsequent calls route to new implementation
5. Old implementation data remains accessible but unused

## Initial Configuration

- **Default Implementation**: zra_v1, instance 1
- **Governance Keys**: Both set to `gov_$ZRA+0000`
- **Default ZRA Rate**: $1.00 (1e18 precision)
- **ZRA Contract**: `$ZRA+0000`

## Notes

- Exchange rates use 18 decimal precision
- Rates are stored as strings to prevent overflow
- Contract holds ZRA to cover network fees
- Proxy address is hardcoded in implementation for security

