# Network Fees Contracts

## Overview

These contracts manage all network transaction fees, validator settings, and fee distribution parameters for the Zera Network. They provide a centralized, upgradeable system for adjusting economic parameters through governance.

## Contracts

### 1. network_fees (Implementation Contract)

**Location**: `network_fees/network_fees/src/lib.rs`

**Purpose**: Stores and manages all network fee parameters, validator settings, and fee distribution ratios.

**Key Functions**:

- `init()`: Initializes all fee parameters with default values
- `update_network_fees(target: String, amount: String)`: Updates fee parameters (restricted to proxy)

**Fee Categories**:

#### Key/Hash Fees
- `A_KEY_FEE`: 0.02 USD - Type A key operations
- `B_KEY_FEE`: 0.05 USD - Type B key operations  
- `a_HASH_FEE`: 0.02 USD - Type a hash operations
- `b_HASH_FEE`: 0.05 USD - Type b hash operations
- `c_HASH_FEE`: 0.01 USD - Type c hash operations
- `RESTRICTED_KEY_FEE`: 3 USD - Restricted key operations

#### Transaction Type Fees
- `COIN_TYPE`: 0.00015 USD - Basic coin transactions
- `STORAGE_FEE`: 0.001 USD - Storage operations
- `CONTRACT_TXN_FEE`: 0.00086 USD - Contract transaction fee
- `EXPENSE_RATIO_TYPE`: 0.004 USD - Expense ratio operations
- `ITEM_MINT_TYPE`: 0.001 USD - Item minting
- `MINT_TYPE`: 0.001 USD - Token minting
- `NFT_TYPE`: 0.0003 USD - NFT operations
- `GAS_FEE`: 0.0000025 USD - Gas fee per unit
- `SAFE_SEND`: 0.0001 USD - Safe send operations

#### Governance & Proposal Fees
- `PROPOSAL_TYPE`: 0.005 USD - Creating proposals
- `PROPOSAL_RESULT_TYPE`: 0.01 USD - Proposal results
- `VOTE_TYPE`: 0.0001 USD - Voting transactions
- `DELEGATED_VOTING_TYPE`: 0.001 USD - Delegated voting setup
- `DELEGATED_VOTING_TXN_FEE`: 0.05 USD - Delegated vote transaction
- `DELEGATED_VOTE_TXN_FEE`: 0.001 USD - Individual delegated vote
- `DELEGATED_FEE`: 0.001 USD - General delegation fee
- `FAST_QUORUM_TYPE`: 0.04 USD - Fast quorum operations
- `QUASH_TYPE`: 0.001 USD - Quashing proposals
- `REVOKE_TYPE`: 0.001 USD - Revoking actions

#### Smart Contract Fees
- `SMART_CONTRACT_TYPE`: 0.0004 USD - Basic SC operations
- `SMART_CONTRACT_EXECUTE_TYPE`: 0.0015 USD - SC execution
- `SMART_CONTRACT_INSTANTIATE_TYPE`: 0.01 USD - SC instantiation
- `UPDATE_CONTRACT_TYPE`: 0.075 USD - Contract updates

#### Validator Settings
- `VALIDATOR_REGISTRATION_TYPE`: 0.01 USD - Registration fee type
- `VALIDATOR_REGISTRATION_TXN_FEE`: 0.01 USD - Registration transaction fee
- `VALIDATOR_HEARTBEAT_TYPE`: 0.00005 USD - Heartbeat fee
- `VALIDATOR_HOLDING_MINIMUM`: 25,000 USD - Minimum holding requirement
- `VALIDATOR_MINIMUM_ZERA`: 0.00001 USD - Minimum ZERA requirement
- `VALIDATOR_FEE_PERCENTAGE`: 50% - Validator fee share

#### Compliance & Other
- `COMPLIANCE_TYPE`: 0.001 USD - Compliance operations
- `COMPLIANCE_TXN_FEE`: 0.001 USD - Compliance transaction fee
- `ALLOWANCE_TYPE`: 0.001 USD - Allowance operations
- `SBT_BURN_TYPE`: 0.001 USD - SBT burning
- `ATTESTATION_QUORUM`: 51% - Attestation quorum threshold

#### Fee Distribution
- `BURN_FEE_PERCENTAGE`: 25% - Percentage of fees burned
- `TREASURY_FEE_PERCENTAGE`: 25% - Percentage to treasury

**Update Validation**:

The `update_network_fees()` function validates all updates:
- Amounts must be valid U256 values
- Amounts must be greater than 0
- `VALIDATOR_MINIMUM_ZERA` ≤ 100,000,000,000,000
- `COIN_TYPE` and `VOTE_TYPE` ≤ 10,000,000,000,000,000
- Fee percentages ≤ 100%
- `ATTESTATION_QUORUM` between 51% and 100%
- Other fees ≤ 500,000,000,000,000,000

**Batch Updates**:
- Accepts multiple targets and amounts separated by `**`
- Example: `target="FEE1**FEE2"`, `amount="100**200"`
- All updates in batch must be valid or entire batch fails

**Authorization**:
- Only calls from the proxy wallet (`5o5AkKgjtcqsTVbxNHCRHvUCfL9nJ7F48CqfayJyuRu`) can update fees

**Initialization Fee**: 10 USD equivalent (converted to ZRA and held on initialization)

---

### 2. network_fee_proxy (Proxy Contract)

**Location**: `network_fees/network_fee_proxy/src/lib.rs`

**Purpose**: Provides an upgradeable proxy pattern for network fee management with governance controls.

**Key Functions**:

- `init()`: Initializes proxy with default implementation (`network_fees_v1` instance 1)
- `execute(function: String, parameters: String)`: Delegates function calls to the implementation contract
- `update(smart_contract: String, instance: String)`: Upgrades to a new implementation version
- `update_update_key(update_key: String)`: Changes the governance key for upgrades
- `update_send_all_key(send_all_key: String)`: Changes the key for fund withdrawals
- `send_all()`: Transfers all contract funds to treasury wallet

**State Management**:
- `SmartContractState`: Tracks current implementation contract and instance
- `GovKeys`: Stores `update_key` and `send_all_key` for governance

**Authorization**:
- Update operations: Requires `update_key` (default: `gov_$ZRA+0000`)
- Send all operations: Requires `send_all_key` (default: `gov_$ZRA+0000`)
- Send all destination: Treasury wallet `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

**Initialization Fee**: 0.5 USD equivalent (held on initialization)

**Network State**: Stores implementation version as `NETWORK_SC` = "network_fees_v1_1"

## Architecture

```
Governance → network_fee_proxy → network_fees_v1
                               ↓
                     Network queries for fee calculations
```

## Use Cases

### 1. Transaction Fee Calculation
Network reads appropriate fee based on transaction type:
```rust
let fee = smart_contracts::retrieve_state("COIN_TYPE");
// Apply fee to transaction
```

### 2. Validator Requirements
Check validator eligibility:
```rust
let min_holding = retrieve_state("VALIDATOR_HOLDING_MINIMUM");
let min_zera = retrieve_state("VALIDATOR_MINIMUM_ZERA");
// Verify validator meets requirements
```

### 3. Fee Distribution
Calculate fee allocation:
```rust
let total_fee = calculate_transaction_fee();
let validator_share = total_fee * VALIDATOR_FEE_PERCENTAGE / 100;
let burn_share = total_fee * BURN_FEE_PERCENTAGE / 100;
let treasury_share = total_fee * TREASURY_FEE_PERCENTAGE / 100;
```

### 4. Governance Adjustments
Update fees through governance:
```rust
// Update multiple fees atomically
execute("update_network_fees", 
        "GAS_FEE**STORAGE_FEE,3000000000000**2000000000000000");
```

## Security Features

1. **Range Validation**: All fee updates validated against min/max bounds
2. **Atomic Updates**: Batch updates succeed or fail together
3. **Governance Control**: Only governance can modify fees
4. **Proxy Pattern**: Allows fee logic upgrades without data migration
5. **Authorization Checks**: Implementation verifies caller is proxy
6. **Treasury Protection**: Send all only goes to hardcoded treasury address

## Fee Economics

### Design Principles
- **Low Base Fees**: Keep basic transactions affordable
- **Progressive Complexity**: Higher fees for complex operations
- **Validator Incentives**: Majority of fees go to validators
- **Deflationary**: 25% of fees burned
- **Treasury Funding**: 25% funds ecosystem development

### Fee Distribution Formula
```
Transaction Fee = Base Fee + (Gas Used × Gas Fee)

Distribution:
- 50% → Validators (incentivizes network security)
- 25% → Burn Address (deflationary pressure)
- 25% → Treasury (ecosystem development)
```

## Operations

### Updating a Single Fee
```
1. Governance proposes fee change
2. Community votes on proposal
3. If approved, call execute() on proxy:
   - function: "update_network_fees"
   - parameters: "GAS_FEE,3000000000000"
4. Proxy delegates to implementation
5. Implementation validates and updates
6. Network immediately uses new fee
```

### Batch Update
```
1. Prepare targets: "FEE1**FEE2**FEE3"
2. Prepare amounts: "value1**value2**value3"
3. Call update_network_fees(targets, amounts)
4. All updates applied atomically
```

### Querying Fees
Any contract or user can query:
```rust
let gas_fee = smart_contracts::retrieve_state("GAS_FEE");
let coin_fee = smart_contracts::retrieve_state("COIN_TYPE");
```

## Upgrade Process

1. Deploy new fee contract implementation (e.g., `network_fees_v2`)
2. Test new implementation on testnet
3. Governance proposal for upgrade
4. If approved, call `update()` on proxy
5. Proxy updates routing to new implementation
6. All fee parameters preserved (same state keys)
7. Network state key updates to new version

## Initial Configuration

- **Default Implementation**: network_fees_v1, instance 1
- **Governance Keys**: Both set to `gov_$ZRA+0000`
- **ZRA Contract**: `$ZRA+0000`
- **Treasury**: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`
- **Fee Distribution**: 50% validators, 25% burn, 25% treasury

## Notes

- All fees denominated in USD equivalent (converted to ZRA) with 18 decimal precision
- Fee updates take effect immediately upon successful execution
- Batch updates are atomic - all succeed or all fail
- Validator percentages must sum correctly (implementation enforces)
- Fees stored as strings to prevent overflow in calculations
- Network queries fee state directly for transaction processing
- Proxy address hardcoded in implementation for security

