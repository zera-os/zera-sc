# Treasury Contracts

## Overview

The treasury contracts manage the Zera Network's central treasury funds. These contracts provide governance-controlled fund management with multi-signature authorization for different operations.

## Contracts

### 1. treasury (zera_treasury_v1) - Treasury Implementation

**Location**: `treasury/treasury/src/lib.rs`

**Purpose**: Manages treasury funds with governance-controlled transfers and strict authorization requirements.

**Key Functions**:

#### Initialization
- `init()`: Simple initialization with no state setup

#### Fund Management
- `send(contract_id: String, amount: String, wallet_address: String)`
  - Sends specific amount of a token to a wallet address
  - Requires multi-governance authorization
  - Authorized keys:
    - `gov_$TREASURY+0000` (Treasury governance)
    - `gov_$ZRA+0000` (ZRA governance)
    - `gov_$IIT+0000` (Infrastructure & Innovation Treasury governance)
    - `gov_$ZMT+0000` (Zera Management Treasury governance)
    - `gov_$ZIP+0000` (Zera Improvement Proposal governance)

- `send_all(wallet_address: String)`
  - Transfers ALL treasury funds to specified wallet
  - Requires highest-level authorization
  - Authorized keys:
    - `gov_$TREASURY+0000` (Treasury governance)
    - `gov_$ZRA+0000` (ZRA governance)
  - **Warning**: This is a highly privileged operation

**Authorization Model**:
```rust
let pub_key = smart_contracts::public_key();

// For send operations - 5 authorized governance keys
if pub_key != "gov_$TREASURY+0000" 
   && pub_key != "gov_$ZRA+0000" 
   && pub_key != "gov_$IIT+0000" 
   && pub_key != "gov_$ZMT+0000" 
   && pub_key != "gov_$ZIP+0000" {
    emit("Failed: Unauthorized sender key");
    return;
}

// For send_all operations - 2 authorized governance keys (stricter)
if pub_key != "gov_$TREASURY+0000" 
   && pub_key != "gov_$ZRA+0000" {
    emit("Failed: Unauthorized sender key");
    return;
}
```

**Security Features**:
- Multiple governance keys for different authorization levels
- Stricter requirements for `send_all` operations
- All operations emit events for transparency
- Direct authorization check (not via proxy)

---

### 2. treasury_proxy (zera_treasury_proxy) - Treasury Proxy

**Location**: `treasury/treasury_proxy/src/lib.rs`

**Purpose**: Provides an upgradeable proxy pattern for treasury management with additional governance controls.

**Key Functions**:

- `init()`: Initializes proxy with default implementation (`zera_treasury_v1` instance 1)
- `execute(function: String, parameters: String)`: Delegates function calls to the implementation contract
- `update(smart_contract: String, instance: String)`: Upgrades to a new implementation version
- `update_update_key(update_key: String)`: Changes the governance key for upgrades
- `update_send_all_key(send_all_key: String)`: Changes the key for fund withdrawals from proxy
- `send_all()`: Transfers all proxy funds to governance wallet (not same as treasury.send_all())

**State Management**:
- `SmartContractState`: Tracks current implementation contract and instance
- `GovKeys`: Stores `update_key` and `send_all_key` for governance

**Authorization**:
- Update operations: Requires `update_key` (default: `gov_$ZRA+0000`)
- Proxy send all: Requires `send_all_key` (default: `gov_$ZRA+0000`)
- Implementation operations: Implementation contract handles its own authorization

**Initialization Fee**: 0.5 USD equivalent (converted to ZRA and held on proxy initialization)

**Key Addresses**:
- Treasury Wallet: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`
- Governance Wallet: `46oLKvxWo9JASRrhJN3i4FDBhJPXTCgr1GtTe8pe8ECc`

**Network State**: Stores implementation version (optional, not set in init)

## Architecture

```
Governance Keys → treasury_proxy → zera_treasury_v1 → Treasury Funds
                                                    ↓
                                          Controlled Transfers
```

### Dual-Level Authorization

**Level 1: Proxy Controls**
- Upgrade implementation
- Withdraw fees held by proxy itself

**Level 2: Implementation Controls**  
- Transfer treasury funds
- Multi-governance authorization
- Different authorization levels for different operations

## Use Cases

### 1. Ecosystem Development Funding
```rust
// IIT governance approves developer grant
treasury_proxy.execute(
    "send",
    "$ZRA+0000,1000000000000000000000,developer_wallet"
);
// Sends 1,000 ZRA to developer
```

### 2. Infrastructure Investment
```rust
// Treasury governance funds infrastructure
treasury_proxy.execute(
    "send",
    "$ZRA+0000,5000000000000000000000,infrastructure_wallet"
);
// Sends 5,000 ZRA to infrastructure project
```

### 3. Emergency Fund Release
```rust
// In emergency, top governance can move all funds
// Requires gov_$ZRA+0000 or gov_$TREASURY+0000
treasury_proxy.execute(
    "send_all",
    "emergency_multisig_wallet"
);
// Transfers entire treasury to secure location
```

### 4. Contract Upgrade
```rust
// Upgrade treasury logic
treasury_proxy.update(
    "zera_treasury_v2",
    "1"
);
// Routes all future calls to new implementation
```

## Governance Structure

### Governance Keys

**Primary Governance** (`gov_$ZRA+0000`):
- Highest level authority
- Can upgrade contracts
- Can authorize all treasury operations
- Can perform `send_all` operations

**Treasury Governance** (`gov_$TREASURY+0000`):
- Treasury-specific governance
- Can authorize treasury transfers
- Can perform `send_all` operations

**Specialized Governance**:
- `gov_$IIT+0000` - Infrastructure & Innovation Treasury
- `gov_$ZMT+0000` - Zera Management Treasury
- `gov_$ZIP+0000` - Zera Improvement Proposals
- Each can authorize specific treasury transfers

### Authorization Matrix

| Operation | Required Keys |
|-----------|---------------|
| `send()` | gov_$TREASURY, gov_$ZRA, gov_$IIT, gov_$ZMT, or gov_$ZIP |
| `send_all()` | gov_$TREASURY or gov_$ZRA only |
| `update()` | gov_$ZRA (via proxy) |
| `update_update_key()` | Current update_key (via proxy) |
| `update_send_all_key()` | Current send_all_key (via proxy) |
| Proxy `send_all()` | send_all_key (via proxy) |

## Security Features

1. **Multi-Signature Governance**: Multiple authorized keys for different operations
2. **Tiered Authorization**: Stricter requirements for sensitive operations
3. **Proxy Pattern**: Upgradeable logic without fund migration
4. **Event Emission**: All operations emit events for transparency
5. **Direct Authorization**: Implementation checks keys directly (not via proxy call)
6. **Separate Proxy Funds**: Proxy holds its own initialization fee separately from treasury

## Operations

### Standard Treasury Transfer

**Process**:
1. Governance proposal created with transfer details
2. Community votes on proposal
3. If approved, authorized governance key executes:
```rust
treasury_proxy.execute(
    "send",
    "contract_id,amount,recipient_address"
)
```
4. Proxy delegates to implementation
5. Implementation validates governance key
6. Transfer executes
7. Event emitted for transparency

### Emergency Treasury Move

**Process**:
1. Emergency situation identified
2. Top-level governance (gov_$ZRA or gov_$TREASURY) decides
3. Execute `send_all`:
```rust
treasury_proxy.execute(
    "send_all",
    "secure_multisig_address"
)
```
4. Entire treasury moves to secure location
5. Operations can continue from new location

### Contract Upgrade

**Process**:
1. New treasury implementation deployed and tested
2. Governance proposal for upgrade
3. Community votes
4. If approved, gov_$ZRA executes:
```rust
treasury_proxy.update(
    "zera_treasury_v2",
    "1"
)
```
5. All future calls route to new implementation
6. Funds remain in treasury wallet (unchanged)

## Fund Flow

### Incoming Flows to Treasury
- Network transaction fees (25% of fees)
- Protocol revenue
- Grants and donations
- Token sales

### Outgoing Flows from Treasury
- Developer grants
- Infrastructure funding
- Marketing and growth
- Security audits
- Community initiatives
- Emergency reserves

### Distribution Authority
- `gov_$IIT+0000`: Infrastructure projects
- `gov_$ZMT+0000`: Management operations  
- `gov_$ZIP+0000`: Community proposals
- `gov_$TREASURY+0000`: General treasury operations
- `gov_$ZRA+0000`: Top-level decisions

## Integration Example

### Funding a ZIP Proposal
```rust
// ZIP governance approves community proposal
let contract_id = "$ZRA+0000";
let amount = "10000000000000000000000"; // 10,000 ZRA
let recipient = "community_project_wallet";

// ZIP governance signs transaction
let result = treasury_proxy.execute(
    "send",
    format!("{},{},{}", contract_id, amount, recipient)
);

// Implementation checks:
// 1. Is caller gov_$ZIP+0000? ✓
// 2. Transfer funds
// 3. Emit success event
```

## Upgrade Process

1. **Develop New Implementation**: Create `zera_treasury_v2`
2. **Test Thoroughly**: Testnet validation
3. **Governance Proposal**: Submit upgrade proposal with details
4. **Community Vote**: Approve or reject
5. **Execute Upgrade**: Call `update()` on proxy
6. **Verify**: Confirm funds accessible and operations work
7. **Monitor**: Watch for any issues post-upgrade

## Initial Configuration

- **Default Implementation**: zera_treasury_v1, instance 1
- **Governance Keys**: update_key and send_all_key = `gov_$ZRA+0000`
- **Treasury Wallet**: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`
- **Governance Wallet** (for proxy): `46oLKvxWo9JASRrhJN3i4FDBhJPXTCgr1GtTe8pe8ECc`
- **ZRA Contract**: `$ZRA+0000`

## Important Distinctions

### Treasury Implementation `send_all()` vs Proxy `send_all()`

**Implementation `send_all(wallet_address: String)`**:
- Transfers ALL treasury funds
- Requires gov_$TREASURY or gov_$ZRA
- Moves entire treasury to specified wallet
- Emergency operation

**Proxy `send_all()`**:
- Transfers fees held by proxy contract itself
- Goes to governance wallet
- Only affects proxy balance, not main treasury
- Routine maintenance operation

### Fund Locations

1. **Treasury Wallet** (`4Yg2Ze...`): Main treasury funds
2. **Proxy Contract**: Holds 0.5 ZRA initialization fee
3. **Implementation Contract**: Logic only, no funds

## Best Practices

### For Governance
- Use appropriate governance key for operation type
- Document all treasury transactions
- Regular transparency reports
- Multi-sig for large transfers when possible
- Reserve `send_all` for true emergencies

### For Development
- Test all changes on testnet first
- Validate governance keys before operations
- Emit detailed events for all operations
- Handle authorization failures gracefully
- Consider gas costs for large transfers

### For Auditing
- Monitor all treasury events
- Track fund flows
- Verify governance authorization
- Report suspicious activity
- Regular balance reconciliation

## Notes

- Treasury wallet address is the actual fund holder
- Proxy only routes calls, doesn't hold main funds
- Implementation has no state, only logic
- All amounts use 18 decimal precision
- Contract ID format: `$SYMBOL+0000` or wallet address
- Events emitted for all operations for transparency
- Multiple governance keys provide checks and balances

