pub mod circulating_whitelist_v1 {
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
    const WHITE_LIST_KEY: &str = "WHITE_LIST";
    const WHITE_LIST_NETWORK_KEY: &str = "WHITE_LIST_NETWORK";
    const PROXY_WALLET: &str = "EMK16opdneub97v9qC4NdSkMTsnvHsRf4LrdZ2KH3cky"; //sc_circulating_supply_proxy_1

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe{
            //1000000000000000000 = 1$ from get_ace_data
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("500000000000000000".to_string()); //0.5$
            let one_dolla_zera = (one_dolla * denomination) / rate;
            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string());
            
            let mut white_list = WhiteList {
                wallets: HashSet::new(),
            };

            white_list.wallets.insert("W5jE3KNH".to_string()); // :fire: encoded
            white_list.wallets.insert("AZfFcttA3nwqmEYzAtsmufops7PaxLYavvkkDRsxTX5j".to_string()); //early backers proxy
            white_list.wallets.insert("AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8".to_string()); //staking proxy
            white_list.wallets.insert("8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko".to_string()); //principle proxy
            white_list.wallets.insert("4KjrhiQMoK999KxK3yjmuGw8LypoJDh1JzqcdmErG2NX".to_string()); //IIT gov wallet
            white_list.wallets.insert("4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH".to_string()); //treasury sc
            white_list.wallets.insert("3yygVMvY5DdRENZuM4J7NUXwiMhyfZE1nBfjnnodeHve".to_string()); //ZMT gov wallet

            save_state(WHITE_LIST_KEY, &white_list);

            let wallets_csv: String = white_list.wallets.iter()
                .cloned()
                .collect::<Vec<String>>()
                .join(",");

            smart_contracts::store_state(WHITE_LIST_NETWORK_KEY.to_string(), wallets_csv.to_string());

        }
    }

    #[wasmedge_bindgen]
    pub fn add_wallet(wallet: String) {
        unsafe{
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();

            if sc_wallet_ != PROXY_WALLET.to_string() {
                return;
            }

            let mut white_list: WhiteList = load_state(WHITE_LIST_KEY).unwrap();

            white_list.wallets.insert(wallet.clone());

            save_state(WHITE_LIST_KEY, &white_list);

            let wallets_csv: String = white_list.wallets.iter()
                .cloned()
                .collect::<Vec<String>>()
                .join(",");

            smart_contracts::store_state(WHITE_LIST_NETWORK_KEY.to_string(), wallets_csv.to_string());

        }
    }

    #[wasmedge_bindgen]
    pub fn remove_wallet(wallet: String) {
        unsafe {
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();

            if sc_wallet_ != PROXY_WALLET.to_string() {
                return;
            }

            let mut white_list : WhiteList = load_state(WHITE_LIST_KEY).unwrap();

            white_list.wallets.remove(&wallet.clone());

            save_state(WHITE_LIST_KEY, &white_list);

            let wallets_csv: String = white_list.wallets.iter()
                .cloned()
                .collect::<Vec<String>>()
                .join(",");

            smart_contracts::store_state(WHITE_LIST_NETWORK_KEY.to_string(), wallets_csv.to_string());
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
    pub struct WhiteList {
        pub wallets: HashSet<String>,
    }
}