pub mod staking_principle_v1 {
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::types::U256;


    const PROXY_WALLET: &str = "8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko";
    const TREASURY_WALLET: &str = "4Yg2ZeYrzMjVBXvU2YWtuZ7CzWR9atnQCD35TQj1kKcH";
    const MAIN_PROXY_WALLET: &str = "AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const AUTH_CONTRACT: &str = "gov_$ZRA+0000";

    fn check_auth() -> bool {
        unsafe {
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
            let sc_wallet = sc_wallet_.clone();

            if sc_wallet != PROXY_WALLET.to_string() {
                let emit1 = format!("Failed: Unauthorized sender key: {}", sc_wallet.clone());
                smart_contracts::emit(emit1.clone());
                return false;
            }
        }
        return true;
    }


    #[wasmedge_bindgen]
    pub fn init() {}

    #[wasmedge_bindgen]
    pub fn release_principle(amount: String) {
        unsafe {

            if !check_auth() {
                return;
            }

            if !types::is_valid_u256(amount.clone()) {
                smart_contracts::emit(format!("Failed: Invalid amount").to_string());
                return;
            }

            let amount_u256 : U256 = types::string_to_u256(amount.clone());

            if !smart_contracts::delegate_send(ZRA_CONTRACT.to_string(), amount_u256.to_string(), MAIN_PROXY_WALLET.to_string(), PROXY_WALLET.to_string())
            {
                smart_contracts::emit(format!("Failed").to_string());
            }
            else {
                smart_contracts::emit(format!("OK").to_string());
            }

        }
    }
}
