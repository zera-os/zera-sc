pub mod zra_v1 {
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;

    const PROXY_WALLET: &str = "6cifeAScHLGvxARJSdxS6QPdTJLwqgXMmHzXWyFU9tHC"; //sc_ace_proxy_1
    const ZRA_CONTRACT: &str = "$ZRA+0000";

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe{
            //1000000000000000000 = 1$ from get_ace_data
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("100000000000000000".to_string()); //0.1$
            let one_dolla_zera = (one_dolla * denomination) / rate;
            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string()); 

            smart_contracts::store_state(ZRA_CONTRACT.to_string(), "1000000000000000000".to_string());
        }
    }


    #[wasmedge_bindgen]
    pub fn update_rate(token: String, price: String) {
        unsafe{

            let sc_caller_wallet = smart_contracts::called_smart_contract_wallet();

            if sc_caller_wallet != PROXY_WALLET.to_string() {
                return;
            }

            smart_contracts::store_state(token.to_string(), price.to_string());
        }
    
    }
}