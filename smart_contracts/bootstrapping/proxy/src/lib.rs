pub mod bootstrapping_proxy {
    use base64::{decode, encode};
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use native_functions::zera::wasmedge_bindgen;
    use postcard::{from_bytes, to_allocvec};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    //ADDRESS: 2nuEvMULK77BCZPyLLThtUn9kvkJkjsSyky7Nb67FMC1
    const SMART_CONTRACT_KEY: &str = "SMART_CONTRACT_";
    const GOV_KEYS_KEY: &str = "GOV_KEYS_";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const TREASURE_WALLET: &str = "4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH";
    const BOOT_MANAGER_KEY: &str = "BOOT_MANAGER";


    #[wasmedge_bindgen]
    pub fn init() {
        unsafe {
            let gov_keys = GovKeys {
                update_key: "A_c_7kdro2TEbd7QZQNRNmjzHLnAVb366KzUNrF761rnN7uz".to_string(),
                send_all_key: "gov_$ZRA+0000".to_string(),
            };

            let smart_contract_state = SmartContractState {
                smart_contract: "bootstrapping".to_string(),
                instance: "0".to_string(),
            };
            let mut boot_manager: BootstrappingManager = BootstrappingManager {
                last_reward_day: 0,
                exploit: false,
            };

            save_state(BOOT_MANAGER_KEY, &boot_manager);
            save_state(SMART_CONTRACT_KEY, &smart_contract_state);
            save_state(GOV_KEYS_KEY, &gov_keys);
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
    pub fn update(smart_contract: String, instance: String) {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();
            if pub_key != gov_keys.update_key.to_string() {
                return;
            }

            let mut smart_contract_state: SmartContractState =
                load_state(SMART_CONTRACT_KEY).unwrap();
            smart_contract_state.smart_contract = smart_contract.clone();
            smart_contract_state.instance = instance.clone();

            save_state(SMART_CONTRACT_KEY, &smart_contract_state);

            let emit1 = format!("Success: Smart contract updated to {} with instance {}", smart_contract.clone(), instance.clone());
            smart_contracts::emit(emit1.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_key(key: String) {
        unsafe{
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let mut gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.update_key.to_string() {
                return;
            }

            gov_keys.update_key = key.clone();
            save_state(GOV_KEYS_KEY, &gov_keys);


            let emit1 = format!("Success: Update key updated to {}", key.clone());
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

    #[wasmedge_bindgen]
    pub fn send_all() {
        unsafe {
            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();
            let gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();
            if pub_key != gov_keys.send_all_key.to_string() {
                return;
            }

            smart_contracts::send_all(TREASURE_WALLET.to_string());
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

    #[derive(Serialize, Deserialize, Debug)]
    pub struct BootstrappingManager {
        pub last_reward_day: u64,
        pub exploit: bool,
    }
}
