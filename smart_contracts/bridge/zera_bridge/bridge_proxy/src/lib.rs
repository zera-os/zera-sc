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
            smart_contracts::emit(format!("Governance smart contract: {}", smart_contract.clone()));
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
}
