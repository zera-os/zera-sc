pub mod zera_treasury_v1 {
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::smart_contracts;

    #[wasmedge_bindgen]
    pub fn init() {
    }

    #[wasmedge_bindgen]
    pub fn send(contract_id: String, amount: String, wallet_address: String) {
        unsafe{
            let pub_key_ = smart_contracts::public_key();
             let pub_key = pub_key_.clone();

            if pub_key != "gov_$TREASURY+0000" && pub_key != "gov_$ZRA+0000" && pub_key != "gov_$IIT+0000" && pub_key != "gov_$ZMT+0000" && pub_key != "gov_$ZIP+0000"{
                let emit1 = format!("Failed: Unauthorized sender key: {}", pub_key.clone());
                smart_contracts::emit(emit1.clone());
                return;
            }

            smart_contracts::send(contract_id, amount, wallet_address);
        }
    }

    #[wasmedge_bindgen]
    pub fn send_all(wallet_address: String){
        unsafe{
             let pub_key_ = smart_contracts::public_key();
             let pub_key = pub_key_.clone();

            if pub_key != "gov_$TREASURY+0000" && pub_key != "gov_$ZRA+0000"{
                let emit1 = format!("Failed: Unauthorized sender key: {}", pub_key.clone());
                smart_contracts::emit(emit1.clone());
                return;
            }

            smart_contracts::send_all(wallet_address.clone());
        }
    }
}