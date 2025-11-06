pub mod bridge_gov_v1 {
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::smart_contracts;

    const GOV_CONTRACT: &str = "gov_$BRIDGEGUARDIAN+0000";
    const PROXY_WALLET: &str = "9fTYjLqHDqCmb1U71a6kRXEYNMwNvTF9xYX48HG4d1WA";

    fn check_auth() -> bool {
        unsafe{
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
            let sc_wallet = sc_wallet_.clone();

            if sc_wallet != PROXY_WALLET.to_string() {
                let emit1 = format!("Failed: Unauthorized sender key: {}", sc_wallet.clone());
                smart_contracts::emit(emit1.clone());
                return false;
            }

            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            if(pub_key != GOV_CONTRACT.to_string())
            {
                let emit1 = format!("Failed: Unauthorized sender key: {}", pub_key.clone());
                smart_contracts::emit(emit1.clone());
                return false;
            }
        }
        return true;
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


            if(pause_level != "0" && pause_level != "1" && pause_level != "2") 
            {
                return;
            }

            if(pause_duration != "0")
            {
                if(pause_duration.parse::<u64>().is_err())
                {
                    smart_contracts::emit("Failed: Invalid pause duration".to_string());
                    return;
                }
            }
            else
            {
                pause_duration = "0".to_string();
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

            smart_contracts::emit("EVENT:UPDATE_GUARDIAN_KEYS".to_string());
            smart_contracts::emit(format!("guardian_keys: {}", guardian_keys.clone()));
            smart_contracts::emit(format!("threshold: {}", threshold.clone()));
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
                let emit1 = format!("Failed: Invalid limit: {}", limit.clone());
                smart_contracts::emit(emit1.clone());
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
                let emit1 = format!("Failed: Invalid rate limit: {}", limit.clone());
                smart_contracts::emit(emit1.clone());
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

}