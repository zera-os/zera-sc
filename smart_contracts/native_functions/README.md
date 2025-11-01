# Native Functions Library

## Overview

The native functions library provides a comprehensive Rust wrapper around Zera Network's core blockchain functionality. This is the foundational library used by all smart contracts on the network to interact with the blockchain runtime.

**Location**: `native_functions/src/lib.rs`

## Purpose

Provides type-safe Rust bindings for:
- State management (storage/retrieval)
- Token operations (send, mint, transfer)
- Smart contract calls (call, delegatecall)
- Cryptographic functions (hashing, signatures)
- Network queries (balance, supply, compliance)
- Governance operations (voting, proposals)

## Module Structure

### 1. zera::types
High-precision numeric types and utilities.

**Types**:
- `U256`: 256-bit unsigned integer for token amounts
- `string_to_u256()`: Converts decimal strings to U256
- `is_valid_u256()`: Validates U256 string format

### 2. zera::smart_contracts
Core smart contract functionality.

#### State Management

**Storage**:
- `store_state(key: String, value: String) -> bool`
- `delegate_store_state(key: String, value: String, contract: String) -> bool`
- `clear_state(key: String)`
- `delegate_clear_state(key: String, contract: String)`

**Retrieval**:
- `retrieve_state(key: String) -> String`
- `delegate_retrieve_state(key: String, contract: String) -> String`

**Database**:
- `db_get_data(key: String) -> String`
- `db_get_any_data(key: String, db_key: String) -> String`

#### Token Operations

**Original Context** (from initiating smart contract):
- `hold(contract_id: String, amount: String) -> bool`: Lock tokens in contract
- `send(contract_id: String, amount: String, address: String) -> bool`: Send tokens
- `send_all(wallet_address: String) -> String`: Transfer all assets
- `mint(contract_id: String, amount: String, address: String) -> bool`: Create new tokens
- `transfer(contract_id: String, amount: String, address: String) -> bool`: Transfer tokens
- `send_multi(contract_id: String, input_amounts: String, amounts: Vec<String>, addresses: Vec<String>) -> bool`: Batch send

**Current Context** (from latest contract in call stack):
- `current_hold(contract_id: String, amount: String) -> bool`
- `current_send(contract_id: String, amount: String, address: String) -> bool`
- `current_send_all(wallet_address: String) -> String`
- `current_mint(contract_id: String, amount: String, address: String) -> bool`

**Delegated Context** (specify which contract's context):
- `delegate_send(contract_id: String, amount: String, address: String, sc_wallet: String) -> bool`
- `delegate_send_all(wallet_address: String, sc_wallet: String) -> String`
- `delegate_mint(contract_id: String, amount: String, address: String, sc_wallet: String) -> bool`

#### Smart Contract Calls

- `call(contract_name: String, nonce: String, function_name: String, parameters: Vec<String>) -> Vec<String>`
  - Creates new contract instance
  - Returns array of emitted results

- `delegatecall(contract_name: String, nonce: String, function_name: String, parameters: Vec<String>) -> Vec<String>`
  - Maintains calling context
  - Used for upgradeable proxy pattern

#### Network Queries

**Contract Information**:
- `contract_exists(contract_id: String) -> bool`
- `contract_denomination(contract_id: String) -> U256`
- `circulating_supply(contract_id: String) -> U256`
- `smart_contract_balance(contract_id: String) -> U256`

**Wallet Information**:
- `wallet_balance(contract_id: String, wallet_address: String) -> U256`
- `wallet_tokens(wallet_address: String) -> Vec<String>`
- `wallet_address() -> String`: Get transaction sender
- `public_key() -> String`: Get sender's public key

**Smart Contract Context**:
- `smart_contract_wallet() -> String`: Original SC wallet
- `current_smart_contract_wallet() -> String`: Current SC wallet
- `called_smart_contract_wallet() -> String`: Calling SC wallet

**Transaction Info**:
- `txn_hash() -> String`: Current transaction hash
- `last_block_time() -> u64`: Latest block timestamp

#### Compliance & Authorization

**Compliance**:
- `compliance(contract_id: String, wallet_address: String) -> bool`
- `compliance_levels(contract_id: String, wallet_address: String) -> Vec<u32>`

**Allowances**:
- `allowance(contract_id: String, wallet_address: String, allowed_equiv: String, allowed_amount: String, period_months: String, period_seconds: String, start_time: String) -> String`
- `allowance_sender(...)`: Set allowance for sender
- `allowance_sender_deauthorize(contract_id: String, wallet_address: String) -> String`

**Exchange Rates**:
- `get_ace_data(contract: String) -> (bool, U256)`: Get authorized exchange rate
- `authorized_currency_equiv(contract_ids: String, rates: String, authorized: String, max_stakes: String) -> String`

#### Governance

**Voting**:
- `vote(proposal_id: String, support: bool) -> String`
- `vote_options(proposal_id: String, support: u32) -> String`

**Contract Management**:
- `instrument_contract_bridge(symbol: String, name: String, denomination: String, contract_id: String, mint_id: String, uri: String, authorized_key: String, wallet: String, amount: String) -> String`
- `expense_ratio(contract_id: String, output_address: String, addresses: Vec<String>) -> String`

#### Cryptography

**Hashing**:
- `sha256(data: String) -> String`
- `sha512(data: String) -> String`
- `blake3(data: String, length: Blake3HashLength) -> String`
  - Lengths: 256, 512, 1024, 2048, 4096, 9001 bits
- `shake(data: String, length: SHAKEHashLength) -> String`
  - Lengths: 1024, 2048, 4096 bits

**Signatures**:
- `verify_signature(message: String, signatures: String, public_key: String) -> bool`

**Miscellaneous**:
- `emit(value: String) -> bool`: Emit event/log
- `version() -> i32`: Runtime version

## Usage Patterns

### Proxy Pattern
```rust
// Delegate call to implementation
let results = smart_contracts::delegatecall(
    impl_contract.to_string(),
    instance.to_string(),
    "function_name".to_string(),
    parameters
);
```

### State Management
```rust
// Serialize and store complex data
fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
    let bytes = postcard::to_allocvec(data).unwrap();
    let b64 = base64::encode(bytes);
    unsafe { smart_contracts::store_state(key.to_string(), b64) }
}
```

### Token Operations
```rust
// Send tokens with safety checks
if smart_contracts::send(
    contract_id,
    amount.to_string(),
    recipient_address
) {
    emit("Success: Tokens sent");
} else {
    emit("Failed: Transfer failed");
}
```

### Authorization
```rust
// Check caller authorization
let caller = smart_contracts::called_smart_contract_wallet();
if caller != AUTHORIZED_WALLET {
    return; // Unauthorized
}
```

## Safety Considerations

1. **Unsafe Blocks**: All network calls are wrapped in `unsafe` blocks
2. **String Conversion**: Amounts are strings to prevent overflow
3. **Buffer Management**: Vectors resized appropriately for responses
4. **Error Handling**: Check return values for success/failure

## Type Precision

- **Token Amounts**: U256 (supports up to 2^256-1)
- **Timestamps**: u64 (Unix seconds)
- **Decimals**: Typically 18 for token amounts
- **Hash Lengths**: Variable (256 to 9001 bits)

## Integration

All Zera smart contracts import this library:
```rust
use native_functions::zera::wasmedge_bindgen;
use native_functions::zera::smart_contracts;
use native_functions::zera::types;
use native_functions::zera::types::U256;
```

## WasmEdge Bindings

The library uses WasmEdge FFI to call native blockchain functions:
- `#[link(wasm_import_module = "native_functions")]`
- Extern "C" declarations for all native functions
- Pointer-based parameter passing for efficient memory usage

## Notes

- All functions return String or primitive types for cross-language compatibility
- Multi-value returns use CSV or custom delimiters
- State keys should be prefixed to avoid collisions
- Amount strings must be valid decimal numbers
- Contract IDs follow format: `$SYMBOL+0000` or wallet addresses

