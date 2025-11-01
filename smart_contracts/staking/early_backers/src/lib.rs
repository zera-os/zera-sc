pub mod release_v1 {
    use native_functions::zera::wasmedge_bindgen;
    use native_functions::zera::smart_contracts;
    use serde::{Deserialize, Serialize};
    use serde::de::DeserializeOwned;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use base64::{encode, decode};
    use postcard::{to_allocvec, from_bytes};
    use std::fmt::Debug;

    const PROXY_WALLET: &str = "AZfFcttA3nwqmEYzAtsmufops7PaxLYavvkkDRsxTX5j"; //sc_release_proxy_1
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const STAKER_STATE_KEY: &str = "STAKER_STATE_";
    const REWARD_MANAGER_STATE_KEY: &str = "REWARD_MANAGER_STATE_";
    const EXPLOIT_LIMIT: u64 = 14_000_000_000_000; //14k ZRA

    fn check_auth() -> bool {
        unsafe{
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
    pub fn init() {
        unsafe{
            //1000000000000000000 = 1$ from get_ace_data
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("100000000000000000000".to_string()); //100$
            let one_dolla_zera = (one_dolla * denomination) / rate;
            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string());
            
            
            let staker_state = StakerState {
                staker_address: "78zRexAX5x5eeZQC1ubACexckAhHYj4tZ65KqNiPzrPx".to_string(),
                principle: 563_000_000,  //1k ZRA
                daily_release: 154_247, //0.0154247 ZRA
                total_released: 0,
            };

            let staker_state2 = StakerState {
                staker_address: "7Ua66538h9tXfxjxuraBDxLMBTetJdvCffdixsX8TXp3".to_string(),
                principle: 999_999_437_000_000, //999,999.437 ZRA
                daily_release: 273_972_448_493, //273.972448493 ZRA
                total_released: 0,
            };

            let staker_state3 = StakerState {
                staker_address: "Gi44BJYMcoZV3C2aq9qAuAaLPDPPknHgJr4BH1xAoYAD".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };

            let staker_state4 = StakerState {
                staker_address: "5unPPFqyqw3CsVc37ryFdHg3fBuhKkwkLpScg9MVWox3".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };

            let staker_state5 = StakerState {
                staker_address: "CsVwuWk9qPpdX63WiBGc1W1jzCYNBH4EL7yi21nvyN5i".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state6 = StakerState {
                staker_address: "6AyyJjviUxuX1TyAeLrZ36jwtVuLBMW44pEXRrQb1Vea".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state7 = StakerState {
                staker_address: "9JSFFjSoNim5rWTQHTEkeEmrf99wrc7TFqWxvp2LkWtF".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state8 = StakerState {
                staker_address: "DFJSJ7E87STGxrQDC7RJHezCTvHHmRqH47GeEABKc6xQ".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state9 = StakerState {
                staker_address: "3JFXob4qg1JBMLbFbp5SjnugHntNSYxtC45pd298AjTB".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state10 = StakerState {
                staker_address: "5sA7vapneGVcAibBwxuwuYg3FRo77Pf9u2hrUGteDG5m".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state11 = StakerState {
                staker_address: "6m3QMTYYkW9B8JVwKMxyYFqVngsRyPgRgm9yAR7R6woh".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state12 = StakerState {
                staker_address: "9XUYp3ge65XUrgHtwBqMBzQAZRHsNRNSd7T9AvzK7wEp".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state13 = StakerState {
                staker_address: "4HAsai4AosHJepYjYC8cPFKTy8tSB23AGgL5tyRxuZTJ".to_string(),
                principle: 1_000_000_000_000_000, //1,000,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state14 = StakerState {
                staker_address: "EcUvQZmmpcJmT53qpYf9xsBS5VZa7dhcsFSYuJQvBrg1".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release:  547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state15 = StakerState {
                staker_address: "GNxvzat9VwAR5QoUXrKkefhUqKkcSjHPKpvamcPDytdQ".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state16 = StakerState {
                staker_address: "HWFQfKEZhQABU47CpL526HRo9fqwNY6mPwXJT1mcGXNN".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state17 = StakerState {
                staker_address: "64HK78NEqd35oSLdDrjoGaegUqjp6uLDD6APQK2AsFTY".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state18 = StakerState {
                staker_address: "4tHs1nRxZjoSxjFZSjxqtTNcwgRbiHo2ZcLdh8p71YRz".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state19 = StakerState {
                staker_address: "CrV4uLsan9DsKdrxSWia5L79cxgMvzn3fycw3VvtsNQe".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state20 = StakerState {
                staker_address: "84SFrLu1YXjZWHZg5NEuu52sm5mJn14Kmt4iWk5LULhR".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state21 = StakerState {
                staker_address: "Dk6ARnKukexf4PxaLPSyHRt54XYrH3uHAsvPAU5PxQXS".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state22 = StakerState {
                staker_address: "DaQeaGGzhvnZKFRSJxkt5k1FriVjsW1N646ktu7nEiMp".to_string(),
                principle: 2_000_000_000_000_000, //2,000,000.000000000 ZRA
                daily_release: 547_945_205_479, //547.945205479 ZRA
                total_released: 0,
            };
            let staker_state23 = StakerState {
                staker_address: "B5iydRRGZ5higjKWMVGyxbGY56pQfvSHBST9jdRAiTgA".to_string(),
                principle: 5_000_000_000_000_000, //5,000,000.000000000 ZRA
                daily_release:  1_369_863_013_699, //1,369.863013699 ZRA
                total_released: 0,
            };
            let staker_state24 = StakerState {
                staker_address: "2tjKdQyJhkn4YPCfRnW7j1R1FEyKsCByrC87hjngqSWs".to_string(),
                principle: 5_000_000_000_000_000, //5,000,000.000000000 ZRA
                daily_release: 1_369_863_013_699, //1,369.863013699 ZRA
                total_released: 0,
            };
            let staker_state25 = StakerState {
                staker_address: "93iQs6VkGWgyVcwyL28HHRVeqiZB5WHpvtCVkaXUnvoV".to_string(),
                principle: 10_000_000_000_000_000, //10,000,000.000000000 ZRA
                daily_release: 2_739_726_027_397, //2,739.726027397 ZRA
                total_released: 0,
            };

            let all_staker_state = AllStakerStates {
                staker_states: vec![staker_state, staker_state2, staker_state3, staker_state4, staker_state5, staker_state6, staker_state7, staker_state8, staker_state9, staker_state10, staker_state11, staker_state12, staker_state13, staker_state14, staker_state15, staker_state16, staker_state17, staker_state18, staker_state19, staker_state20, staker_state21, staker_state22, staker_state23, staker_state24, staker_state25],
            };

            save_state(STAKER_STATE_KEY, &all_staker_state);

            let last_reward_time = smart_contracts::last_block_time();

            let last_reward_day : u64 = (last_reward_time / 86400);

            let reward_manager_state = RewardManagerState {
                last_reward_day: last_reward_day,
                exploit: false,
            };

            save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
        }
    }


    #[wasmedge_bindgen]
    pub fn process_rewards() {
        unsafe{

            if !check_auth() {
                return;
            }

            let current_day : u64 = smart_contracts::last_block_time() / 86400 as u64;
            let mut reward_manager_state : RewardManagerState = load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if current_day <= reward_manager_state.last_reward_day || reward_manager_state.exploit {
                return;
            }

            let mut staker_states : AllStakerStates = load_state(STAKER_STATE_KEY).unwrap();

            let days_elapsed = current_day - reward_manager_state.last_reward_day;

            let mut new_staker_states : AllStakerStates = AllStakerStates {
                staker_states: vec![],
            };

            let mut amounts_released = Vec::<String>::new();
            let mut wallets_released = Vec::<String>::new();
            let mut input_amount = U256::from(0);

                for staker_state in staker_states.staker_states {

                    let daily_release : u64 = staker_state.daily_release * days_elapsed;
                    let total_released : u64 = staker_state.total_released + daily_release;
   
                    if total_released > staker_state.principle {
                        let finish_release : u64 = staker_state.principle - staker_state.total_released;

                        if(finish_release > 0)
                        {
                            amounts_released.push(finish_release.to_string());
                            wallets_released.push(staker_state.staker_address.clone());
                            input_amount += U256::from(finish_release);
                        }
                        continue;
                    }

                    amounts_released.push(daily_release.to_string());
                    wallets_released.push(staker_state.staker_address.clone());

                    let new_staker_state = StakerState {
                        staker_address: staker_state.staker_address.clone(),
                        principle: staker_state.principle.clone(),
                        daily_release: staker_state.daily_release.clone(),
                        total_released: total_released,
                    };

                    input_amount += U256::from(daily_release);
                    new_staker_states.staker_states.push(new_staker_state);
                }
            
            let exploit_limit : U256 = U256::from(EXPLOIT_LIMIT);

            let limit_days : U256 = exploit_limit * U256::from(days_elapsed);
            
            if input_amount >= limit_days {
                reward_manager_state.exploit = true;
                save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
                let emit1 = format!("Failed: Exploit detected");
                smart_contracts::emit(emit1.clone());
                return;
            }

            if !smart_contracts::send_multi(ZRA_CONTRACT.to_string(), input_amount.to_string(), amounts_released, wallets_released) {
                let emit1 = format!("Failed to multi_send");
                smart_contracts::emit(emit1.clone());
                return;
            }

            reward_manager_state.last_reward_day = current_day;
            save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
            save_state(STAKER_STATE_KEY, &new_staker_states);    

            let emit1 = format!("Success: Processed rewards for {} days", days_elapsed);
            smart_contracts::emit(emit1.clone());
        }
    
    }

    #[wasmedge_bindgen]
    pub fn update_wallet(wallet_address: String) {
        unsafe{
            if !check_auth() {
                return;
            }
            let mut reward_manager_state: RewardManagerState = load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }

            if wallet_address == "" {
                return;
            }

            let sender_wallet = smart_contracts::wallet_address();

            let mut staker_states : AllStakerStates = load_state(STAKER_STATE_KEY).unwrap();

            if let Some(s) = staker_states
                .staker_states
                .iter_mut()
                .find(|s| s.staker_address == sender_wallet)
            {
                s.staker_address = wallet_address.clone();
                save_state(STAKER_STATE_KEY, &staker_states);
                let emit1 = format!("Success: Wallet updated");
                smart_contracts::emit(emit1.clone());
            } 
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
    pub struct AllStakerStates {
        pub staker_states: Vec<StakerState>,
    }

    #[derive(Serialize, Deserialize)]
    pub struct StakerState {
        pub staker_address: String,
        pub principle: u64,
        pub daily_release: u64,
        pub total_released: u64,
    }

    #[derive(Serialize, Deserialize)]
    pub struct RewardManagerState {
        pub last_reward_day: u64,
        pub exploit: bool,
    }
}