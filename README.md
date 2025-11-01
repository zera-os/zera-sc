# Zera Smart Contracts

This repository contains the smart contracts deployed on the Zera Network. These contracts are provided for **transparency and reference purposes only** - this repository is not intended to be built or compiled.

## Overview

The Zera Network utilizes a sophisticated smart contract architecture built in Rust using the WasmEdge runtime. The contracts follow a proxy pattern for upgradeability and are organized into several functional categories.

## Architecture

### Proxy Pattern
Most contracts follow a proxy pattern architecture:
- **Proxy Contracts**: Entry points that delegate calls to implementation contracts
- **Implementation Contracts**: Versioned logic contracts (v1, v2, etc.)
- **Upgradeability**: Governance can update which implementation version the proxy uses

### Governance
Contracts are controlled by governance keys:
- `update_key`: Can upgrade contract implementations
- `send_all_key`: Can transfer all funds from contract
- Typically set to `gov_$ZRA+0000` or similar governance addresses

## Smart Contract Categories

### 1. ACE (Automated Currency Exchange)
Located in `smart_contracts/ace/`
- **Purpose**: Manages exchange rate data for the $ZRA token
- **Contracts**:
  - `zra_v1`: Stores and updates ZRA exchange rates
  - `ace_proxy`: Upgradeable proxy for ACE functionality

### 2. Circulating Supply Management
Located in `smart_contracts/circulating_supply/`
- **Purpose**: Manages whitelist of addresses excluded from circulating supply calculations
- **Contracts**:
  - `circulating_whitelist_v1`: Whitelist management implementation
  - `circulating_supply_proxy`: Upgradeable proxy

### 3. Native Functions
Located in `smart_contracts/native_functions/`
- **Purpose**: Core library providing native blockchain functions
- **Functionality**: 
  - State management
  - Token operations (send, mint, transfer)
  - Smart contract calls
  - Cryptographic functions
  - Balance and supply queries

### 4. Network Fees
Located in `smart_contracts/network_fees/`
- **Purpose**: Manages network transaction fees and validator settings
- **Contracts**:
  - `network_fees`: Stores and updates all network fee parameters
  - `network_fee_proxy`: Upgradeable proxy

### 5. Restricted Symbols
Located in `smart_contracts/restricted_symbols/`
- **Purpose**: Manages restricted token symbols that cannot be used by regular users
- **Contracts**:
  - `restricted_symbols_v1`: Symbol restriction implementation
  - `restricted_symbols_proxy`: Upgradeable proxy

### 6. Staking System
Located in `smart_contracts/staking/`
- **Purpose**: Comprehensive staking system with multiple lock periods and reward mechanisms
- **Contracts**:
  - `early_backers` (release_v1): Manages early backer token releases over time
  - `early_proxy` (release_proxy): Proxy for early backer releases
  - `normal_stake` (staking_v1): Main staking with 6-month to 5-year lock periods
  - `normal_proxy` (staking_proxy): Proxy for staking operations
  - `principle`: Manages staked principal amounts
  - `principle_proxy`: Proxy for principle management

### 7. Treasury
Located in `smart_contracts/treasury/`
- **Purpose**: Central treasury for fund management
- **Contracts**:
  - `treasury`: Treasury implementation with governance-controlled fund transfers
  - `treasury_proxy`: Upgradeable proxy

## Key Features

### Security
- Authorization checks on all privileged operations
- Exploit detection mechanisms in reward distribution
- Safe arithmetic to prevent overflows
- Governance key management for upgrades

### Upgradeability
- Proxy pattern allows logic upgrades without data migration
- Version tracking for all implementations
- Governance-controlled upgrade process

### State Management
- Efficient state serialization using postcard and base64
- Structured state storage with typed keys
- Clear state separation between contracts

### Token Economics
- Dynamic reward calculations based on lock periods
- Supply tracking and circulating supply management
- Fee distribution to validators and treasury
- Multi-signature governance controls

## Network Integration

These contracts integrate with the Zera Network's native functions:
- **Consensus**: Validator registration and heartbeats
- **Governance**: On-chain voting and proposals
- **Compliance**: KYC/AML integration
- **Allowances**: Spending limits and authorizations

## Important Notes

⚠️ **This repository is for reference only**
- Not designed to be built or compiled
- Shows deployed contract code for transparency
- Actual deployment is managed by Zera Network governance

⚠️ **Security Considerations**
- All contracts include exploit detection
- Hardcoded wallet addresses are for production deployment
- Governance keys control critical operations

## Contract Addresses

Proxy wallet addresses are hardcoded in each contract for security:
- ACE Proxy: `6cifeAScHLGvxARJSdxS6QPdTJLwqgXMmHzXWyFU9tHC`
- Circulating Supply Proxy: `EMK16opdneub97v9qC4NdSkMTsnvHsRf4LrdZ2KH3cky`
- Network Fee Proxy: `5o5AkKgjtcqsTVbxNHCRHvUCfL9nJ7F48CqfayJyuRu`
- Restricted Symbols Proxy: `H7YTw7bry3VQVmAADF3tQw4eGYUMuenac3rbNb7r5SZA`
- Staking Proxy: `AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8`
- Early Backers Proxy: `AZfFcttA3nwqmEYzAtsmufops7PaxLYavvkkDRsxTX5j`
- Principle Proxy: `8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko`
- Treasury: `4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH`

## Technology Stack

- **Language**: Rust
- **Runtime**: WasmEdge
- **Serialization**: Postcard + Base64
- **Cryptography**: Native Zera functions (SHA256, SHA512, Blake3, SHAKE)
- **Token Standard**: Zera native tokens

## Documentation Structure

Each smart contract directory contains its own README.md with:
- Contract purpose and functionality
- Key functions and their parameters
- State management details
- Security considerations
- Integration points

## License

See individual contract licenses for details.

## Contact

For questions about these smart contracts, please refer to the official Zera Network documentation.

