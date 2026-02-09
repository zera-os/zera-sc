pub mod zera_bridge_proxy {
    use base64::{decode, encode};
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use native_functions::zera::wasmedge_bindgen;
    use postcard::{from_bytes, to_allocvec};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    //wallet - 66Eb7Yo5S2Qz8wbfHz8q9UhUKST9LNAGNxsa2zokB6U8
    const SMART_CONTRACT_KEY: &str = "SMART_CONTRACT_";
    const GOV_KEYS_KEY: &str = "GOV_KEYS_";
    const ZRA_CONTRACT: &str = "$ZRA+0000";

    const GUARDIAN_STATE_KEY: &str = "GUARDIAN_STATE"; //key for guardian state
    const RATE_LIMIT_KEY: &str = "RATE_LIMIT_"; //key for rate limit
    const PAUSE_CONFIG_KEY: &str = "PAUSE_CONFIG"; //key for pause config

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe {
            let gov_keys = GovKeys {
                update_key: "gov_$BRIDGEGUARDIAN+0000".to_string(),
                send_all_key: "gov_$BRIDGEGUARDIAN+0000".to_string(),
            };

            let smart_contract_state = SmartContractState {
                smart_contract: "bridge_v2".to_string(),
                instance: "1".to_string(),
                sc_gov: "bridge_gov_v2".to_string(),
                sc_gov_instance: "1".to_string(),
            };

            save_state(GOV_KEYS_KEY, &gov_keys);
            save_state(SMART_CONTRACT_KEY, &smart_contract_state);
            store_old_states();
        }
    }

    #[wasmedge_bindgen]
    pub fn execute(function: String, parameters: String) {
        unsafe {
            let smart_contract_state: SmartContractState = load_state(SMART_CONTRACT_KEY).unwrap();

            let parameters_vec: Vec<String> = parameters
                .clone()
                .split(",")
                .map(|s| s.to_string())
                .collect();

            let results = smart_contracts::delegatecall(
                smart_contract_state.smart_contract.clone(),
                smart_contract_state.instance.clone(),
                function.clone(),
                parameters_vec.clone(),
            );

            for result in results {
                smart_contracts::emit(result.clone());
            }
        }
    }

    #[wasmedge_bindgen]
    pub fn execute_gov(function: String, parameters: String) {
        unsafe {
            let sc_gov_state: SmartContractState = load_state(SMART_CONTRACT_KEY).unwrap();

            let parameters_vec: Vec<String> = parameters
                .clone()
                .split(",")
                .map(|s| s.to_string())
                .collect();

            let results = smart_contracts::delegatecall(
                sc_gov_state.sc_gov.clone(),
                sc_gov_state.sc_gov_instance.clone(),
                function.clone(),
                parameters_vec.clone(),
            );

            for result in results {
                smart_contracts::emit(result.clone());
            }
        }
    }

    #[wasmedge_bindgen]
    pub fn update_gov(smart_contract: String, instance: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();
            let gov_keys: GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.update_key.to_string() {
                smart_contracts::emit("Failed: Unauthorized sender".to_string());
                return;
            }

            if !smart_contracts::smart_contract_exists(smart_contract.clone(), instance.clone()) {
                smart_contracts::emit("Failed: Smart contract does not exist".to_string());
                return;
            }

            let mut sc_gov_state: SmartContractState = load_state(SMART_CONTRACT_KEY).unwrap();
            sc_gov_state.sc_gov = smart_contract.clone();
            sc_gov_state.sc_gov_instance = instance.clone();

            if !save_state(SMART_CONTRACT_KEY, &sc_gov_state) {
                smart_contracts::emit(
                    "Failed: Failed to save governance smart contract state".to_string(),
                );
                return;
            }

            smart_contracts::emit("SUCCESS:GOVERNANCE_SMART_CONTRACT_UPDATED".to_string());
            smart_contracts::emit(format!(
                "Governance smart contract: {}",
                smart_contract.clone()
            ));
            smart_contracts::emit(format!("Governance instance: {}", instance.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update(smart_contract: String, instance: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let gov_keys: GovKeys = load_state(GOV_KEYS_KEY).unwrap();
            if pub_key != gov_keys.update_key.to_string() {
                smart_contracts::emit("Failed: Unauthorized sender".to_string());
                return;
            }

            if !smart_contracts::smart_contract_exists(smart_contract.clone(), instance.clone()) {
                smart_contracts::emit("Failed: Smart contract does not exist".to_string());
                return;
            }

            let mut smart_contract_state: SmartContractState =
                load_state(SMART_CONTRACT_KEY).unwrap();
            smart_contract_state.smart_contract = smart_contract.clone();
            smart_contract_state.instance = instance.clone();

            if !save_state(SMART_CONTRACT_KEY, &smart_contract_state) {
                smart_contracts::emit("Failed: Failed to save smart contract state".to_string());
                return;
            }

            smart_contracts::emit("SUCCESS:SMART_CONTRACT_UPDATED".to_string());
            smart_contracts::emit(format!("Smart contract: {}", smart_contract.clone()));
            smart_contracts::emit(format!("Instance: {}", instance.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update_update_key(update_key: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let mut gov_keys: GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.update_key.to_string() {
                smart_contracts::emit("Failed: Unauthorized sender".to_string());
                return;
            }

            if !verify_update_gov_key(update_key.clone()) {
                smart_contracts::emit("Failed: Invalid governance authorization key".to_string());
                return;
            }

            gov_keys.update_key = update_key.clone();
            if !save_state(GOV_KEYS_KEY, &gov_keys) {
                smart_contracts::emit("Failed: Failed to save governance keys".to_string());
                return;
            }
            smart_contracts::emit("SUCCESS:UPDATE_KEY_UPDATED".to_string());
            smart_contracts::emit(format!("Update key: {}", update_key.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn update_send_all_key(send_all_key: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let mut gov_keys: GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.send_all_key.to_string() {
                smart_contracts::emit("Failed: Unauthorized sender".to_string());
                return;
            }

            if !verify_update_gov_key(send_all_key.clone()) {
                smart_contracts::emit("Failed: Invalid governance authorization key".to_string());
                return;
            }

            gov_keys.send_all_key = send_all_key.clone();
            if !save_state(GOV_KEYS_KEY, &gov_keys) {
                smart_contracts::emit("Failed: Failed to save governance keys".to_string());
                return;
            }

            smart_contracts::emit("SUCCESS:SEND_ALL_KEY_UPDATED".to_string());
            smart_contracts::emit(format!("Send all key: {}", send_all_key.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn send_all(wallet: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();
            let gov_keys: GovKeys = load_state(GOV_KEYS_KEY).unwrap();
            if pub_key != gov_keys.send_all_key.to_string() {
                smart_contracts::emit("Failed: Unauthorized sender".to_string());
                return;
            }

            if !smart_contracts::wallet_exists(wallet.clone()) {
                smart_contracts::emit("Failed: Wallet does not exist".to_string());
                return;
            }

            smart_contracts::send_all(wallet.clone());
            smart_contracts::emit("SUCCESS:SEND_ALL".to_string());
            smart_contracts::emit(format!("Wallet: {}", wallet.clone()));
        }
    }

    fn verify_update_gov_key(key: String) -> bool {
        unsafe {
            // Validate that send_all_key starts with "gov_"
            if !key.starts_with("gov_") {
                return false;
            }

            // Extract contract name after "gov_" prefix
            let contract_id = &key[4..]; // Skip "gov_" (4 characters)

            // Validate that there's something after "gov_"
            if contract_id.is_empty() {
                return false;
            }

            // Verify the contract exists
            if !smart_contracts::contract_exists(contract_id.to_string()) {
                return false;
            }

            // If passed this verifys that this gov key is from a valid contract.
            return true;
        }
    }

    fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
        let bytes = postcard::to_allocvec(data).unwrap();
        let b64 = base64::encode(bytes);
        unsafe { smart_contracts::store_state(key.to_string(), b64) }
    }
    fn load_state<T: DeserializeOwned>(key: &str) -> Result<T, bool> {
        let b64 = unsafe { smart_contracts::retrieve_state(key.to_string()) };
        let bytes = base64::decode(b64).map_err(|_| false)?;
        postcard::from_bytes(&bytes).map_err(|_| false)
    }

    fn store_old_states() {
        unsafe {
            smart_contracts::store_state(
                "CONTRACT_EXISTEPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
                "$sol-USDC+000000".to_string(),
            );
            smart_contracts::store_state(
                "CONTRACT_EXISTSo11111111111111111111111111111111111111111".to_string(),
                "$sol-SOL+000000".to_string(),
            );
            smart_contracts::store_state("GUARDIAN_STATE".to_string(), "AzBBX2NfQzY4QmdNSmtzNjlmc241eXI0Y0tObll1dzl5enRXM3ZCTnlrNGhDeXIzaUUwQV9jX0IxTmdjelhnVmJKakpMVWRiSGtRNXhlNmZ4bnp2elFrN01QN282SnFLM2RwMEFfY185YVo2WnltYlVFVGRBOW5lU25ManZqajlpRDhTcUhmS284TDlRRnR2MVBHSgI=".to_string());
            smart_contracts::store_state(
                "MINT_ID$sol-SOL+000000".to_string(),
                "So11111111111111111111111111111111111111111".to_string(),
            );
            smart_contracts::store_state(
                "MINT_ID$sol-USDC+000000".to_string(),
                "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string(),
            );
            smart_contracts::store_state("PAUSE_CONFIG".to_string(), "AAA=".to_string());
            smart_contracts::store_state("RATE_LIMIT_".to_string(), "9vcdAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA6r4BAAAA1P0CAAAAAAAAAAAAAAAAAAAAAAAAAASA0NvD9AKAoJSljR0=".to_string());
            smart_contracts::store_state("SYMBOL_CONFIG".to_string(), "AQ==".to_string());
            smart_contracts::store_state("TX_SIGNATURE2qf74vqhdAhNNgjmAiG3gxyhaG2PsP2NAxsKEAzHwaEyr5xQuPsxkP7fSKzU3GmyVtfo1FoBHSyHAe8k6jvgmkaB".to_string(), "1".to_string());
            smart_contracts::store_state("TX_SIGNATUREwt6w8vw3Xp7EDwDehDVAnADK8BFbW36ttJpPEm3Wcim8qDLDENFZFBCWi1tLFobRXyRjojTR3iodqTS7GCEuqHM".to_string(), "1".to_string());
            smart_contracts::store_state(
                "USED_SIG_66ba05d2a198dd58a8deb23f5e799a31899c4ec70f22efb860ed8649f130f75c"
                    .to_string(),
                "used".to_string(),
            );
            smart_contracts::store_state(
                "USED_SIG_cf0188f7801d28d0b349336f5c8150d890a2ac1f3a7bc2a4c42c177d8da39c48"
                    .to_string(),
                "used".to_string(),
            );

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

    #[derive(Serialize, Deserialize)]
    pub struct SmartContractState {
        pub smart_contract: String,
        pub instance: String,
        pub sc_gov: String,
        pub sc_gov_instance: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct GovKeys {
        pub update_key: String,
        pub send_all_key: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct GuardianState {
        pub guardians: Vec<String>,
        pub guardian_threshold: u8,
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
    pub struct PauseConfig {
        pub pause_level: u8,
        pub pause_expiry: u64,
    }
}
