# Zera ↔ Solana Bridge Contracts

## Overview

The Zera-Solana bridge is a bidirectional cross-chain bridge enabling asset transfers between the Zera Network and Solana blockchain. The bridge uses a guardian-based security model with multi-signature verification to ensure secure cross-chain transactions.

## Architecture

The bridge consists of two main components:

### 1. **Solana Side** (Anchor Framework - Rust)
- **Core Bridge**: Manages governance, upgrades, guardians, and security
- **Token Bridge**: Handles token operations (lock/release/mint/burn)

### 2. **Zera Network Side** (WasmEdge - Rust)
- **Bridge Proxy**: Upgradeable proxy pattern
- **Bridge Logic**: Main bridge implementation
- **Bridge Governance**: Cross-chain governance operations

```
┌─────────────────────────────────────────────────────────────┐
│                    Bridge Architecture                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌──────────────┐                      ┌──────────────┐     │
│  │   Solana     │                      │    Zera      │     │
│  │   Network    │◄────Guardians────►   │   Network    │     │
│  └──────────────┘                      └──────────────┘     │
│         │                                      │            │
│    ┌────▼────┐                          ┌────▼────┐         │
│    │  Core   │                          │  Proxy  │         │
│    │ Bridge  │                          │ Contract│         │
│    └────┬────┘                          └────┬────┘         │
│         │                                    │              │
│    ┌────▼────┐                          ┌────▼────┐         │
│    │  Token  │                          │ Bridge  │         │
│    │ Bridge  │                          │  Logic  │         │
│    └─────────┘                          └────┬────┘         │
│                                              │              │
│                                         ┌────▼────┐         │
│                                         │ Bridge  │         │
│                                         │   Gov   │         │
│                                         └─────────┘         │
└─────────────────────────────────────────────────────────────┘
```

---

## Solana Bridge Contracts

### 1. zera_bridge_core (Core Bridge Program)

**Location**: `solana_bridge/zera_bridge_core/src/lib.rs`

**Program ID**: `zera3giq7oM9QJaD6mY1ajGmakv9TZcax5Giky99HD8`

**Purpose**: Core governance and security layer for the Solana side of the bridge.

**Key Functions**:

#### Initialization
- `initialize()`: Sets up initial guardians, thresholds, rate limits, and pause state
  - Default guardians: 3 keys with 2-of-3 threshold
  - Rate limit: $10M USD equivalent per 24 hours
  - Single transaction limit: $1M USD equivalent

#### Guardian Management
- `set_guardians_with_sigs()`: Updates guardian set with multi-sig authorization
- Guardian threshold: Minimum signatures required for operations (default: 2)
- Replay protection: Used signatures tracked on-chain

#### Bridge Upgrades
- `upgrade_token_bridge()`: Upgrades the token bridge program
- `upgrade_self()`: Upgrades the core bridge program itself
- Uses Solana BPF Loader Upgradeable mechanism
- Requires guardian signatures and governance PDA authority

#### Pause Controls
- `pause_incoming()`: Pauses incoming transfers (Solana → Zera)
  - Pause level: 1 (IncomingOnly)
- `pause_complete()`: Pauses all bridge operations
  - Pause level: 2 (Complete)
- `unpause()`: Resumes normal operations
  - Pause level: 0 (Active)
- Supports timed pauses with auto-expiry

#### Rate Limit Management
- `update_rate_limit()`: Updates 24-hour rate limit (in USD cents)
- `update_single_tx_limit()`: Updates per-transaction limit (in USD cents)
- Tracks net flow across 24 hourly buckets

#### Transfer Verification
- `post_verified_transfer()`: Verifies and marks guardian-approved transfers
- `post_verified_admin_action()`: Verifies admin actions (e.g., rate limit reset)

**State Storage**:

```rust
RouterConfig {
    guardians: Vec<Pubkey>,           // Guardian public keys
    guardian_threshold: u8,            // Required signatures
    version: u32,                      // Config version
    pause_level: u8,                   // 0=Active, 1=Incoming, 2=Complete
    pause_expiry: i64,                 // Unix timestamp, 0=indefinite
    rate_limit_usd: u64,              // 24h limit in cents
    single_tx_limit_usd: u64,         // Per-tx limit in cents
}
```

**Security Features**:
- Ed25519 signature verification
- Replay protection via used marker PDAs
- Multi-signature threshold enforcement
- Hash-based verification: `version + domain + action + timestamp + expiry + txn_hash + event_index + target_program + payload`

**Authorization**:
- Governance PDA controls all upgrades
- Guardian signatures required for configuration changes
- Initialized guardians: 3 keys with 2-of-3 multi-sig

---

### 2. zera_bridge_token (Token Bridge Program)

**Location**: `solana_bridge/zera_bridge_token/src/lib.rs`

**Program ID**: `WrapZ8f88HR8waSp7wR8Vgc68z4hKj3p3i2b81oeSxR`

**Purpose**: Handles all token operations for the Solana side of the bridge.

**Key Functions**:

#### Initialization
- `initialize_rate_limit_state()`: Sets up 24-hour rolling rate limit tracking
- `initialize_token_price_registry()`: Creates guardian-attested price registry

#### Outgoing Operations (Solana → Zera)

**`lock_sol()`**: Lock native SOL for transfer to Zera
- Transfers SOL to vault PDA
- Checks pause level (blocks if level ≥ 2)
- Checks rate limits with single tx limit
- Emits event for guardian indexing
- Fee: Network determined

**`lock_spl()`**: Lock SPL tokens for transfer to Zera
- Transfers tokens to vault ATA (owned by router_signer PDA)
- Validates mint is not bridge-wrapped (prevents wrapped tokens from being locked)
- Checks pause level (blocks if level ≥ 2)
- Checks rate limits with single tx limit
- Creates vault ATA if needed
- Emits event for guardian indexing

**`burn_wrapped()`**: Burn wrapped Zera tokens to send back to Zera
- Burns wrapped tokens from user's ATA
- Validates token is bridge-wrapped via bridge_info PDA
- Checks pause level (blocks if level ≥ 2)
- Checks rate limits with single tx limit
- Emits event with original Zera contract ID
- Requires valid bridge_info PDA

#### Incoming Operations (Zera → Solana)

**`release_sol()`**: Release native SOL from vault to recipient
- Requires guardian-verified VAA from core bridge
- Checks pause level (blocks if level ≥ 1)
- Updates token price registry from guardian-attested price
- Tracks rate limits (no single tx limit for incoming)
- Transfers SOL from vault PDA to recipient
- Creates redeemed marker to prevent replay

**`release_spl()`**: Release SPL tokens from vault to recipient
- Requires guardian-verified VAA from core bridge
- Checks pause level (blocks if level ≥ 1)
- Updates token price registry from guardian-attested price
- Tracks rate limits (no single tx limit for incoming)
- Transfers tokens from vault ATA to recipient ATA
- Creates recipient ATA if needed
- Creates redeemed marker to prevent replay

**`mint_wrapped()`**: Mint wrapped Zera tokens on Solana
- Requires guardian-verified VAA from core bridge
- Checks pause level (blocks if level ≥ 1)
- Updates token price registry from guardian-attested price
- Tracks rate limits (no single tx limit for incoming)
- **First Mint**: Creates mint, metadata, and bridge_info PDA
  - Requires full metadata (name, symbol, decimals, uri)
  - Uses Metaplex Token Metadata Program
  - Mint authority: PDA `[b"mint_authority", mint]`
  - Wrapped symbol: `w{original_symbol}`
  - Wrapped name: `Wrapped {original_name}`
- **Subsequent Mints**: Just mints more tokens
  - Validates existing bridge_info matches
  - No metadata required
- Creates redeemed marker to prevent replay

#### Admin Functions
- `execute_reset_rate_limit()`: Resets all rate limit buckets (requires admin action marker from core)

**State Storage**:

```rust
BridgeTokenInfo {
    mint: Pubkey,                // Wrapped mint address
    zera_contract_id: Vec<u8>,   // Original Zera contract ID
    source_chain: String,         // "Zera"
    first_minted_at: i64,        // Creation timestamp
    decimals: u8,                 // Token decimals
}

RateLimitState {
    current_hour: u64,              // Current hour (Unix / 3600)
    hourly_buckets: [i64; 24],      // Net flow per hour in USD cents
    current_bucket_index: u8,       // Current bucket (0-23)
}

TokenPriceRegistry {
    entries: Vec<TokenPriceEntry>,  // Max 50 entries
}

TokenPriceEntry {
    mint: Pubkey,                   // Token mint (System::id() for SOL)
    usd_price_cents: u64,          // Guardian-attested price
    last_updated: i64,              // Update timestamp
}
```

**PDA Seeds**:
- Router Signer: `[b"router_signer"]`
- SOL Vault: `[b"vault"]`
- SPL Vault ATA: Associated Token Address for `(router_signer, mint)`
- Wrapped Mint: `[b"mint", hash(zera_contract_id)]`
- Mint Authority: `[b"mint_authority", mint]`
- Bridge Info: `[b"bridge_info", mint]`
- Verified Marker: `[b"verified_transfer", expected_hash]` (in core program)
- Released Marker: `[b"released_transfer", expected_hash]`
- Admin Action Marker: `[b"verified_admin", nonce]` (in core program)

**Rate Limiting**:
- 24-hour rolling window with hourly buckets
- Tracks net flow: `(incoming - outgoing)` in USD cents
- Separate limits for:
  - 24-hour total: $10M USD equivalent (default)
  - Single transaction: $1M USD equivalent (default, outgoing only)
- Uses guardian-attested prices from VAAs
- Automatically rotates buckets as time progresses

**Pause Levels**:
- Level 0 (Active): All operations allowed
- Level 1 (IncomingOnly): Blocks outgoing (lock_sol, lock_spl, burn_wrapped)
- Level 2 (Complete): Blocks all operations

**Security Features**:
- Guardian-verified VAAs required for all releases/mints
- Replay protection via used/redeemed marker PDAs
- Rate limits based on guardian-attested USD prices
- Pause functionality for emergency stops
- Bridge-wrapped tokens tracked via bridge_info PDA

---

## Zera Network Bridge Contracts

### 1. bridge_proxy (Bridge Proxy Contract)

**Location**: `zera_bridge/bridge_proxy/src/lib.rs`

**Wallet Address**: `9fTYjLqHDqCmb1U71a6kRXEYNMwNvTF9xYX48HG4d1WA`

**Purpose**: Upgradeable proxy for the Zera side bridge implementation.

**Key Functions**:

- `init()`: Initializes proxy with bridge_v1 and bridge_gov_v1
  - Initial fee: 100 USD equivalent in ZRA
  - Update key: `gov_$BRIDGEGUARDIAN+0000`
  - Send all key: `gov_$BRIDGEGUARDIAN+0000`

- `execute(function: String, parameters: String)`: Delegates calls to bridge logic implementation
  - Public entry point for bridge operations
  - Comma-separated parameters
  
- `execute_gov(function: String, parameters: String)`: Delegates calls to governance implementation
  - Governance-only operations

- `update(smart_contract: String, instance: String)`: Upgrades bridge logic implementation
  - Restricted to update_key

- `update_gov(smart_contract: String, instance: String)`: Upgrades governance implementation
  - Restricted to update_key

- `update_update_key(update_key: String)`: Changes governance key
  - Restricted to current update_key

- `update_send_all_key(send_all_key: String)`: Changes fund control key
  - Restricted to current send_all_key

- `send_all(wallet: String)`: Transfers all funds to specified wallet
  - Restricted to send_all_key

**State Storage**:
```rust
SmartContractState {
    smart_contract: String,      // "bridge_v1"
    instance: String,            // "1"
    sc_gov: String,             // "bridge_gov_v1"
    sc_gov_instance: String,    // "1"
}

GovKeys {
    update_key: String,         // Governance key for upgrades
    send_all_key: String,       // Key for emergency fund withdrawal
}
```

**Initialization Fee**: 100 USD equivalent (converted to ZRA and held on initialization)

**Authorization**:
- Governance: `gov_$BRIDGEGUARDIAN+0000`
- Implementation: bridge_v1, instance 1
- Governance Implementation: bridge_gov_v1, instance 1

---

### 2. bridge_logic (bridge_v1 Implementation)

**Location**: `zera_bridge/bridge_logic/src/lib.rs`

**Purpose**: Main bridge logic for Zera network operations.

**Key Functions**:

#### Initialization
- `init()`: Sets up guardians, rate limits, and pause configuration
  - Initial fee: 10 USD equivalent in ZRA
  - Guardians: 3 keys with 2-of-3 threshold
  - Rate limit: $10M USD equivalent per 24 hours
  - Single transaction limit: $1M USD equivalent

#### User-Facing Functions (Outgoing: Zera → Solana)

**`lock_zera()`**: Send native Zera tokens to Solana
- Locks tokens in bridge contract
- Fee: 0.5 USD equivalent in ZRA
- Validates contract ID format (must end with `+0000`)
- Validates denomination is power of 10
- Checks pause level (blocks if level ≥ 2)
- Checks rate limits with single tx limit
- Emits event: `EVENT:SEND_NATIVE_ZERA_TO_SOLANA`

**`burn_sol()`**: Send wrapped Solana tokens back to Solana
- Burns wrapped tokens (sends to `:fire:` wallet)
- Validates contract ID format (must start with `$sol-` and end with `+000000`)
- Validates mint ID exists
- Checks pause level (blocks if level ≥ 2)
- Checks rate limits with single tx limit
- Emits event: `EVENT:SEND_WRAPPED_SOLANA_TO_SOLANA`

#### Guardian-Verified Functions (Incoming: Solana → Zera)

**`mint_sol()`**: Mint existing wrapped Solana tokens on Zera
- Requires guardian signatures
- Validates transaction signature hasn't been used
- Validates hash: `sha256(mint_id + amount + wallet_address + tx_signature)`
- Checks pause level (blocks if level ≥ 1)
- Tracks rate limits (no single tx limit for incoming)
- Mints tokens to recipient
- Emits event: `SUCCESS:MINT_NATIVE_SOLANA_TO_ZERA`

**`create_sol()`**: Create new wrapped Solana token on Zera
- Requires guardian signatures
- Validates transaction signature hasn't been used
- Validates hash: `sha256(symbol + name + denomination + wallet + amount + mint_id + uri + authorized_key + tx_signature)`
- Checks pause level (blocks if level ≥ 1)
- Creates new contract with format: `$sol-{symbol}+{suffix}`
  - Suffix auto-increments per symbol (6 digits, zero-padded)
- Uses `instrument_contract_bridge()` native function
- Stores mint ID mapping
- Emits event: `SUCCESS: CONTRACT_CREATED`

**`release_zera()`**: Release locked native Zera tokens back to Zera
- Requires guardian signatures
- Validates transaction signature hasn't been used
- Validates hash: `sha256(contract_id + amount + wallet_address + tx_signature)`
- Checks pause level (blocks if level ≥ 1)
- Tracks rate limits (no single tx limit for incoming)
- Releases tokens from contract
- Emits event: `SUCCESS:RELEASE_NATIVE_ZERA`

#### Configuration Functions (Governance Only)

**`update_pause_config()`**: Updates pause level and expiry
- Pause levels:
  - 0: Unpause (all operations)
  - 1: IncomingOnly (blocks outgoing: lock_zera, burn_sol)
  - 2: Complete (blocks all operations)
- Duration in seconds (0 = indefinite)
- Auto-unpause when expiry time reached

**`update_guardian_state()`**: Updates guardian keys and threshold
- Pipe-separated guardian list
- Threshold: minimum signatures required

**`reset_rate_limit()`**: Resets all rate limit buckets to zero
- Emergency function for rate limit issues

**`update_rate_limit()`**: Updates rate limit parameters
- `single_limit`: Per-transaction limit in USD cents
- `rate_limit`: 24-hour total limit in USD cents

**State Storage**:

```rust
GuardianState {
    guardians: Vec<String>,      // Guardian public keys
    guardian_threshold: u8,       // Required signatures
}

BucketLimit {
    current_hour: u64,                      // Current hour (Unix / 3600)
    hourly_buckets_incoming: [u64; 24],    // Incoming flow per hour
    hourly_buckets_outgoing: [u64; 24],    // Outgoing flow per hour
    current_bucket_index: u8,               // Current bucket (0-23)
    single_limit: u64,                      // Per-tx limit in USD cents
    rate_limit: u64,                        // 24h limit in USD cents
}

PauseConfig {
    pause_level: u8,       // 0=Active, 1=IncomingOnly, 2=Complete
    pause_expiry: u64,     // Unix timestamp, 0=indefinite
}

SymbolConfig {
    suffix_count: u64,     // Counter for wrapped token suffix
}
```

**Guardian Verification**:
- Multi-signature verification with threshold
- Replay protection: signatures marked as used after verification
- Pipe-separated parallel arrays for signatures and guardian keys
- Hash verification: `sha256(payload)` must match `signed_hash`

**Rate Limiting**:
- Separate tracking for incoming and outgoing flows
- Net flow calculation: `|incoming - outgoing|` must not exceed rate_limit
- 24-hour rolling window with hourly buckets
- Uses ACE (exchange rate) data to convert amounts to USD cents
- Formula: `(amount * ace_rate / denomination) / 10^16` = USD cents

**Security Features**:
- Authorization check: only callable from proxy wallet
- Guardian signature verification with threshold
- Transaction signature replay protection
- Hash verification for all guardian-signed operations
- Rate limiting based on USD value
- Pause functionality for emergency stops

---

### 3. bridge_gov (bridge_gov_v1 Governance)

**Location**: `zera_bridge/bridge_gov/src/lib.rs`

**Purpose**: Cross-chain governance operations that emit events for Solana guardian execution.

**Key Functions**:

**`pause()`**: Pause bridge operations
- Emits event: `EVENT:PAUSE_SOLANA_BRIDGE`
- Parameters:
  - `pause_level`: "0" (unpause), "1" (incoming only), "2" (complete)
  - `pause_duration`: seconds (0 = indefinite)
- Guardians execute corresponding action on Solana

**`update_token_bridge()`**: Upgrade Solana token bridge program
- Emits event: `EVENT:UPDATE_TOKEN_BRIDGE`
- Parameters:
  - `buffer_account`: Base58 account with new program code
  - `spill_account`: Base58 account for refunded SOL
- Guardians execute upgrade on Solana

**`update_core_bridge()`**: Upgrade Solana core bridge program
- Emits event: `EVENT:UPDATE_CORE_BRIDGE`
- Parameters:
  - `buffer_account`: Base58 account with new program code
  - `spill_account`: Base58 account for refunded SOL
- Guardians execute upgrade on Solana

**`update_guardian_keys()`**: Update guardian set on Solana
- Emits event: `EVENT:UPDATE_GUARDIAN_KEYS`
- Parameters:
  - `guardian_keys`: Pipe-separated list of guardian public keys
  - `threshold`: Minimum signatures required
- Guardians execute on Solana

**`update_single_limit()`**: Update single transaction limit
- Emits event: `EVENT:UPDATE_SINGLE_LIMIT`
- Parameter: `limit` in USD cents
- Guardians execute on both Zera and Solana

**`update_rate_limit()`**: Update 24-hour rate limit
- Emits event: `EVENT:UPDATE_RATE_LIMIT`
- Parameter: `limit` in USD cents
- Guardians execute on both Zera and Solana

**`reset_rate_limit()`**: Reset rate limit state
- Emits event: `EVENT:RESET_RATE_LIMIT`
- Guardians execute on both Zera and Solana

**Authorization**:
- Requires calls from proxy wallet: `9fTYjLqHDqCmb1U71a6kRXEYNMwNvTF9xYX48HG4d1WA`
- Requires governance key: `gov_$BRIDGEGUARDIAN+0000`
- All functions are governance-gated

**Event-Driven Architecture**:
- Zera governance emits events
- Off-chain guardians monitor events
- Guardians sign and execute corresponding Solana transactions
- Enables cross-chain governance coordination

---

## Bridge Flow Examples

### Sending Native Zera Token to Solana

1. **User calls** `lock_zera()` on Zera bridge proxy
   - Locks ZRA tokens in bridge
   - Emits `EVENT:SEND_NATIVE_ZERA_TO_SOLANA`
   
2. **Guardians monitor** Zera events
   - Verify transaction and amount
   - Create VAA (Verifiable Action Approval)
   - Sign with 2-of-3 threshold
   
3. **User/Guardian submits** to Solana `release_spl()`
   - Provides guardian signatures
   - Core bridge verifies signatures
   - Token bridge releases tokens from vault

### Sending Native SOL to Zera

1. **User calls** `lock_sol()` on Solana token bridge
   - Locks SOL in vault PDA
   - Emits `Lock_SOL` event
   
2. **Guardians monitor** Solana events
   - Verify transaction and amount
   - Create signed payload
   - Submit to Zera with signatures
   
3. **User/Guardian calls** `release_zera()` on Zera bridge proxy
   - Provides guardian signatures
   - Bridge verifies signatures and hash
   - Releases locked tokens to recipient

### Creating Wrapped Solana Token on Zera (First Time)

1. **User calls** `lock_spl()` on Solana (first time for a token)
   - Locks tokens in vault ATA
   - Emits `Lock_SPL` event with mint info
   
2. **Guardians monitor** and verify
   - Fetch token metadata from Solana
   - Create signed payload with metadata
   - Submit to Zera
   
3. **Guardian calls** `create_sol()` on Zera bridge proxy
   - Provides metadata (symbol, name, decimals, uri)
   - Bridge verifies guardian signatures
   - Creates new wrapped token contract: `$sol-{symbol}+{suffix}`
   - Mints initial amount to recipient

### Governance: Pausing the Bridge

1. **Governance calls** `pause()` on Zera bridge governance
   - Emits `EVENT:PAUSE_SOLANA_BRIDGE`
   - Parameters: level and duration
   
2. **Guardians monitor** governance events
   - Create VAA for pause action
   - Sign with 2-of-3 threshold
   
3. **Guardians submit** to Solana core bridge
   - Calls `pause_incoming()` or `pause_complete()`
   - Updates pause state on Solana
   - Both chains now paused

---

## Security Model

### Guardian System

**Purpose**: Decentralized verification of cross-chain operations

**Guardians** (Default):
1. `C68BgMJks69fsn5yr4cKNnYuw9yztW3vBNyk4hCyr3iE` (Solana)
   - `A_c_C68BgMJks69fsn5yr4cKNnYuw9yztW3vBNyk4hCyr3iE` (Zera)
2. `B1NgczXgVbJjJLUdbHkQ5xe6fxnzvzQk7MP7o6JqK3dp` (Solana)
   - `A_c_B1NgczXgVbJjJLUdbHkQ5xe6fxnzvzQk7MP7o6JqK3dp` (Zera)
3. `9aZ6ZymbUETdA9neSnLjvjj9iD8SqHfKo8L9QFtv1PGJ` (Solana)
   - `A_c_9aZ6ZymbUETdA9neSnLjvjj9iD8SqHfKo8L9QFtv1PGJ` (Zera)

**Threshold**: 2-of-3 signatures required

**Signature Verification**:
- Ed25519 signatures on Solana
- Native signature verification on Zera
- Hash-based payload verification on both chains

### Rate Limiting

**Purpose**: Prevent large-scale exploits and ensure bridge liquidity

**Limits** (Default):
- 24-hour total: $10M USD equivalent
- Single transaction: $1M USD equivalent (outgoing only)

**Mechanism**:
- 24 hourly buckets in rolling window
- Tracks net flow: `|incoming - outgoing|`
- Uses guardian-attested prices for USD conversion
- Automatically rotates buckets as time progresses

**Override**: Governance can reset rate limits in emergency

### Pause Mechanism

**Purpose**: Emergency stop for security incidents

**Pause Levels**:
- **Level 0 (Active)**: All operations allowed
- **Level 1 (IncomingOnly)**: Blocks incoming transfers to that chain
  - Zera: Blocks mint_sol, create_sol, release_zera (Solana → Zera)
  - Solana: Blocks release_sol, release_spl, mint_wrapped (Zera → Solana)
- **Level 2 (Complete)**: Blocks all transfers
  - Both incoming and outgoing operations paused on that chain

⚠️ **IMPORTANT: Pause States Are Chain-Independent**

The pause state on each chain is **completely independent** and **not synchronized**:

- Setting a pause on Solana does **NOT** automatically pause the Zera side
- Setting a pause on Zera does **NOT** automatically pause the Solana side
- Each chain maintains its own `pause_level` and `pause_expiry` values
- Cross-chain pause coordination requires separate governance actions on each chain

**Why This Matters**:
- If Zera is paused at Level 1, incoming transfers to Zera (from Solana) are blocked
- However, users can still initiate transfers FROM Zera TO Solana (outgoing from Zera)
- These transfers will succeed on the Zera side but may fail on the Solana side if Solana is also paused
- For a complete bridge halt, **both chains must be paused independently**

**Coordinated Pause Flow**:
1. Governance calls `pause()` on Zera bridge governance
2. This emits `EVENT:PAUSE_SOLANA_BRIDGE` for guardians
3. Guardians must separately execute the pause on Solana
4. Until both chains are paused, transfers may partially succeed

**Timed Pauses**:
- Can set expiry timestamp
- Automatic unpause when time reached
- 0 = indefinite pause

**Authorization**: Governance only

### Replay Protection

**Zera Side**:
- Transaction signatures stored after use
- Key: `TX_SIGNATURE_{tx_signature}`
- Prevents same transaction from being executed twice

**Solana Side**:
- Used marker PDAs created after verification
- Seed: `[b"verified_transfer", expected_hash]`
- Redeemed marker PDAs created after execution
- Seed: `[b"released_transfer", expected_hash]`

### Hash Verification

**Zera → Solana**:
```
hash = sha256(
    mint_id + 
    amount + 
    wallet_address + 
    tx_signature
)
```

**Solana → Zera**:
```
hash = sha256(
    version +
    domain +
    action +
    timestamp +
    expiry +
    txn_hash +
    event_index +
    target_program +
    payload
)
```

**Verification**:
- Hash must match guardian-signed hash
- Prevents payload tampering
- Ensures integrity of cross-chain messages

---

## Token Naming Conventions

### Native Zera Tokens on Solana

**Format**: Native tokens keep their original symbol and name
- Example: `$ZRA+0000` → wrapped as `wZRA`
- Wrapped prefix: `w{original_symbol}`
- Wrapped name: `Wrapped {original_name}`

### Native Solana Tokens on Zera

**Format**: `$sol-{symbol}+{suffix}`
- Prefix: `$sol-` (identifies as Solana-origin)
- Symbol: Original SPL token symbol
- Suffix: 6-digit zero-padded counter (e.g., `+000000`, `+000001`)

**Examples**:
- First USDC: `$sol-USDC+000000`
- Second USDC (if different mint): `$sol-USDC+000001`
- SOL: `$sol-SOL+000000`

**Suffix Handling**:
- Per-symbol counter stored in `SymbolConfig`
- Prevents name collisions for same symbol
- Allows multiple mints with same symbol name

---

## Governance

**Governance Key**: `gov_$BRIDGEGUARDIAN+0000`

**Governance Capabilities**:

### Zera Network
- Upgrade bridge logic implementation
- Upgrade bridge governance implementation
- Update guardian set and threshold
- Pause/unpause bridge operations
- Update rate limits (single tx and 24-hour)
- Reset rate limit state
- Emergency fund withdrawal

### Solana Network (via Guardian Execution)
- Upgrade token bridge program
- Upgrade core bridge program
- Update guardian set and threshold
- Pause/unpause bridge operations
- Update rate limits
- Reset rate limit state

**Cross-Chain Coordination**:
1. Governance action initiated on Zera
2. Event emitted from bridge_gov
3. Guardians monitor and verify
4. Guardians create and sign Solana transaction
5. Transaction executed on Solana with guardian signatures

---

## Contract Addresses & Identifiers

### Solana
- **Core Bridge Program**: `zera3giq7oM9QJaD6mY1ajGmakv9TZcax5Giky99HD8`
- **Token Bridge Program**: `WrapZ8f88HR8waSp7wR8Vgc68z4hKj3p3i2b81oeSxR`

### Zera Network
- **Bridge Proxy Wallet**: `9fTYjLqHDqCmb1U71a6kRXEYNMwNvTF9xYX48HG4d1WA`
- **Bridge Logic**: `bridge_v1` (instance 1)
- **Bridge Governance**: `bridge_gov_v1` (instance 1)
- **Governance Key**: `gov_$BRIDGEGUARDIAN+0000`
- **Base Token**: `$ZRA+0000`
- **Burn Wallet**: `:fire:`

---

## Initialization Fees

All fees are denominated in **USD equivalent** amounts that are converted to the appropriate native token at initialization:

### Solana Side
- **Core Bridge**: 10 USD equivalent (converted to SOL)
- **Token Bridge**: N/A (initialized by core)

### Zera Network Side
- **Bridge Proxy**: 100 USD equivalent (converted to ZRA)
- **Bridge Logic**: 10 USD equivalent (converted to ZRA)
- **Bridge Governance**: N/A (no initialization fee)

### User Operation Fees
- **lock_zera()**: 0.5 USD equivalent (converted to ZRA per transaction)
- **Other operations**: Network-determined transaction fees

---

## Technology Stack

### Solana Contracts
- **Language**: Rust
- **Framework**: Anchor 0.30+
- **Runtime**: Solana BPF
- **Dependencies**:
  - anchor-lang
  - anchor-spl (Token, AssociatedToken)
  - mpl-token-metadata (Metaplex)
  - solana-program (Ed25519, sysvar)

### Zera Network Contracts
- **Language**: Rust
- **Runtime**: WasmEdge
- **Framework**: Custom Zera SDK
- **Dependencies**:
  - native_functions (Zera blockchain bindings)
  - serde (serialization)
  - base64 (state encoding)
  - postcard (binary serialization)

---

## Important Notes

⚠️ **This repository is for transparency and reference only**
- Contracts are NOT meant to be compiled from this repository
- These are the deployed contracts on Zera and Solana networks
- Provided for audit, review, and educational purposes

⚠️ **Security Considerations**
- Bridge uses guardian-based security model
- 2-of-3 multi-signature threshold required for transfers
- Rate limits and pause mechanisms provide additional safety
- All cross-chain operations require guardian verification

⚠️ **Bridge Status**
- Monitor pause status before initiating transfers
- Check rate limit availability for large transfers
- Guardian response time may vary based on network conditions

⚠️ **Token Wrapping**
- Wrapped tokens are NOT directly redeemable for underlying assets without guardian approval
- First-time wrapping of new tokens requires metadata initialization
- Bridge-wrapped tokens cannot be re-locked (prevents circular wrapping)

---

## Related Contracts

- [ACE (Exchange Rate)](../ace/README.md) - Provides USD price data for rate limiting
- [Treasury](../treasury/README.md) - Potential destination for bridge fees
- [Network Fees](../network_fees/README.md) - Network transaction fee parameters

---

## Audit & Security

**Guardian Model**: Multi-signature verification provides decentralized security
**Rate Limits**: Prevent large-scale exploits and ensure liquidity
**Pause Mechanism**: Emergency stop capability for security incidents
**Replay Protection**: Prevents double-spending across chains
**Hash Verification**: Ensures payload integrity
