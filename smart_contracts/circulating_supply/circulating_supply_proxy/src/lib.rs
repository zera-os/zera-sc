pub mod circulating_supply_proxy {
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::smart_contracts;
    use serde::{Deserialize, Serialize};
    use serde::de::DeserializeOwned;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use base64::{encode, decode};
    use postcard::{to_allocvec, from_bytes};

    const SMART_CONTRACT_KEY: &str = "SMART_CONTRACT_";
    const GOV_KEYS_KEY: &str = "GOV_KEYS_";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const WHITELIST_SC: &str = "WHITELIST_SC";

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe{
            //1000000000000000000 = 1$ from get_ace_data
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("500000000000000000".to_string()); //0.5$
            let one_dolla_zera = (one_dolla * denomination) / rate;
            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string());
            
            let gov_keys = GovKeys {
                update_key: "gov_$ZRA+0000".to_string(),
                send_all_key: "gov_$ZRA+0000".to_string(),
            };

            let smart_contract_state = SmartContractState {
                smart_contract: "circulating_whitelist_v1".to_string(),
                instance: "1".to_string(),
            };

            smart_contracts::store_state(WHITELIST_SC.to_string(), "circulating_whitelist_v1_1".to_string());
            save_state(SMART_CONTRACT_KEY, &smart_contract_state);
            save_state(GOV_KEYS_KEY, &gov_keys);
        }
    }

    #[wasmedge_bindgen]
    pub fn execute(function: String, parameters: String) {
        unsafe{
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();
            let gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.update_key.to_string() {
                return;
            }

            let smart_contract_state : SmartContractState = load_state(SMART_CONTRACT_KEY).unwrap();

            let parameters_vec: Vec<String> = parameters.clone().split(",").map(|s| s.to_string()).collect();

            let results = smart_contracts::delegatecall(smart_contract_state.smart_contract.clone(), smart_contract_state.instance.clone(), function.clone(), parameters_vec.clone());

            for result in results {
                smart_contracts::emit(result.clone());
            }
        }
    
    }

    #[wasmedge_bindgen]
    pub fn update(smart_contract: String, instance: String) {
        unsafe{
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();
            let gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();
            if pub_key != gov_keys.update_key.to_string() {
                return;
            }

            let mut smart_contract_state : SmartContractState = load_state(SMART_CONTRACT_KEY).unwrap();
            smart_contract_state.smart_contract = smart_contract.clone();
            smart_contract_state.instance = instance.clone();

            save_state(SMART_CONTRACT_KEY, &smart_contract_state);

            let network_state = smart_contract_state.smart_contract.clone() + "_" + &smart_contract_state.instance.clone();
            smart_contracts::store_state(WHITELIST_SC.to_string(), network_state.to_string());

            let emit1 = format!("Success: Smart contract updated to {} with instance {}", smart_contract.clone(), instance.clone());
            smart_contracts::emit(emit1.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn send_all(wallet: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();
            let gov_keys : GovKeys= load_state(GOV_KEYS_KEY).unwrap();
            if pub_key != gov_keys.send_all_key.to_string() {
                return;
            }

            smart_contracts::send_all(wallet.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_update_key(update_key: String) {
        unsafe{
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let mut gov_keys : GovKeys= load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.update_key.to_string() {
                return;
            }

            gov_keys.update_key = update_key.clone();
            save_state(GOV_KEYS_KEY, &gov_keys);


            let emit1 = format!("Success: Update key updated to {}", update_key.clone());
            smart_contracts::emit(emit1.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_send_all_key(send_all_key: String) {
        unsafe{
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let mut gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.send_all_key.to_string() {
                return;
            }

            gov_keys.send_all_key = send_all_key.clone();
            save_state(GOV_KEYS_KEY, &gov_keys);


            let emit1 = format!("Success: Send all key updated to {}", send_all_key.clone());
            smart_contracts::emit(emit1.clone());
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
    }

    #[derive(Serialize, Deserialize)]
    pub struct GovKeys{
        pub update_key: String,
        pub send_all_key: String,
    }
}