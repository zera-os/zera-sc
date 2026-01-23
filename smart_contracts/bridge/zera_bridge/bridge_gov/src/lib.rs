pub mod bridge_gov_v2 {
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::types::is_valid_u256;
    use native_functions::zera::types::string_to_u256;
    use native_functions::zera::types::U256;
    use serde::{Serialize, Deserialize};
    use serde::de::DeserializeOwned;
    use base64::{encode, decode};
    use postcard::{to_allocvec, from_bytes};

    const GOV_CONTRACT: &str = "gov_$BRIDGEGUARDIAN+0000";
    const PROXY_WALLET: &str = "66Eb7Yo5S2Qz8wbfHz8q9UhUKST9LNAGNxsa2zokB6U8"; //sc_zera_bridge_proxy_1
    const GUARDIAN_KEYS_KEY: &str = "GUARDIAN_KEYS";
    const BRIDGE_PROXY_CONTRACT: &str = "zera_bridge_proxy_1";

    fn check_auth() -> bool {
        unsafe{
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
            let sc_wallet = sc_wallet_.clone();

            if sc_wallet != PROXY_WALLET.to_string() {
                smart_contracts::emit(format!("Failed: Unauthorized sender key: {}", sc_wallet.clone()));
                return false;
            }

            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            if(pub_key != GOV_CONTRACT.to_string())
            {
                smart_contracts::emit(format!("Failed: Unauthorized sender key: {}", pub_key.clone()));
                return false;
            }
        }
        return true;
    }

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
        const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        
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
    #[wasmedge_bindgen]
    pub fn init() {
        unsafe{

        }
    }

    //pause_level 0=Unpause, 1=IncomingOnly, 2=Complete
    //pause_duration in seconds 0=Indefinite
    #[wasmedge_bindgen]
    pub fn pause(pause_level: String, pause_duration: String) {
        unsafe{
            let mut pause_duration = pause_duration;
            if(!check_auth())
            {
                return;
            }

            if(pause_duration.parse::<u64>().is_err())
            {
                smart_contracts::emit(format!("Failed: Invalid pause duration: {}", pause_duration.clone()));
                return;
            }

            if(pause_level.parse::<u64>().is_err())
            {
                smart_contracts::emit(format!("Failed: Invalid pause level: {}", pause_level.clone()));
                return;
            }
            
            if(pause_level != "0" && pause_level != "1" && pause_level != "2") 
            {
                smart_contracts::emit(format!("Failed: Invalid pause level: {}", pause_level.clone()));
                return;
            }


            smart_contracts::emit("EVENT:PAUSE_SOLANA_BRIDGE".to_string());
            smart_contracts::emit(format!("pause_level: {}", pause_level.clone()));
            smart_contracts::emit(format!("pause_duration: {}", pause_duration.clone()));

        }
    
    }

    //buffer_account and spill_account are base58 encoded strings
    //buffer account is the account that hold the updated token bridge on solana
    //spill account is the account is where the refunded solana is sent to
    #[wasmedge_bindgen]
    pub fn update_token_bridge(buffer_account: String, spill_account: String) {
        unsafe{

            if(!check_auth())
            {
                return;
            }

            // Validate buffer_account is a valid Solana address
            if !is_valid_solana_address(&buffer_account) {
                smart_contracts::emit(format!("Failed: Invalid buffer_account address: {}", buffer_account));
                return;
            }

            smart_contracts::emit("EVENT:UPDATE_TOKEN_BRIDGE".to_string());
            smart_contracts::emit(format!("buffer: {}", buffer_account.clone()));
            smart_contracts::emit(format!("spill: {}", spill_account.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update_core_bridge(buffer_account: String, spill_account: String) {
        unsafe{

            if(!check_auth())
            {
                return;
            }

            // Validate buffer_account is a valid Solana address
            if !is_valid_solana_address(&buffer_account) {
                smart_contracts::emit(format!("Failed: Invalid buffer_account address: {}", buffer_account));
                return;
            }

            // Validate spill_account is a valid Solana address
            if !is_valid_solana_address(&spill_account) {
                smart_contracts::emit(format!("Failed: Invalid spill_account address: {}", spill_account));
                return;
            }

            smart_contracts::emit("EVENT:UPDATE_CORE_BRIDGE".to_string());
            smart_contracts::emit(format!("buffer: {}", buffer_account.clone()));
            smart_contracts::emit(format!("spill: {}", spill_account.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update_guardian_keys(guardian_keys: String, threshold: String) {
        unsafe{

            if(!check_auth())
            {
                return;
            }

            // Validate threshold is a valid u64
            if(threshold.parse::<u64>().is_err())
            {
                smart_contracts::emit(format!("Failed: Invalid threshold: {}", threshold.clone()));
                return;
            }

            // Split guardian keys by "|" and validate each one
            let keys: Vec<&str> = guardian_keys.split('|').collect();
            
            // Check that we have at least one key
            if keys.is_empty() {
                smart_contracts::emit("Failed: No guardian keys provided".to_string());
                return;
            }

            let mut guardian_keys_vec: Vec<String> = Vec::new();

            // Validate each guardian key is a valid Solana address
            for (index, key) in keys.iter().enumerate() {
                let trimmed_key = key.trim();
                
                // Check key is not empty
                if trimmed_key.is_empty() {
                    smart_contracts::emit(format!("Failed: Empty guardian key at position {}", index));
                    return;
                }
                
                // Validate it's a proper Solana address
                if !is_valid_solana_address(trimmed_key) {
                    smart_contracts::emit(format!("Failed: Invalid Solana address at position {}: {}", index, trimmed_key));
                    return;
                }

                guardian_keys_vec.push(trimmed_key.to_string());
            }


            // Validate threshold is not greater than number of keys
            let threshold_value = threshold.parse::<u64>().unwrap();
            let num_keys = keys.len() as u64;
            
            if threshold_value > num_keys {
                smart_contracts::emit(format!("Failed: Threshold ({}) cannot be greater than number of guardian keys ({})", threshold_value, num_keys));
                return;
            }

            if threshold_value == 0 {
                smart_contracts::emit("Failed: Threshold cannot be zero".to_string());
                return;
            }

            let guardian_keys_state: GuardianKeys = GuardianKeys {
                guardian_keys: guardian_keys_vec,
                guardian_threshold: threshold_value,
            };

            let timestamp = smart_contracts::block_timestamp();

            let key = format!("{}{}", GUARDIAN_KEYS_KEY, timestamp.to_string());

            if !save_state(key.as_str(), &guardian_keys_state) {
                smart_contracts::emit("Failed: Failed to save guardian keys state".to_string());
                return;
            }

            smart_contracts::emit("EVENT:UPDATE_GUARDIAN_KEYS".to_string());
            smart_contracts::emit(format!("guardian_keys: {}", guardian_keys.clone()));
            smart_contracts::emit(format!("threshold: {}", threshold.clone()));
            smart_contracts::emit(format!("timestamp: {}", timestamp.to_string()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update_single_limit(limit: String) {
        unsafe{

            if(!check_auth())
            {
                return;
            }

            if(limit.parse::<u64>().is_err())
            {
                smart_contracts::emit(format!("Failed: {}", limit.clone()));
                return;
            }
            
            
            smart_contracts::emit("EVENT:UPDATE_SINGLE_LIMIT".to_string());
            smart_contracts::emit(format!("limit: {}", limit.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update_rate_limit(limit: String) {
        unsafe{

            if(!check_auth())
            {
                return;
            }

            if(limit.parse::<u64>().is_err())
            {
                smart_contracts::emit(format!("Failed: Invalid rate limit: {}", limit.clone()));
                return;
            }
            
            
            smart_contracts::emit("EVENT:UPDATE_RATE_LIMIT".to_string());
            smart_contracts::emit(format!("limit: {}", limit.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn reset_rate_limit() {
        unsafe{

            if(!check_auth())
            {
                return;
            }

            smart_contracts::emit("EVENT:RESET_RATE_LIMIT".to_string());
        }
    }

    fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
        let bytes = postcard::to_allocvec(data).unwrap();
        let b64 = base64::encode(bytes);
        unsafe { smart_contracts::delegate_store_state(key.to_string(), b64, BRIDGE_PROXY_CONTRACT.to_string()) }
    }

    #[derive(Serialize, Deserialize)]
    pub struct GuardianKeys {
        pub guardian_keys: Vec<String>,
        pub guardian_threshold: u64,
    }

}