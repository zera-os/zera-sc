pub mod bridge_v2 {
    use base64::{decode, encode};
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::is_valid_u256;
    use native_functions::zera::types::string_to_u256;
    use native_functions::zera::types::U256;
    use native_functions::zera::wasmedge_bindgen;
    use postcard::{from_bytes, to_allocvec};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    //TODO add real gov
    const GOV_CONTRACT: &str = "gov_$BRIDGEGUARDIAN+0000";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const CONTRACT_EXIST_KEY: &str = "CONTRACT_EXIST";
    const SUFFIX_KEY: &str = "SUFFIX";
    const MINT_ID_KEY: &str = "MINT_ID";
    const SOLANA_SUFFIX: &str = "+000000";
    const SOLANA_PREFIX: &str = "$sol-";
    const BURN_WALLET: &str = ":fire:";
    const PROXY_WALLET: &str = "66Eb7Yo5S2Qz8wbfHz8q9UhUKST9LNAGNxsa2zokB6U8"; //sc_zera_bridge_proxy_1
    const SYMBOL_CONFIG_KEY: &str = "SYMBOL_CONFIG";
    const BRIDGE_PROXY_CONTRACT: &str = "zera_bridge_proxy_1";
    const ONE_USD: &str = "1000000000";
    const TOKEN_PRICE_KEY: &str = "TOKEN_PRICE";

    // Guardian management constants
    const GUARDIAN_STATE_KEY: &str = "GUARDIAN_STATE"; //key for guardian state
    const USED_SIGNATURES_KEY: &str = "USED_SIG_"; //key for used signatures
    const TX_SIGNATURE_KEY: &str = "TX_SIGNATURE"; //key for nonce tracking
    const RATE_LIMIT_KEY: &str = "RATE_LIMIT_"; //key for rate limit
    const PAUSE_CONFIG_KEY: &str = "PAUSE_CONFIG"; //key for pause config

    // Validates that a string is a valid Solana base58 address
    // Solana addresses are 32-byte public keys encoded in base58
    fn is_valid_solana_address(address: &str) -> bool {
        // Base58 alphabet (Bitcoin/Solana style - excludes 0, O, I, l)
        const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        // Check length (base58 encoded 32 bytes is typically 32-44 characters)
        if address.len() < 32 || address.len() > 44 {
            return false;
        }

        // Check all characters are valid base58
        for c in address.chars() {
            if !BASE58_ALPHABET.contains(c) {
                return false;
            }
        }

        // Additional validation: decode and verify it's exactly 32 bytes
        match decode_base58(address) {
            Some(decoded) => decoded.len() == 32,
            None => false,
        }
    }

    // Decodes a base58 string to bytes
    fn decode_base58(input: &str) -> Option<Vec<u8>> {
        const BASE58_ALPHABET: &[u8] =
            b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

        let mut result: Vec<u8> = vec![0];

        for byte in input.bytes() {
            let mut carry = BASE58_ALPHABET.iter().position(|&x| x == byte)? as u32;

            for result_byte in result.iter_mut() {
                carry += (*result_byte as u32) * 58;
                *result_byte = (carry & 0xFF) as u8;
                carry >>= 8;
            }

            while carry > 0 {
                result.push((carry & 0xFF) as u8);
                carry >>= 8;
            }
        }

        // Add leading zeros
        for byte in input.bytes() {
            if byte == b'1' {
                result.push(0);
            } else {
                break;
            }
        }

        result.reverse();
        Some(result)
    }

    fn get_usd_value(amount_str: String, contract_id: String, token_price: String) -> u64 {
        unsafe {
            let amount = types::string_to_u256(amount_str.clone());
            //1000000000000000000 = 1$ from get_ace_data
            //100 = 1$ in rate_limit
            //divide our usd value by 10000000000000000 to get the rate limit value
            let denomination = smart_contracts::contract_denomination(contract_id.clone());

            let (authorized, rate) = smart_contracts::get_ace_data(contract_id.clone());
            if !authorized || rate == 0 {
                let mut token_price_u256 = types::string_to_u256(token_price.clone());
                if token_price.clone() == "0" {
                    let token_price_state = smart_contracts::retrieve_state(format!(
                        "{}{}",
                        TOKEN_PRICE_KEY.to_string(),
                        contract_id.clone()
                    ));


                    if token_price_state.is_empty() {
                        return 0;
                    }

                    if !types::is_valid_u256(token_price_state.clone()) {
                        return 0;
                    }

                    if token_price_state.clone() == "0" {
                        return 0;
                    }

                    token_price_u256 = types::string_to_u256(token_price_state.clone());
                    
                }

                let divisor = types::string_to_u256(ONE_USD.clone());
                let usd_value = (((amount * token_price_u256) / denomination) / divisor) * 100;
                let v: u64 = if usd_value > U256::from(u64::MAX) {u64::MAX} else {usd_value.low_u64()};

                return v;
            }

        let divisor = types::string_to_u256("10000000000000000".to_string());
        let denomination = smart_contracts::contract_denomination(contract_id.clone());
        let usd_value = ((amount * rate) / denomination) / divisor;

        let v: u64 = if usd_value > U256::from(u64::MAX) {
            u64::MAX
        } else {
            usd_value.low_u64()
        };

        return v;
    }
}
// Check if denomination is a power of 10 (divisible only by 10)
fn is_power_of_10(mut n: U256) -> bool {
    if n == U256::zero() {
        return false;
    }
    while n % U256::from(10) == U256::zero() {
        n = n / 10;
    }
    n == U256::from(1)
}

fn store_tx_signature(tx_signature: String) {
    unsafe {
        smart_contracts::store_state(
            format!("{}{}", TX_SIGNATURE_KEY.to_string(), tx_signature.clone()),
            "1".to_string(),
        );
    }
}

fn check_tx_signature(tx_signature: String) -> Result<(), SimpleErr> {
    unsafe {
        let tx_signature_state = smart_contracts::retrieve_state(format!(
            "{}{}",
            TX_SIGNATURE_KEY.to_string(),
            tx_signature.clone()
        ));
        if !tx_signature_state.is_empty() {
            return Err(SimpleErr::UsedTxSignature);
        }
        return Ok(());
    }
}

fn check_auth() -> Result<(), SimpleErr> {
    unsafe {
        let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
        let sc_wallet = sc_wallet_.clone();

        if sc_wallet != PROXY_WALLET.to_string() {
            return Err(SimpleErr::UnauthorizedSender);
        }

        return Ok(());
    }
}

fn check_gov_auth() -> Result<(), SimpleErr> {
    unsafe {
        let pub_key_ = smart_contracts::public_key();
        let pub_key = pub_key_.clone();
        if (pub_key != GOV_CONTRACT.to_string()) {
            return Err(SimpleErr::UnauthorizedSender);
        }

        return Ok(());
    }
}

/// Validates that a guardian key has the correct Zera prefix format
/// Valid format: ONE uppercase letter + underscore, followed by zero or more (lowercase + underscore), then base58 key
/// Examples: "A_5vMGD...", "A_c_5vMGD...", "B_c_a_d_b_5vMGD..."
/// Invalid: "AB_...", "a_...", "A_C_...", "A5vMGD..." (no underscore)
fn validate_zera_guardian_prefix(key: &str) -> bool {
    let chars: Vec<char> = key.chars().collect();

    if chars.len() < 3 {
        return false; // Too short to have prefix + key
    }

    // First character must be uppercase letter
    if !chars[0].is_ascii_uppercase() {
        return false;
    }

    // Second character must be underscore
    if chars[1] != '_' {
        return false;
    }

    let mut i = 2;

    // After the first uppercase + underscore, we can have zero or more (lowercase + underscore)
    while i < chars.len() {
        if chars[i].is_ascii_lowercase() {
            // Must be followed by underscore
            if i + 1 >= chars.len() || chars[i + 1] != '_' {
                return false;
            }
            i += 2; // Skip lowercase + underscore
        } else if chars[i].is_ascii_uppercase() {
            // No more uppercase letters allowed after the first one
            return false;
        } else {
            // Found start of base58 key (number or lowercase without following underscore)
            break;
        }
    }

    // Make sure we have remaining characters for the actual key
    if i >= chars.len() {
        return false;
    }

    true
}

/// Strips the prefix from a guardian key
/// Prefixes are in format: uppercase letters followed by underscores (e.g., "A_b_", "B_c_a_", "A_c_")
/// Returns the base58 Solana key without the prefix
fn strip_guardian_prefix(key: &str) -> String {
    // Find the last underscore that's part of the prefix pattern
    // The actual key starts after the last underscore in the prefix
    let mut last_underscore_idx = None;

    for (i, c) in key.chars().enumerate() {
        if c == '_' {
            last_underscore_idx = Some(i);
        } else if c.is_lowercase() || c.is_numeric() {
            // Found start of base58 key (base58 uses lowercase and numbers)
            break;
        }
    }

    // If we found underscores, return the part after the last one
    if let Some(idx) = last_underscore_idx {
        if idx + 1 < key.len() {
            return key[idx + 1..].to_string();
        }
    }

    // No prefix found, return the original key
    key.to_string()
}

fn check_and_update_rate_limit(
    is_outgoing: bool,
    amount_str: String,
    current_time: u64,
    contract_id: String,
    token_price: String,
) -> Result<(), SimpleErr> {
    unsafe {
        let current_hour = current_time / 3600 as u64;
        let amount_usd =
            get_usd_value(amount_str.clone(), contract_id.clone(), token_price.clone());
        let mut rate_limit_state: BucketLimit = load_state(RATE_LIMIT_KEY)?;

        if is_outgoing {
            // amount_usd is already u64 (unsigned), no need for abs()
            if amount_usd > rate_limit_state.single_limit {
                return Err(SimpleErr::SingleTxLimitExceeded);
            }
        }

        // Rotate buckets if we've moved to a new hour
        if current_hour != rate_limit_state.current_hour {
            let hours_elapsed = current_hour.saturating_sub(rate_limit_state.current_hour);

            if hours_elapsed >= 24 {
                // More than 24 hours passed, reset all buckets
                rate_limit_state.hourly_buckets_incoming = [0; 24];
                rate_limit_state.hourly_buckets_outgoing = [0; 24];
                rate_limit_state.current_bucket_index = 0;
            } else {
                // Rotate buckets forward
                for _ in 0..hours_elapsed {
                    rate_limit_state.current_bucket_index =
                        (rate_limit_state.current_bucket_index + 1) % 24;
                    rate_limit_state.hourly_buckets_incoming
                        [rate_limit_state.current_bucket_index as usize] = 0;
                    rate_limit_state.hourly_buckets_outgoing
                        [rate_limit_state.current_bucket_index as usize] = 0;
                }
            }

            rate_limit_state.current_hour = current_hour;
        }

        let current_net_flow_incoming: u64 =
            rate_limit_state.hourly_buckets_incoming.iter().sum::<u64>();
        let current_net_flow_outgoing: u64 =
            rate_limit_state.hourly_buckets_outgoing.iter().sum::<u64>();

        let mut flow_delta_incoming: u64 = 0;
        let mut flow_delta_outgoing: u64 = 0;

        if is_outgoing {
            flow_delta_outgoing = amount_usd;
        } else {
            flow_delta_incoming = amount_usd;
        }

        let mut new_net_flow_incoming: u64 = 0;
        let mut new_net_flow_outgoing: u64 = 0;

        new_net_flow_incoming = current_net_flow_incoming
            .checked_add(flow_delta_incoming)
            .ok_or(SimpleErr::ArithmeticOverflow)?;
        new_net_flow_outgoing = current_net_flow_outgoing
            .checked_add(flow_delta_outgoing)
            .ok_or(SimpleErr::ArithmeticOverflow)?;

        let mut new_net_flow: u64 = 0;

        if new_net_flow_incoming > new_net_flow_outgoing {
            new_net_flow = new_net_flow_incoming - new_net_flow_outgoing;
        } else {
            new_net_flow = new_net_flow_outgoing - new_net_flow_incoming;
        }

        if new_net_flow > rate_limit_state.rate_limit {
            smart_contracts::emit(
                format!(
                    "Failed: Rate limit exceeded: new_net_flow={} limit={}",
                    new_net_flow, rate_limit_state.rate_limit
                )
                .to_string(),
            );
            return Err(SimpleErr::RateLimitExceeded);
        }

        // Update current bucket
        let current_bucket_value_incoming = rate_limit_state.hourly_buckets_incoming
            [rate_limit_state.current_bucket_index as usize];
        let current_bucket_value_outgoing = rate_limit_state.hourly_buckets_outgoing
            [rate_limit_state.current_bucket_index as usize];

        let mut new_bucket_value_incoming: u64 = 0;
        let mut new_bucket_value_outgoing: u64 = 0;

        new_bucket_value_incoming = current_bucket_value_incoming
            .checked_add(flow_delta_incoming)
            .ok_or(SimpleErr::ArithmeticOverflow)?;
        new_bucket_value_outgoing = current_bucket_value_outgoing
            .checked_add(flow_delta_outgoing)
            .ok_or(SimpleErr::ArithmeticOverflow)?;

        rate_limit_state.hourly_buckets_incoming[rate_limit_state.current_bucket_index as usize] =
            new_bucket_value_incoming;
        rate_limit_state.hourly_buckets_outgoing[rate_limit_state.current_bucket_index as usize] =
            new_bucket_value_outgoing;

        save_state(RATE_LIMIT_KEY, &rate_limit_state);

        Ok(())
    }
}
// Helper function to verify guardian signatures (order-based)
fn verify_guardian_signatures(
    signed_hash: String,
    signatures: String,
    guardian_keys: String,
) -> Result<(), SimpleErr> {
    unsafe {
        // Check if signatures have been used before (replay protection)
        let used_sig_key = format!("{}{}", USED_SIGNATURES_KEY.to_string(), signed_hash.clone());
        let used_sig = smart_contracts::retrieve_state(used_sig_key.clone());

        if !used_sig.is_empty() {
            return Err(SimpleErr::SignatureAlreadyUsed); // Signature already used
        }

        let guardian_state: GuardianState = load_state(GUARDIAN_STATE_KEY)?;
        let required_threshold = guardian_state.guardian_threshold as usize;

        // Parse signatures and guardian keys (comma-separated, parallel arrays)
        let sig_list: Vec<String> = signatures
            .split('|')
            .map(|s| s.trim().to_string())
            .collect();

        let key_list: Vec<String> = guardian_keys
            .split('|')
            .map(|s| s.trim().to_string())
            .collect();

        // Ensure parallel arrays have same length
        if sig_list.len() != key_list.len() {
            return Err(SimpleErr::InvalidGuardianSignatures);
        }

        // First, verify all passed public keys are in the guardian list
        for guardian_key in key_list.iter() {
            if guardian_key.is_empty() {
                continue; // Skip empty keys
            }

            if !guardian_state.guardians.contains(guardian_key) {
                return Err(SimpleErr::GuardianNotFound);
            }
        }

        // Now verify signatures against their corresponding public keys
        let mut valid_signatures = 0 as usize;

        for (i, signature) in sig_list.iter().enumerate() {
            if signature.is_empty() {
                continue; // Skip empty signatures
            }

            let guardian_pub_key = &key_list[i];

            if guardian_pub_key.is_empty() {
                continue; // Skip if no corresponding key
            }

            // Verify signature against the hash that guardians signed
            if smart_contracts::verify_signature(
                signed_hash.clone(),
                signature.clone(),
                guardian_pub_key.clone(),
            ) {
                valid_signatures += 1;
            }
        }

        // Check if threshold is met
        if valid_signatures >= required_threshold {
            // Mark signatures as used
            smart_contracts::store_state(used_sig_key.clone(), "used".to_string());
            return Ok(());
        }

        return Err(SimpleErr::GuardianSignaturesNotMet);
    }
}

fn check_pause(required_level: u8, current_time: u64) -> Result<(), SimpleErr> {
    unsafe {
        let mut pause_config: PauseConfig = load_state(PAUSE_CONFIG_KEY)?;

        let old_pause_level = pause_config.pause_level;

        if pause_config.pause_level > 0 && pause_config.pause_expiry > 0 {
            // Check if timed pause has expired
            if current_time >= pause_config.pause_expiry {
                pause_config.pause_level = 0;
                pause_config.pause_expiry = 0;
            }
        }

        if pause_config.pause_level >= required_level {
            return Err(SimpleErr::PauseLevelTooLow);
        }

        if old_pause_level != pause_config.pause_level {
            save_state(PAUSE_CONFIG_KEY, &pause_config);
        }

        return Ok(());
    }
}

#[wasmedge_bindgen]
pub fn init() {
    unsafe {
        //Initialize Guardian State
        let mut guardian_state: GuardianState = GuardianState {
            guardians: vec![],
            guardian_threshold: 2,
        };
        guardian_state
            .guardians
            .push("A_c_C68BgMJks69fsn5yr4cKNnYuw9yztW3vBNyk4hCyr3iE".to_string());
        guardian_state
            .guardians
            .push("A_c_B1NgczXgVbJjJLUdbHkQ5xe6fxnzvzQk7MP7o6JqK3dp".to_string());
        guardian_state
            .guardians
            .push("A_c_9aZ6ZymbUETdA9neSnLjvjj9iD8SqHfKo8L9QFtv1PGJ".to_string());
        /////////////////////
        save_state(GUARDIAN_STATE_KEY, &guardian_state);

        //Initialize Rate Limit State
        let current_time = smart_contracts::last_block_time();
        let current_hour = current_time / 3600 as u64;
        let rate_limit_state: BucketLimit = BucketLimit {
            current_hour: current_hour,
            hourly_buckets_incoming: [0; 24],
            hourly_buckets_outgoing: [0; 24],
            current_bucket_index: 0,
            single_limit: 100_000_000_000, //1m in cents
            rate_limit: 1_000_000_000_000, //10m in cents
        };
        save_state(RATE_LIMIT_KEY, &rate_limit_state);

        //Initialize Bridge Config
        let pause_config: PauseConfig = PauseConfig {
            pause_level: 0,
            pause_expiry: 0,
        };
        save_state(PAUSE_CONFIG_KEY, &pause_config);
    }
}
//USER FACING FUNCTION
//send_native_zera_to_solana
#[wasmedge_bindgen]
pub fn lock_zera(contract_id: String, amount: String, solana_address: String) {
    unsafe {
        if !types::is_valid_u256(amount.clone()) {
            return;
        }

        if amount.clone() == "0" {
            smart_contracts::emit("Failed: Amount cannot be 0".to_string());
            return;
        }

        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let current_time = smart_contracts::last_block_time();

        if let Err(e) = check_pause(2, current_time) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        // Check if the last 5 characters are "+0000"
        if contract_id.len() < 5 || !contract_id.ends_with("+0000") {
            smart_contracts::emit("Failed: Invalid contract id".to_string());
            return;
        }

        if !is_valid_solana_address(solana_address.clone()) {
            smart_contracts::emit(format!(
                "Failed: Invalid Solana address: {}",
                solana_address.clone()
            ));
            return;
        }

        let denomination = smart_contracts::contract_denomination(contract_id.clone());

        if !is_power_of_10(denomination) {
            smart_contracts::emit("Failed: Invalid denomination".to_string());
            return;
        }

        if let Err(e) = check_and_update_rate_limit(
            true,
            amount.clone(),
            current_time,
            contract_id.clone(),
            "0".to_string(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if (!smart_contracts::hold(contract_id.clone(), amount.clone())) {
            smart_contracts::emit("Failed: Lock failed".to_string());
            return;
        }
        let usd_value = get_usd_value(amount.clone(), contract_id.clone(), "0".to_string());
        let wallet_address = smart_contracts::wallet_address();
        smart_contracts::emit("EVENT:SEND_NATIVE_ZERA_TO_SOLANA".to_string());
        smart_contracts::emit(format!("contract_id: {}", contract_id.clone()));
        smart_contracts::emit(format!("amount: {}", amount.clone()));
        smart_contracts::emit(format!("solana_address: {}", solana_address.clone()));
        smart_contracts::emit(format!("zera_address: {}", wallet_address.clone()));
        smart_contracts::emit(format!("usd_value: {}", usd_value.clone()));
    }
}
//USER FACING FUNCTION
//send_wrapped_solana_to_solana
#[wasmedge_bindgen]
pub fn burn_sol(contract_id: String, amount: String, solana_address: String) {
    unsafe {
        if !types::is_valid_u256(amount.clone()) {
            return;
        }

        if amount.clone() == "0" {
            smart_contracts::emit("Failed: Amount cannot be 0".to_string());
            return;
        }

        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let current_time = smart_contracts::last_block_time();

        if let Err(e) = check_pause(2, current_time) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        // Check if the contract_id begins with "sol-"
        if !contract_id.starts_with("$sol-") {
            smart_contracts::emit("Failed: Invalid contract id".to_string());
            return;
        }

        // Check if the last 7 characters are "+000000"
        if contract_id.len() < 7 || !contract_id.ends_with("+000000") {
            smart_contracts::emit("Failed: Invalid contract id".to_string());
            return;
        }

        let mint_id_key = format!("{}{}", MINT_ID_KEY.to_string(), contract_id.clone());

        let mint_id = smart_contracts::retrieve_state(mint_id_key.clone());

        if mint_id.is_empty() {
            smart_contracts::emit("Failed: Mint ID not found ".to_string() + &contract_id.clone());
            return;
        }

        if !is_valid_solana_address(solana_address.clone()) {
            smart_contracts::emit(format!(
                "Failed: Invalid Solana address: {}",
                solana_address.clone()
            ));
            return;
        }

        if let Err(e) = check_and_update_rate_limit(
            true,
            amount.clone(),
            current_time,
            contract_id.clone(),
            "0".to_string(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if !smart_contracts::transfer(contract_id.clone(), amount.clone(), BURN_WALLET.to_string())
        {
            smart_contracts::emit("Failed: Transfer failed".to_string());
            return;
        }

        let wallet_address = smart_contracts::wallet_address();
        smart_contracts::emit("EVENT:SEND_WRAPPED_SOLANA_TO_SOLANA".to_string());
        smart_contracts::emit(format!("contract_id: {}", contract_id.clone()));
        smart_contracts::emit(format!("amount: {}", amount.clone()));
        smart_contracts::emit(format!("solana_address: {}", solana_address.clone()));
        smart_contracts::emit(format!("mint_id: {}", mint_id.clone()));
        smart_contracts::emit(format!("zera_address: {}", wallet_address.clone()));
    }
}
//PAYLOAD FUNCTION - mint already created solana coins on zera
//mint_native_solana_to_zera
#[wasmedge_bindgen]
pub fn mint_sol(
    mint_id: String,
    amount: String,
    wallet_address: String,
    token_price: String,
    tx_signature: String,
    signed_hash: String,
    signatures: String,
    guardian_keys: String,
) {
    unsafe {
        if !types::is_valid_u256(amount.clone()) {
            return;
        }

        if amount.clone() == "0" {
            smart_contracts::emit("Failed: Amount cannot be 0".to_string());
            return;
        }

        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let current_time = smart_contracts::last_block_time();

        if let Err(e) = check_pause(1, current_time) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if !types::is_valid_u256(token_price.clone()) {
            smart_contracts::emit("Failed: Invalid token price, must be a number".to_string());
            return;
        }

        let contract_id = smart_contracts::retrieve_state(format!(
            "{}{}",
            CONTRACT_EXIST_KEY.to_string(),
            mint_id.clone()
        ));

        if contract_id.is_empty() {
            smart_contracts::emit("Failed: Invalid mint id".to_string());
            return;
        }

        if let Err(e) = check_tx_signature(tx_signature.clone()) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let seed = format!(
            "{}{}{}{}",
            mint_id.clone(),
            amount.clone(),
            wallet_address.clone(),
            token_price.clone(),
            tx_signature.clone()
        );

        let manual_hash = smart_contracts::sha256(seed.clone());

        if manual_hash.clone() != signed_hash.clone() {
            smart_contracts::emit("FAILED:INVALID_HASH".to_string());
            return;
        }

        if let Err(e) = check_and_update_rate_limit(
            false,
            amount.clone(),
            current_time,
            contract_id.clone(),
            token_price.clone(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        // Verify guardian signatures
        if let Err(e) = verify_guardian_signatures(
            signed_hash.clone(),
            signatures.clone(),
            guardian_keys.clone(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        store_tx_signature(tx_signature.clone());

        if (!smart_contracts::mint(contract_id.clone(), amount.clone(), wallet_address.clone())) {
            smart_contracts::emit("Failed: Mint failed".to_string());
            return;
        }

        smart_contracts::store_state(
            format!("{}{}", TOKEN_PRICE_KEY.to_string(), contract_id.clone()),
            token_price.clone(),
        );

        // Proceed with minting logic
        smart_contracts::emit("SUCCESS:MINT_NATIVE_SOLANA_TO_ZERA".to_string());
    }
}
//PAYLOAD FUNCTION  - create new solana contracts on zera
//create_native_solana_to_zera
#[wasmedge_bindgen]
pub fn create_sol(
    symbol: String,
    name: String,
    denomination: String,
    wallet: String,
    amount: String,
    mint_id: String,
    uri: String,
    authorized_key: String,
    tx_signature: String,
    signed_hash: String,
    signatures: String,
    guardian_keys: String,
    token_price: String,
) {
    unsafe {
        if !types::is_valid_u256(amount.clone()) {
            return;
        }

        if amount.clone() == "0" {
            smart_contracts::emit("Failed: Amount cannot be 0".to_string());
            return;
        }

        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let current_time = smart_contracts::last_block_time();

        if let Err(e) = check_pause(1, current_time) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if let Err(e) = check_tx_signature(tx_signature.clone()) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if !types::is_valid_u256(token_price.clone()) {
            smart_contracts::emit("Failed: Invalid token price, must be a number".to_string());
            return;
        }

        let seed = format!(
            "{}{}{}{}{}{}{}{}{}{}",
            symbol.clone(),
            name.clone(),
            denomination.clone(),
            wallet.clone(),
            amount.clone(),
            mint_id.clone(),
            uri.clone(),
            authorized_key.clone(),
            token_price.clone(),
            tx_signature.clone()
        );

        let manual_hash = smart_contracts::sha256(seed.clone());

        if manual_hash.clone() != signed_hash.clone() {
            smart_contracts::emit("Failed: Invalid hash".to_string());
            return;
        }
        if let Err(e) = check_and_update_rate_limit(
            false,
            amount.clone(),
            current_time,
            contract_id.clone(),
            token_price.clone(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        // Verify guardian signatures
        if let Err(e) = verify_guardian_signatures(
            signed_hash.clone(),
            signatures.clone(),
            guardian_keys.clone(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        store_tx_signature(tx_signature.clone());

        //make key to check if mint id already exists
        let exist_key = format!("{}{}", CONTRACT_EXIST_KEY.to_string(), mint_id.clone());
        let contract_id_state = smart_contracts::retrieve_state(exist_key.clone());

        //if mint id does not exist create it with preminted amount, else mint the tokens
        if contract_id_state.is_empty() {
            //contract does not exist,
            //create key to see if symbol already exists
            let mut contract_id = SOLANA_PREFIX.to_string() + &symbol.clone();

            let symbol_config_key = format!("{}{}", SYMBOL_CONFIG_KEY.to_string(), symbol.clone());

            // Option 2: Provide explicit default
            let mut symbol_config: SymbolConfig =
                load_state(&symbol_config_key).unwrap_or(SymbolConfig { suffix_count: 0 });

            let formatted_suffix = format!("{:06}", symbol_config.suffix_count); // Pads with zeros to 6 digits
            contract_id = contract_id + "+" + &formatted_suffix;
            symbol_config.suffix_count = symbol_config.suffix_count + 1;

            let res = smart_contracts::instrument_contract_bridge(
                symbol.clone(),
                name.clone(),
                denomination.clone(),
                contract_id.clone(),
                mint_id.clone(),
                uri.clone(),
                authorized_key.clone(),
                wallet.clone(),
                amount.clone(),
            );

            if (res != "OK") {
                smart_contracts::emit("Failed: Contract creation failed".to_string());
                return;
            }

            smart_contracts::store_state(
                format!("{}{}", TOKEN_PRICE_KEY.to_string(), contract_id.clone()),
                token_price.clone(),
            );

            let mint_id_key = format!("{}{}", MINT_ID_KEY.to_string(), contract_id.clone());

            smart_contracts::store_state(exist_key.clone(), contract_id.clone());

            save_state(SYMBOL_CONFIG_KEY, &symbol_config);

            smart_contracts::store_state(mint_id_key.clone(), mint_id.clone());

            smart_contracts::emit("SUCCESS:CONTRACT_CREATED".to_string());
            smart_contracts::emit(format!("contract_id: {}", contract_id.clone()));
            smart_contracts::emit(format!("mint_id: {}", mint_id.clone()));
            smart_contracts::emit(format!("recipient: {}", wallet.clone()));
            smart_contracts::emit(format!("amount: {}", amount.clone()));
        } else {
            smart_contracts::emit("Failed: Contract already exists".to_string());
            return;
        }
    }
}
//PAYLOAD FUNCTION - release native zera coins back to zera //test working
//release_native_zera_to_zera
#[wasmedge_bindgen]
pub fn release_zera(
    contract_id: String,
    amount: String,
    wallet_address: String,
    tx_signature: String,
    signed_hash: String,
    signatures: String,
    guardian_keys: String,
) {
    unsafe {
        if !types::is_valid_u256(amount.clone()) {
            return;
        }

        if amount.clone() == "0" {
            smart_contracts::emit("Failed: Amount cannot be 0".to_string());
            return;
        }

        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let current_time = smart_contracts::last_block_time();

        if let Err(e) = check_pause(1, current_time) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if let Err(e) = check_tx_signature(tx_signature.clone()) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        //verify hash
        let seed = format!(
            "{}{}{}{}",
            contract_id.clone(),
            amount.clone(),
            wallet_address.clone(),
            tx_signature.clone()
        );

        let manual_hash = smart_contracts::sha256(seed.clone());

        if manual_hash.clone() != signed_hash.clone() {
            smart_contracts::emit("Failed: Invalid hash".to_string());
            return;
        }

        if let Err(e) = check_and_update_rate_limit(
            false,
            amount.clone(),
            current_time,
            contract_id.clone(),
            "0".to_string(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        // Verify guardian signatures
        if let Err(e) = verify_guardian_signatures(
            signed_hash.clone(),
            signatures.clone(),
            guardian_keys.clone(),
        ) {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        // Mark nonce as used
        store_tx_signature(tx_signature.clone());

        if (!smart_contracts::send(contract_id.clone(), amount.clone(), wallet_address.clone())) {
            smart_contracts::emit("Failed: Send failed".to_string());
            return;
        }

        smart_contracts::emit("SUCCESS:RELEASE_NATIVE_ZERA".to_string());
        smart_contracts::emit(format!("contract_id: {}", contract_id.clone()));
        smart_contracts::emit(format!("amount: {}", amount.clone()));
        smart_contracts::emit(format!("recipient: {}", wallet_address.clone()));
    }
}

#[wasmedge_bindgen]
pub fn update_pause_config(pause_level: String, pause_expiry: String) {
    unsafe {
        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if let Err(e) = check_gov_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let mut pause_config: PauseConfig = match load_state(PAUSE_CONFIG_KEY) {
            Ok(state) => state,
            Err(e) => {
                smart_contracts::emit(format!("Failed: {}", e.msg()));
                return;
            }
        };

        if (pause_expiry.parse::<u64>().is_err()) {
            smart_contracts::emit(format!(
                "Failed: Invalid pause expiry: {}",
                pause_expiry.clone()
            ));
            return;
        }

        if (pause_level.parse::<u8>().is_err()) {
            smart_contracts::emit(format!(
                "Failed: Invalid pause level: {}",
                pause_level.clone()
            ));
            return;
        }

        if (pause_level != "0" && pause_level != "1" && pause_level != "2") {
            smart_contracts::emit(format!(
                "Failed: Invalid pause level: {}",
                pause_level.clone()
            ));
            return;
        }

        let pause_level_value = pause_level.parse::<u8>().unwrap();
        let pause_expiry_value = pause_expiry.parse::<u64>().unwrap();

        pause_config.pause_level = pause_level_value;
        pause_config.pause_expiry = pause_expiry_value;

        if !save_state(PAUSE_CONFIG_KEY, &pause_config) {
            smart_contracts::emit("Failed: Failed to save pause config".to_string());
            return;
        }

        smart_contracts::emit("SUCCESS:PAUSE_CONFIG_UPDATED".to_string());
        smart_contracts::emit(format!("Pause level: {}", pause_level_value));
        smart_contracts::emit(format!("Pause expiry: {}", pause_expiry_value));
    }
}

#[wasmedge_bindgen]
pub fn update_guardian_state(guardians: String, guardian_threshold: String) {
    unsafe {
        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if let Err(e) = check_gov_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if guardian_threshold.parse::<u64>().is_err() {
            smart_contracts::emit(format!(
                "Failed: Invalid guardian threshold: {}",
                guardian_threshold.clone()
            ));
            return;
        }

        let guardian_threshold_value = guardian_threshold.parse::<u64>().unwrap();

        if guardian_threshold_value == 0 {
            smart_contracts::emit("Failed: Guardian threshold cannot be zero".to_string());
            return;
        }

        //This checks that the guardian state is being changed on the solana side aswell.
        let timestamp = smart_contracts::block_timestamp();
        let key = format!("{}{}", GUARDIAN_STATE_KEY, timestamp.to_string());

        let guardian_keys: GuardianKeys = match load_delegate_state(key.as_str()) {
            Ok(state) => state,
            Err(e) => {
                smart_contracts::emit(format!("Failed: Solana guardian state not found",));
                return;
            }
        };

        // Split incoming guardians - keep original with prefixes for Zera storage
        let mut guardians_vec: Vec<String> =
            guardians.split("|").map(|s| s.trim().to_string()).collect();

        if guardians_vec.is_empty() {
            smart_contracts::emit("Failed: No guardians provided".to_string());
            return;
        }

        if guardian_threshold_value > guardians_vec.len() as u64 {
            smart_contracts::emit(
                format!(format!("Failed: Threshold ({}) cannot be greater than number of guardian keys ({})", guardian_threshold_value, guardians_vec.len() as u64);),
            );
            return;
        }

        // Validate each guardian has correct Zera prefix format
        for (i, guardian) in guardians_vec.iter().enumerate() {
            if !validate_zera_guardian_prefix(guardian) {
                smart_contracts::emit(format!(
                        "Failed: Invalid Zera guardian key format at position {}: {}. Expected format: ONE uppercase + underscore, then optional (lowercase + underscore) pairs, then base58 key",
                        i, guardian
                    ));
                return;
            }
        }

        guardians_vec.sort();

        // Strip prefixes to validate against Solana guardian keys
        let mut guardians_stripped: Vec<String> = guardians_vec
            .iter()
            .map(|s| strip_guardian_prefix(s))
            .collect();

        guardians_stripped.sort();

        // Create a sorted version of guardian_keys for comparison
        let mut expected_keys = guardian_keys.guardian_keys.clone();
        expected_keys.sort();

        // Validate that stripped guardians match the expected guardian keys from Solana
        if guardians_stripped.len() != expected_keys.len() {
            smart_contracts::emit(format!(
                "Failed: Guardian count mismatch. Expected {}, got {}",
                expected_keys.len(),
                guardians_stripped.len()
            ));
            return;
        }

        for (i, guardian) in guardians_stripped.iter().enumerate() {
            if guardian != &expected_keys[i] {
                smart_contracts::emit(format!(
                    "Failed: Guardian mismatch at position {}. Expected {}, got {}",
                    i, expected_keys[i], guardian
                ));
                return;
            }
        }

        // Validate threshold matches
        if guardian_threshold_value != guardian_keys.guardian_threshold {
            smart_contracts::emit(format!(
                "Failed: Guardian threshold mismatch. Expected {}, got {}",
                guardian_keys.guardian_threshold, guardian_threshold
            ));
            return;
        }

        let mut guardian_state: GuardianState = match load_state(GUARDIAN_STATE_KEY) {
            Ok(state) => state,
            Err(e) => {
                smart_contracts::emit(format!("Failed: {}", e.msg()));
                return;
            }
        };

        // Store original guardians WITH prefixes for Zera network
        guardian_state.guardians = guardians_vec;
        guardian_state.guardian_threshold = guardian_threshold_value;

        if !save_state(GUARDIAN_STATE_KEY, &guardian_state) {
            smart_contracts::emit("Failed: Failed to save guardian state".to_string());
            return;
        }

        smart_contracts::emit("EVENT:GUARDIAN_STATE_UPDATED".to_string());
        smart_contracts::emit(format!("timestamp: {}", timestamp.to_string()));
    }
}

#[wasmedge_bindgen]
pub fn reset_rate_limit() {
    unsafe {
        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if let Err(e) = check_gov_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        let current_hour = smart_contracts::last_block_time() / 3600 as u64;

        let mut rate_limit_state: BucketLimit = match load_state(RATE_LIMIT_KEY) {
            Ok(state) => state,
            Err(e) => {
                smart_contracts::emit(format!("Failed: {}", e.msg()));
                return;
            }
        };

        rate_limit_state.hourly_buckets_incoming = [0; 24];
        rate_limit_state.hourly_buckets_outgoing = [0; 24];
        rate_limit_state.current_hour = current_hour;
        rate_limit_state.current_bucket_index = 0;

        save_state(RATE_LIMIT_KEY, &rate_limit_state);

        smart_contracts::emit("SUCCESS:RATE_LIMIT_RESET".to_string());
    }
}

#[wasmedge_bindgen]
pub fn update_rate_limit(single_limit: String, rate_limit: String) {
    unsafe {
        if let Err(e) = check_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if let Err(e) = check_gov_auth() {
            smart_contracts::emit(format!("Failed: {}", e.msg()));
            return;
        }

        if single_limit.parse::<u64>().is_err() {
            smart_contracts::emit(format!(
                "Failed: Invalid single limit: {}",
                single_limit.clone()
            ));
            return;
        }

        if rate_limit.parse::<u64>().is_err() {
            smart_contracts::emit(format!(
                "Failed: Invalid rate limit: {}",
                rate_limit.clone()
            ));
            return;
        }

        let single_limit_value = single_limit.parse::<u64>().unwrap();
        let rate_limit_value = rate_limit.parse::<u64>().unwrap();

        if single_limit_value > rate_limit_value {
            smart_contracts::emit(
                "Failed: Single limit cannot be greater than rate limit".to_string(),
            );
            return;
        }

        let mut rate_limit_state: BucketLimit = match load_state(RATE_LIMIT_KEY) {
            Ok(state) => state,
            Err(e) => {
                smart_contracts::emit(format!("Failed: {}", e.msg()));
                return;
            }
        };

        rate_limit_state.single_limit = single_limit_value;
        rate_limit_state.rate_limit = rate_limit_value;

        if !save_state(RATE_LIMIT_KEY, &rate_limit_state) {
            {
                smart_contracts::emit("Failed: Failed to save rate limit".to_string());
                return;
            }

            smart_contracts::emit("SUCCESS:RATE_LIMIT_UPDATED".to_string());
            smart_contracts::emit(format!("Single limit: {}", single_limit_value));
            smart_contracts::emit(format!("Rate limit: {}", rate_limit_value));
        }
    }

    fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
        let bytes = postcard::to_allocvec(data).unwrap();
        let b64 = base64::encode(bytes);
        unsafe { smart_contracts::store_state(key.to_string(), b64) }
    }
    fn load_state<T: DeserializeOwned>(key: &str) -> Result<T, SimpleErr> {
        let b64 = unsafe { smart_contracts::retrieve_state(key.to_string()) };
        let bytes = base64::decode(b64).map_err(|_| SimpleErr::InvalidStateData)?;
        postcard::from_bytes(&bytes).map_err(|_| SimpleErr::InvalidStateData)
    }
    fn load_delegate_state<T: DeserializeOwned>(key: &str) -> Result<T, SimpleErr> {
        let b64 = unsafe {
            smart_contracts::delegate_retrieve_state(
                key.to_string(),
                BRIDGE_PROXY_CONTRACT.to_string(),
            )
        };
        let bytes = base64::decode(b64).map_err(|_| SimpleErr::InvalidStateData)?;
        postcard::from_bytes(&bytes).map_err(|_| SimpleErr::InvalidStateData)
    }

    #[derive(Serialize, Deserialize)]
    pub struct BucketLimit {
        pub current_hour: u64,                  // Current hour (Unix timestamp / 3600)
        pub hourly_buckets_incoming: [u64; 24], // Net flow per hour in USD cents (signed)
        pub hourly_buckets_outgoing: [u64; 24], // Net flow per hour in USD cents (signed)
        pub current_bucket_index: u8,           // Which bucket we're currently in (0-23)
        pub single_limit: u64,
        pub rate_limit: u64,
    }
    #[derive(Serialize, Deserialize)]
    pub struct SymbolConfig {
        pub suffix_count: u64,
    }

    #[derive(Serialize, Deserialize)]
    pub struct GuardianState {
        pub guardians: Vec<String>,
        pub guardian_threshold: u8,
    }

    #[derive(Serialize, Deserialize)]
    pub struct PauseConfig {
        pub pause_level: u8,
        pub pause_expiry: u64,
    }

    #[derive(Serialize, Deserialize)]
    pub struct GuardianKeys {
        pub guardian_keys: Vec<String>,
        pub guardian_threshold: u64,
    }

    pub enum SimpleErr {
        SingleTxLimitExceeded,
        RateLimitExceeded,
        InvalidGuardianSignatures,
        InvalidHash,
        InvalidNonce,
        InvalidContractId,
        ArithmeticOverflow,
        SignatureAlreadyUsed,
        GuardianSystemNotInitialized,
        GuardianNotFound,
        GuardianSignaturesNotMet,
        UnauthorizedSender,
        UsedTxSignature,
        SerializationError,
        InvalidStateData,
        PauseLevelTooLow,
    }

    impl SimpleErr {
        pub fn msg(&self) -> &str {
            match self {
                SimpleErr::SingleTxLimitExceeded => "Single transaction limit exceeded",
                SimpleErr::RateLimitExceeded => "Rate limit exceeded",
                SimpleErr::InvalidGuardianSignatures => "Invalid guardian signatures",
                SimpleErr::InvalidHash => "Invalid hash",
                SimpleErr::InvalidNonce => "Invalid nonce",
                SimpleErr::InvalidContractId => "Invalid contract id",
                SimpleErr::ArithmeticOverflow => "Arithmetic overflow",
                SimpleErr::SignatureAlreadyUsed => "Signature already used",
                SimpleErr::GuardianSystemNotInitialized => "Guardian system not initialized",
                SimpleErr::GuardianNotFound => "Guardian not found",
                SimpleErr::GuardianSignaturesNotMet => "Guardian signatures not met",
                SimpleErr::UnauthorizedSender => "Unauthorized sender",
                SimpleErr::UsedTxSignature => "Tx signature already used",
                SimpleErr::SerializationError => "Serialization error",
                SimpleErr::InvalidStateData => "Invalid state data",
                SimpleErr::PauseLevelTooLow => "Pause level too low",
            }
        }
    }
}
