# Restricted Symbols Contracts

## Overview

These contracts manage a registry of restricted token symbols that cannot be used by regular users when creating new tokens. This prevents impersonation of official tokens and protects key ecosystem symbols.

## Contracts

### 1. restricted_symbols_v1 (Implementation Contract)

**Location**: `restricted_symbols/restricted_symbols_v1/src/lib.rs`

**Purpose**: Maintains a list of restricted token symbols that are reserved for official use only.

**Key Functions**:

- `init()`: Initializes the contract with default restricted symbols
- `add_symbol(symbol: String)`: Adds a symbol to the restricted list (restricted to proxy)
- `remove_symbol(symbol: String)`: Removes a symbol from the restricted list (restricted to proxy)

**State Storage**:
Each restricted symbol is stored individually as a key-value pair:
- Key: Symbol name (e.g., "ZRA", "ACE")
- Value: "true" (indicating restriction is active)

**Default Restricted Symbols** (initialized in `init()`):
- `ZRA` - Zera native token
- `ACE` - Automated Currency Exchange
- `ZIP` - Zera Improvement Proposal governance
- `LEGAL` - Legal/compliance related
- `TREASURY` - Treasury operations
- `IIT` - Infrastructure & Innovation Treasury
- `ZMT` - Zera Management Treasury
- `BRIDGEGUARDIAN` - Bridge security
- `BRIDGETOKENS` - Bridge token operations

**Authorization**:
- Only calls from the proxy wallet (`H7YTw7bry3VQVmAADF3tQw4eGYUMuenac3rbNb7r5SZA`) can modify symbols

**Initialization Fee**: 0.5 USD equivalent (converted to ZRA and held on initialization)

---

### 2. restricted_symbols_proxy (Proxy Contract)

**Location**: `restricted_symbols/restricted_symbols_proxy/src/lib.rs`

**Purpose**: Provides an upgradeable proxy pattern for symbol restriction management with governance controls.

**Key Functions**:

- `init()`: Initializes proxy with default implementation (`restricted_symbols_v1` instance 1)
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

**Initialization Fee**: 0.5 USD equivalent (converted to ZRA and held on initialization)

**Network State**: Stores implementation version as `RESTRICTED_SC` = "restricted_symbols_v1_1"

## Architecture

```
Network Token Creation → Check Restricted Symbols → restricted_symbols_proxy 
                                                    ↓
                                            restricted_symbols_v1
                                                    ↓
                                            Allow or Reject
```

## Use Cases

### 1. Token Creation Validation
When a user attempts to create a new token:
```rust
let symbol = "ZRA";  // User's proposed symbol
let is_restricted = smart_contracts::retrieve_state(symbol);

if is_restricted == "true" {
    return Error("Symbol is restricted");
}
// Proceed with token creation
```

### 2. Protecting Official Symbols
Prevents confusion and scams:
- Users cannot create "$ZRA" imposters
- Official governance symbols protected
- Treasury and bridge symbols reserved

### 3. Dynamic Symbol Management
Governance can add new restrictions:
```rust
// Add new symbol restriction
execute("add_symbol", "NEWTOKEN");

// Remove restriction if needed
execute("remove_symbol", "OLDTOKEN");
```

## Data Structure

### Storage Format
Unlike other contracts, this uses direct key-value storage:
```
Key: "ZRA" → Value: "true"
Key: "ACE" → Value: "true"
Key: "TREASURY" → Value: "true"
...
```

This allows O(1) lookup during token creation without deserializing complex structures.

## Operations

### Adding a Restricted Symbol
1. Governance proposes new restriction
2. Community votes on proposal  
3. If approved, call `execute()` on proxy:
   - function: "add_symbol"
   - parameters: "NEWSYMBOL"
4. Proxy delegates to implementation
5. Implementation stores: `NEWSYMBOL → "true"`
6. Network immediately enforces restriction

### Removing a Restriction
1. Governance proposes unrestricting symbol
2. Community votes on proposal
3. If approved, call `execute()` on proxy:
   - function: "remove_symbol"
   - parameters: "OLDSYMBOL"
4. Proxy delegates to implementation  
5. Implementation calls `clear_state(OLDSYMBOL)`
6. Symbol becomes available for use

### Checking Restrictions (Network Level)
```rust
// During token creation
let symbol_status = smart_contracts::retrieve_state(proposed_symbol);

match symbol_status.as_str() {
    "true" => Err("Symbol is restricted"),
    "" => Ok("Symbol available"),
    _ => Ok("Symbol available")
}
```

## Security Features

1. **Proxy Pattern**: Allows logic upgrades without data migration
2. **Governance Control**: Only governance can modify restrictions
3. **Authorization Checks**: Implementation verifies caller is proxy
4. **Immutable Defaults**: Critical symbols hardcoded at initialization
5. **Simple Storage**: Direct key-value for performance and security
6. **Treasury Protection**: Send all only goes to treasury

## Protected Categories

### Core Protocol Symbols
- `ZRA` - Main network token
- `ACE` - Exchange rate system

### Governance Symbols
- `ZIP` - Zera Improvement Proposals
- `LEGAL` - Legal governance
- `IIT` - Infrastructure treasury
- `ZMT` - Management treasury
- `TREASURY` - General treasury

### Bridge Symbols
- `BRIDGEGUARDIAN` - Bridge security
- `BRIDGETOKENS` - Bridge token management

## Integration Example

### Network Token Creation Flow
```rust
fn create_token(symbol: String, name: String, supply: U256) -> Result<String> {
    // 1. Validate symbol is not restricted
    let restricted_check = smart_contracts::retrieve_state(symbol.clone());
    
    if restricted_check == "true" {
        return Err("Error: Symbol is restricted for official use only");
    }
    
    // 2. Check symbol format and length
    if !is_valid_symbol_format(&symbol) {
        return Err("Error: Invalid symbol format");
    }
    
    // 3. Check symbol not already in use
    if smart_contracts::contract_exists(format!("${}+0000", symbol)) {
        return Err("Error: Symbol already exists");
    }
    
    // 4. Create the token
    let result = smart_contracts::instrument_contract_bridge(
        symbol,
        name,
        supply.to_string(),
        // ... other parameters
    );
    
    Ok(result)
}
```

## Upgrade Process

1. Deploy new symbols implementation (e.g., `restricted_symbols_v2`)
2. Test new implementation on testnet
3. Governance proposal for upgrade
4. If approved, call `update()` on proxy
5. Proxy updates routing to new implementation
6. All symbol restrictions preserved (same state keys)
7. Network state key updates to new version

## Best Practices

### For Governance
- Add restrictions proactively for new official tokens
- Review restriction list periodically
- Only unrestrict after careful consideration
- Document reasons for each restriction

### For Developers
- Always check restrictions before token creation
- Provide clear error messages to users
- Cache restriction list if making multiple checks
- Handle empty/missing state as "not restricted"

### For Users
- Choose unique, non-official symbols
- Avoid abbreviations of well-known brands
- Check symbol availability before planning token
- Use descriptive names to avoid confusion

## Initial Configuration

- **Default Implementation**: restricted_symbols_v1, instance 1
- **Governance Keys**: Both set to `gov_$ZRA+0000`
- **ZRA Contract**: `$ZRA+0000`
- **Treasury**: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`
- **Initial Restrictions**: 9 symbols (see above)

## Notes

- Symbol checks are case-sensitive
- Restrictions apply to exact symbol matches only
- Network enforces at token creation time
- Existing tokens not affected by new restrictions
- State stored as individual keys for performance
- Empty `retrieve_state()` result means unrestricted
- Proxy address hardcoded in implementation for security

