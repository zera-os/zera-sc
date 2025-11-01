pub mod restricted_symbols_v1 {
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::smart_contracts;
    use serde::{Deserialize, Serialize};
    use serde::de::DeserializeOwned;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use base64::{encode, decode};
    use postcard::{to_allocvec, from_bytes};
    use std::collections::HashSet;

    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const PROXY_WALLET: &str = "H7YTw7bry3VQVmAADF3tQw4eGYUMuenac3rbNb7r5SZA"; //sc_restricted_symbols_proxy_1

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe{
            //1000000000000000000 = 1$ from get_ace_data
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("500000000000000000".to_string()); //0.5$
            let one_dolla_zera = (one_dolla * denomination) / rate;
            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string());
            
            smart_contracts::store_state("ZRA".to_string(), "true".to_string());
            smart_contracts::store_state("ACE".to_string(), "true".to_string());
            smart_contracts::store_state("ZIP".to_string(), "true".to_string());
            smart_contracts::store_state("LEGAL".to_string(), "true".to_string());
            smart_contracts::store_state("TREASURY".to_string(), "true".to_string());
            smart_contracts::store_state("IIT".to_string(), "true".to_string());
            smart_contracts::store_state("ZMT".to_string(), "true".to_string());
            smart_contracts::store_state("BRIDGEGUARDIAN".to_string(), "true".to_string());
            smart_contracts::store_state("BRIDGETOKENS".to_string(), "true".to_string());
        }
    }

    #[wasmedge_bindgen]
    pub fn add_symbol(symbol: String) {
        unsafe{
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();

            if sc_wallet_ != PROXY_WALLET.to_string() {
                return;
            }

            smart_contracts::store_state(symbol.clone(), "true".to_string());
        }
    }

    #[wasmedge_bindgen]
    pub fn remove_symbol(symbol: String) {
        unsafe {
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();

            if sc_wallet_ != PROXY_WALLET.to_string() {
                return;
            }

            smart_contracts::clear_state(symbol.clone());
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
    pub struct RestrictedSymbols {
        pub symbols: HashSet<String>,
    }
}