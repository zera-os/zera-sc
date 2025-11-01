pub mod staking_v1 {
    use base64::{decode, encode};
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use native_functions::zera::wasmedge_bindgen;
    use postcard::{from_bytes, to_allocvec};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};
    use std::collections::HashMap;
    use std::fmt::Debug;

    const PRINCIPLE_PROXY_CONTRACT: &str = "staking_principle_proxy";
    const PRINCIPLE_PROXY_INSTANCE: &str = "1";
    const PRINCIPLE_PROXY_FUNCTION: &str = "execute";
    const PRINCIPLE_FUNCTION: &str = "release_principle";

    const PRINCIPLE_WALLET: &str = "8DABUMTHJtRXPTR4EkqHAYB6jW4XJy5F1YWNcFiSMDko";
    const PROXY_WALLET: &str = "AgYUDBYC7dmxyJRaLjrmPmopHexLuwz4zaDGCpK13Ls8"; //sc_staking_proxy_1
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const ID_BUMP_KEY: &str = "ID_BUMP_";
    const EARLY_STAKER_STATE_KEY: &str = "EARLY_STAKER_STATE_";
    const NORMAL_STAKER_STATE_KEY: &str = "NORMAL_STAKER_STATE_";
    const LIQUID_STAKER_STATE_KEY: &str = "LIQUID_STAKER_STATE_";
    const REWARD_MANAGER_STATE_KEY: &str = "REWARD_MANAGER_STATE_";
    const WALLET_STAKES_KEY: &str = "WALLET_STAKES_";
    const ALL_STAKERS_KEY: &str = "ALL_STAKERS_";
    const exploit_limit: u64 = 50_000_000_000_000; //50k ZRA
    
    enum StakingType {
        SIX_MONTHS,
        ONE_YEAR,
        TWO_YEARS,
        THREE_YEARS,
        FOUR_YEARS,
        FIVE_YEARS,
        LIQUID,
    }

    impl StakingType {
        fn as_str(&self) -> &str {
            match self {
                StakingType::SIX_MONTHS => "6_months",
                StakingType::ONE_YEAR => "1_year",
                StakingType::TWO_YEARS => "2_years",
                StakingType::THREE_YEARS => "3_years",
                StakingType::FOUR_YEARS => "4_years",
                StakingType::FIVE_YEARS => "5_years",
                StakingType::LIQUID => "liquid",
            }
        }
    }

    fn get_reward(staking_type: String, principle: u64) -> (u64, u64) {
        let mut total_reward: u64 = 0;
        let mut daily_release: u64 = 0;

        if staking_type == StakingType::SIX_MONTHS.as_str() {
            let yearly_reward: u64 = (principle * 2) / 100;
            total_reward = yearly_reward / 2;
            daily_release = total_reward / 182;
        } else if staking_type == StakingType::ONE_YEAR.as_str() {
            total_reward = (principle * 6) / 100;
            daily_release = total_reward / 365;
        } else if staking_type == StakingType::TWO_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 8) / 100;
            total_reward = yearly_reward * 2;
            daily_release = total_reward / 730;
        } else if staking_type == StakingType::THREE_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 7) / 100;
            total_reward = yearly_reward * 3;
            daily_release = total_reward / 1095;
        } else if staking_type == StakingType::FOUR_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 7) / 100;
            total_reward = yearly_reward * 4;
            daily_release = total_reward / 1460;
        } else if staking_type == StakingType::FIVE_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 7) / 100;
            total_reward = yearly_reward * 5;
            daily_release = total_reward / 1825;
        } else if staking_type == StakingType::LIQUID.as_str() {
            let yearly_reward: u64 = (principle * 1) / 1000;
            daily_release = yearly_reward / 365;
            total_reward = 0;
        } else {
            return (0, 0);
        }

        return (total_reward, daily_release);
    }

    fn check_auth() -> bool {
        unsafe {
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
            let sc_wallet = sc_wallet_.clone();

            if sc_wallet != PROXY_WALLET.to_string() {
                return false;
            }
        }
        return true;
    }

    fn process_early_staking(
        days_elapsed: u64,
        wallets_released_map: &mut HashMap<String, u64>,
        input_amount: &mut u64,
    ) -> (AllEarlyStakerStates) {
        unsafe {
            let staker_states: AllEarlyStakerStates = load_state(EARLY_STAKER_STATE_KEY).unwrap();

            let mut new_staker_states: AllEarlyStakerStates = AllEarlyStakerStates {
                staker_states: vec![],
            };

            for staker_state in &staker_states.staker_states {
                let daily_release: u64 = staker_state.daily_release.saturating_mul(days_elapsed);
                let total_released: u64 = staker_state.total_released.saturating_add(daily_release);

                if total_released > staker_state.total_reward {
                    let finish_release: u64 = staker_state
                        .total_reward
                        .saturating_sub(staker_state.total_released);
                    if finish_release > 0 {
                        wallets_released_map.insert(staker_state.staker_address.clone(), finish_release);
                        *input_amount += finish_release;
                    }
                    continue;
                }

                wallets_released_map.insert(staker_state.staker_address.clone(), daily_release);

                let new_staker_state = EarlyStakerState {
                    bump_id: staker_state.bump_id.clone(),
                    staker_address: staker_state.staker_address.clone(),
                    total_reward: staker_state.total_reward,
                    daily_release: staker_state.daily_release,
                    total_released: total_released,
                };

                *input_amount += daily_release;
                new_staker_states.staker_states.push(new_staker_state);
            }

            (new_staker_states)
        }
    }

    fn process_wallet_stakes(
        wallet_stake: AllWalletStakes,
        current_day: u64,
        allowed_liquid_release: u64,
    ) -> (AllWalletStakes, u64, u64, u64) {
        unsafe{
        let mut total_amount: u64 = 0;
        let mut principle_release: u64 = 0;

        let mut new_wallet_stakes: AllWalletStakes = AllWalletStakes {
            staker_states: HashMap::new(),
            liquid_stake: wallet_stake.liquid_stake,
        };

        for (id_str, stake) in &wallet_stake.staker_states {
            let mut new_stake = WalletStake {
                principle: stake.principle,
                total_reward: stake.total_reward,
                daily_release: stake.daily_release,
                total_released: stake.total_released,
                last_reward_day: stake.last_reward_day,
            };

            let day_diff: u64 = current_day.saturating_sub(new_stake.last_reward_day);

            if day_diff == 0 {
                new_wallet_stakes
                    .staker_states
                    .insert(id_str.clone(), new_stake);
                continue;
            }

            let daily_release: u64 = stake.daily_release.saturating_mul(day_diff);
            let total_released: u64 = stake.total_released.saturating_add(daily_release);

            if total_released > new_stake.total_reward {
                let finish_release: u64 = new_stake
                    .total_reward
                    .saturating_sub(new_stake.total_released);

                if finish_release > 0 {
                    total_amount += finish_release;
                }

                principle_release += new_stake.principle;
                continue;
            }

            total_amount += daily_release;
            new_stake.last_reward_day = current_day;
            new_stake.total_released = total_released;
            new_wallet_stakes
                .staker_states
                .insert(id_str.clone(), new_stake);
        }

        let mut liquid_release: u64 = 0;
        let day_diff: u64 =
            current_day.saturating_sub(new_wallet_stakes.liquid_stake.last_reward_day);

        if new_wallet_stakes.liquid_stake.bump_id != 0 && day_diff > 0 {
            let mut daily_release: u64 = new_wallet_stakes
                .liquid_stake
                .daily_release
                .saturating_mul(day_diff);
            let mut release_principle = false;
            if daily_release > allowed_liquid_release {
                daily_release = allowed_liquid_release;
                release_principle = true;
            }

            total_amount += daily_release;
            liquid_release += daily_release;
            new_wallet_stakes.liquid_stake.last_reward_day = current_day;

            if release_principle || new_wallet_stakes.liquid_stake.unstake_day <= current_day {
                principle_release += new_wallet_stakes.liquid_stake.principle;
                new_wallet_stakes.liquid_stake.bump_id = 0;
                new_wallet_stakes.liquid_stake.principle = 0;
                new_wallet_stakes.liquid_stake.last_reward_day = 0;
                new_wallet_stakes.liquid_stake.daily_release = 0;
                new_wallet_stakes.liquid_stake.unstake_day = 0;
            }
        }

        (
            new_wallet_stakes,
            total_amount,
            principle_release,
            liquid_release,
        )
    }
    }

    fn process_all_wallet_stakes(
        wallets_released_map: &mut HashMap<String, u64>,
        total_principle_amount: &mut u64,
        total_liquid_released: &mut u64,
        allowed_liquid_release: u64,
        input_amount: &mut u64,
        remove_wallet: &mut Vec<String>,
    ) -> (HashMap<String, AllWalletStakes>) {
        unsafe {
            let mut new_all_wallet_stakes: HashMap<String, AllWalletStakes> = HashMap::new();
            let all_wallet_stakes: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();
            let current_day: u64 = smart_contracts::last_block_time() / 86400;
            let mut allowed_release: u64 = allowed_liquid_release;

            //get all wallet stakes
            for (wallet_address, _) in &all_wallet_stakes.staker_states {
                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                let mut wallet_stakes: AllWalletStakes = match load_state(&wallet_stake_key) {
                    Ok(stakes) => stakes,
                    Err(_) => {
                        continue;
                    }
                };

                let (new_wallet_stakes, total_amount_released, principle_release, liquid_release) =
                    process_wallet_stakes(wallet_stakes, current_day, allowed_release);
                
                if total_amount_released > 0 {

                    let actual_amount = total_amount_released.saturating_add(principle_release);

                    if let Some(existing_amount) = wallets_released_map.get_mut(wallet_address) {
                        *existing_amount += actual_amount;
                    } else {
                        wallets_released_map.insert(wallet_address.clone(), actual_amount);
                    }
                    *input_amount += actual_amount;
                }
                if principle_release > 0 {
                    *total_principle_amount += principle_release;
                }

                if new_wallet_stakes.liquid_stake.bump_id != 0
                    || new_wallet_stakes.staker_states.len() > 0
                {
                    new_all_wallet_stakes.insert(wallet_address.clone(), new_wallet_stakes);
                } else {
                    remove_wallet.push(wallet_address.clone());
                }

                *total_liquid_released += liquid_release;
                allowed_release = allowed_release.saturating_sub(liquid_release);
            }
            (new_all_wallet_stakes)
        }
    }

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe {
            //1000000000000000000 = 1$ from get_ace_data
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("10000000000000000000".to_string()); //10$
            let one_dolla_zera = (one_dolla * denomination) / rate;
            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string());

            let staker_state = EarlyStakerState {
                bump_id: 1,
                staker_address: "78zRexAX5x5eeZQC1ubACexckAhHYj4tZ65KqNiPzrPx".to_string(),
                total_reward: 281_500_000,  //0.8445 ZRA
                daily_release: 77_123, //0.000077123 ZRA
                total_released: 0,
            };

            let staker_state2 = EarlyStakerState {
                bump_id: 2,
                staker_address: "7Ua66538h9tXfxjxuraBDxLMBTetJdvCffdixsX8TXp3".to_string(),
                total_reward: 499_999_718_500_000, //499,999.718500000 ZRA
                daily_release: 136_986_224_247,    //136.986224247 ZRA
                total_released: 0,
            };

            let staker_state3 = EarlyStakerState {
                bump_id: 3,
                staker_address: "Gi44BJYMcoZV3C2aq9qAuAaLPDPPknHgJr4BH1xAoYAD".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };

            let staker_state4 = EarlyStakerState {
                bump_id: 4,
                staker_address: "5unPPFqyqw3CsVc37ryFdHg3fBuhKkwkLpScg9MVWox3".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };

            let staker_state5 = EarlyStakerState {
                bump_id: 5,
                staker_address: "CsVwuWk9qPpdX63WiBGc1W1jzCYNBH4EL7yi21nvyN5i".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state6 = EarlyStakerState {
                bump_id: 6,
                staker_address: "6AyyJjviUxuX1TyAeLrZ36jwtVuLBMW44pEXRrQb1Vea".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state7 = EarlyStakerState {
                bump_id: 7,
                staker_address: "9JSFFjSoNim5rWTQHTEkeEmrf99wrc7TFqWxvp2LkWtF".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state8 = EarlyStakerState {
                bump_id: 8,
                staker_address: "DFJSJ7E87STGxrQDC7RJHezCTvHHmRqH47GeEABKc6xQ".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state9 = EarlyStakerState {
                bump_id: 9,
                staker_address: "3JFXob4qg1JBMLbFbp5SjnugHntNSYxtC45pd298AjTB".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state10 = EarlyStakerState {
                bump_id: 10,
                staker_address: "5sA7vapneGVcAibBwxuwuYg3FRo77Pf9u2hrUGteDG5m".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state11 = EarlyStakerState {
                bump_id: 11,
                staker_address: "6m3QMTYYkW9B8JVwKMxyYFqVngsRyPgRgm9yAR7R6woh".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state12 = EarlyStakerState {
                bump_id: 12,
                staker_address: "9XUYp3ge65XUrgHtwBqMBzQAZRHsNRNSd7T9AvzK7wEp".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state13 = EarlyStakerState {
                bump_id: 13,
                staker_address: "4HAsai4AosHJepYjYC8cPFKTy8tSB23AGgL5tyRxuZTJ".to_string(),
                total_reward: 500_000_000_000_000, //500,000.000000000 ZRA
                daily_release:  136_986_301_370, //136.986301370 ZRA
                total_released: 0,
            };
            let staker_state14 = EarlyStakerState {
                bump_id: 14,
                staker_address: "EcUvQZmmpcJmT53qpYf9xsBS5VZa7dhcsFSYuJQvBrg1".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state15 = EarlyStakerState {
                bump_id: 15,
                staker_address: "GNxvzat9VwAR5QoUXrKkefhUqKkcSjHPKpvamcPDytdQ".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state16 = EarlyStakerState {
                bump_id: 16,
                staker_address: "HWFQfKEZhQABU47CpL526HRo9fqwNY6mPwXJT1mcGXNN".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state17 = EarlyStakerState {
                bump_id: 17,
                staker_address: "64HK78NEqd35oSLdDrjoGaegUqjp6uLDD6APQK2AsFTY".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state18 = EarlyStakerState {
                bump_id: 18,
                staker_address: "4tHs1nRxZjoSxjFZSjxqtTNcwgRbiHo2ZcLdh8p71YRz".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state19 = EarlyStakerState {
                bump_id: 19,
                staker_address: "CrV4uLsan9DsKdrxSWia5L79cxgMvzn3fycw3VvtsNQe".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state20 = EarlyStakerState {
                bump_id: 20,
                staker_address: "84SFrLu1YXjZWHZg5NEuu52sm5mJn14Kmt4iWk5LULhR".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state21 = EarlyStakerState {
                bump_id: 21,
                staker_address: "Dk6ARnKukexf4PxaLPSyHRt54XYrH3uHAsvPAU5PxQXS".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state22 = EarlyStakerState {
                bump_id: 22,
                staker_address: "DaQeaGGzhvnZKFRSJxkt5k1FriVjsW1N646ktu7nEiMp".to_string(),
                total_reward: 1_000_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 273_972_602_740, //273.972602740 ZRA
                total_released: 0,
            };
            let staker_state23 = EarlyStakerState {
                bump_id: 23,
                staker_address: "B5iydRRGZ5higjKWMVGyxbGY56pQfvSHBST9jdRAiTgA".to_string(),
                total_reward: 2_500_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 684_931_506_849, //684.931506849 ZRA
                total_released: 0,
            };
            let staker_state24 = EarlyStakerState {
                bump_id: 24,
                staker_address: "2tjKdQyJhkn4YPCfRnW7j1R1FEyKsCByrC87hjngqSWs".to_string(),
                total_reward: 2_500_000_000_000_000, //500,000.000000000 ZRA
                daily_release: 684_931_506_849, //684.931506849 ZRA
                total_released: 0,
            };
            let staker_state25 = EarlyStakerState {
                bump_id: 25,
                staker_address: "93iQs6VkGWgyVcwyL28HHRVeqiZB5WHpvtCVkaXUnvoV".to_string(),
                total_reward: 5_000_000_000_000_000, //10,000,000.000000000 ZRA
                daily_release: 1_369_863_013_699, //1,369.863013699 ZRA
                total_released: 0,
            };

            let all_staker_state = AllEarlyStakerStates {
                staker_states: vec![staker_state, staker_state2, staker_state3, staker_state4, staker_state5, staker_state6, staker_state7, staker_state8, staker_state9, staker_state10, staker_state11, staker_state12, staker_state13, staker_state14, staker_state15, staker_state16, staker_state17, staker_state18, staker_state19, staker_state20, staker_state21, staker_state22, staker_state23, staker_state24, staker_state25],
            };

            save_state(EARLY_STAKER_STATE_KEY, &all_staker_state);

            let last_reward_time = smart_contracts::last_block_time();

            let last_reward_day: u64 = (last_reward_time / 86400);

            let reward_manager_state = RewardManagerState {
                last_reward_day: last_reward_day,
                total_supply: 40_000_000_000_000_000, //40m ZRA
                used_supply: 25_000_000_000_000_000,  //25m ZRA
                exploit: false,
            };

            let id_bump_state = IdBumpState { id: 26 };
            save_state(ID_BUMP_KEY, &id_bump_state);

            save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
        }
    }

    #[wasmedge_bindgen]
    pub fn process_rewards() {
        unsafe {
            if !check_auth() {
                return;
            }

            let current_day: u64 = smart_contracts::last_block_time() / 86400 as u64;
            let mut reward_manager_state: RewardManagerState = load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }

            if current_day <= reward_manager_state.last_reward_day || reward_manager_state.exploit {
                return;
            }

            let days_elapsed = current_day - reward_manager_state.last_reward_day;

            let mut wallets_released_map = HashMap::<String, u64>::new();

            let mut total_principle_amount: u64 = 0;
            let mut total_liquid_released: u64 = 0;
            let mut input_amount: u64 = 0;
            let mut remove_wallet = Vec::<String>::new();

            let (new_staker_states) = process_early_staking(
                days_elapsed,
                &mut wallets_released_map,
                &mut input_amount,
            );

            let allowed_liquid_release = reward_manager_state
                .total_supply
                .saturating_sub(reward_manager_state.used_supply);

            let (new_all_wallet_stakes) = process_all_wallet_stakes(
                &mut wallets_released_map,
                &mut total_principle_amount,
                &mut total_liquid_released,
                allowed_liquid_release,
                &mut input_amount,
                &mut remove_wallet,
            );

            let max_daily_check = input_amount.saturating_sub(total_principle_amount);

            let max_exploit = exploit_limit.saturating_mul(days_elapsed);
            if max_daily_check > max_exploit {
                reward_manager_state.exploit = true;
                save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
                let emit1 = format!("Failed: Exploit detected");
                smart_contracts::emit(emit1.clone());
                return;
            }

            reward_manager_state.used_supply = reward_manager_state
                .used_supply
                .saturating_add(total_liquid_released);

            if total_principle_amount > 0 {
                let parameters_vec: Vec<String> = [
                    PRINCIPLE_FUNCTION.to_string(),
                    total_principle_amount.to_string(),
                ]
                .to_vec();

                let results = smart_contracts::delegatecall(
                    PRINCIPLE_PROXY_CONTRACT.to_string(),
                    PRINCIPLE_PROXY_INSTANCE.to_string(),
                    PRINCIPLE_PROXY_FUNCTION.to_string(),
                    parameters_vec.clone(),
                );

                for result in results {
                    if result != "OK" {
                        let emit1 = format!("Failed to release principle");
                        smart_contracts::emit(emit1.clone());
                        return;
                    }
                }
            }
            let mut amounts_released = Vec::<String>::new();
            let mut wallets_released = Vec::<String>::new();

            for (wallet_address, amount) in &wallets_released_map {
                amounts_released.push(amount.to_string());
                wallets_released.push(wallet_address.clone());
            }

            if !smart_contracts::send_multi(
                ZRA_CONTRACT.to_string(),
                input_amount.to_string(),
                amounts_released,
                wallets_released,
            ) {
                if !smart_contracts::send(
                    ZRA_CONTRACT.to_string(),
                    total_principle_amount.to_string(),
                    PRINCIPLE_WALLET.to_string(),
                ) {
                    let emit1 = format!("Failed: Unable to send multi or refund principle");
                    smart_contracts::emit(emit1.clone());
                } else {
                    let emit1 = format!("Failed: Unable to send multi. Successfully refunded principle back to principle proxy.");
                    smart_contracts::emit(emit1.clone());
                }
                return;
            }

            let mut new_all_stakers: AllStakers = AllStakers {
                staker_states: HashMap::new(),
            };

            for (wallet_address, wallet_stakes) in &new_all_wallet_stakes {
                new_all_stakers
                    .staker_states
                    .insert(wallet_address.clone(), 1);
                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                save_state(&wallet_stake_key, &wallet_stakes);
            }

            for wallet_address in &remove_wallet {
                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                smart_contracts::clear_state(wallet_stake_key);
            }

            reward_manager_state.last_reward_day = current_day;
            save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
            save_state(EARLY_STAKER_STATE_KEY, &new_staker_states);
            save_state(ALL_STAKERS_KEY, &new_all_stakers);

            let emit1 = format!("Success: Rewards sent to stakers");
            smart_contracts::emit(emit1.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn stake(amount: String, wallet_address: String, staking_type: String) {
        unsafe {
            if !check_auth() {
                return;
            }

            let mut reward_manager_state: RewardManagerState = load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }

            if !types::is_valid_u256(amount.to_string()) {
                return;
            }

            let principle: u64 = amount.parse::<u64>().unwrap();

            let (total_reward, daily_release) = get_reward(staking_type.clone(), principle);

            if total_reward == 0 && daily_release == 0 {
                return;
            }

            let mut return_id = 0;

            if staking_type != StakingType::LIQUID.as_str() {
                let used_supply: u64 = reward_manager_state.used_supply + total_reward;

                if used_supply > reward_manager_state.total_supply {
                    let emit1 = format!("Failed: Insufficient supply");
                    smart_contracts::emit(emit1.clone());
                    return;
                }

                reward_manager_state.used_supply = used_supply;

                if !smart_contracts::transfer(
                    ZRA_CONTRACT.to_string(),
                    principle.to_string(),
                    PRINCIPLE_WALLET.to_string(),
                ) {
                    return;
                }

                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());

                let mut wallet_stakes: AllWalletStakes =
                    load_state(&wallet_stake_key).unwrap_or(AllWalletStakes {
                        staker_states: HashMap::new(),
                        liquid_stake: LiquidStake {
                            bump_id: 0,
                            principle: 0,
                            last_reward_day: 0,
                            daily_release: 0,
                            unstake_day: 0,
                        },
                    });

                let mut id_bump_state: IdBumpState = load_state(ID_BUMP_KEY).unwrap();

                let last_reward_time: u64 = (smart_contracts::last_block_time() / 86400);

                wallet_stakes.staker_states.insert(
                    id_bump_state.id.to_string(),
                    WalletStake {
                        principle: principle,
                        total_reward: total_reward,
                        daily_release: daily_release,
                        total_released: 0,
                        last_reward_day: last_reward_time,
                    },
                );

                return_id = id_bump_state.id;
                id_bump_state.id = id_bump_state.id + 1;

                let mut all_stakers: AllStakers =
                    load_state(ALL_STAKERS_KEY).unwrap_or(AllStakers {
                        staker_states: HashMap::new(),
                    });

                all_stakers.staker_states.insert(wallet_address.clone(), 1);

                save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
                save_state(ALL_STAKERS_KEY, &all_stakers);
                save_state(ID_BUMP_KEY, &id_bump_state);
                save_state(&wallet_stake_key, &wallet_stakes);
            } else {

                if !smart_contracts::transfer(
                    ZRA_CONTRACT.to_string(),
                    principle.to_string(),
                    PRINCIPLE_WALLET.to_string(),
                ) {
                    return;
                }


                let mut id_bump_state: IdBumpState = load_state(ID_BUMP_KEY).unwrap();

                let last_reward_time: u64 = (smart_contracts::last_block_time() / 86400);

                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());

                let mut wallet_stakes: AllWalletStakes = load_state(&wallet_stake_key.to_string())
                    .unwrap_or(AllWalletStakes {
                        staker_states: HashMap::new(),
                        liquid_stake: LiquidStake {
                            bump_id: 0,
                            principle: 0,
                            last_reward_day: 0,
                            daily_release: 0,
                            unstake_day: 0,
                        },
                    });

                wallet_stakes.liquid_stake.bump_id = id_bump_state.id;
                wallet_stakes.liquid_stake.principle += principle;
                wallet_stakes.liquid_stake.last_reward_day = last_reward_time;
                wallet_stakes.liquid_stake.daily_release += daily_release;
                wallet_stakes.liquid_stake.unstake_day = u64::MAX;

                let mut all_stakers: AllStakers =
                    load_state(ALL_STAKERS_KEY).unwrap_or(AllStakers {
                        staker_states: HashMap::new(),
                    });

                all_stakers.staker_states.insert(wallet_address.clone(), 1);
                return_id = id_bump_state.id;
                id_bump_state.id = id_bump_state.id + 1;
                save_state(ID_BUMP_KEY, &id_bump_state);
                save_state(&wallet_stake_key, &wallet_stakes);
                save_state(ALL_STAKERS_KEY, &all_stakers);
            }

            let emit1 = format!("Success: Wallet {} Staked {} with id {}", wallet_address.clone(), staking_type.clone(), return_id.to_string().clone());
            smart_contracts::emit(emit1.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn release_liquid_stake() {
        unsafe {
            if !check_auth() {
                return;
            }

            let mut reward_manager_state: RewardManagerState = load_state(REWARD_MANAGER_STATE_KEY).unwrap();
            
            if reward_manager_state.exploit {
                return;
            }

            let sender_wallet = smart_contracts::wallet_address();
            let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, sender_wallet.clone());

            let mut wallet_stakes: AllWalletStakes = match load_state(&wallet_stake_key) {
                Ok(stakes) => stakes,
                Err(_) => {
                    return;
                }
            };

            if wallet_stakes.liquid_stake.bump_id == 0 {
                let emit1 = format!(
                    "Failed: No liquid stake found for wallet: {}",
                    sender_wallet.clone()
                );
                smart_contracts::emit(emit1.clone());
                return;
            }

            let unstake_day: u64 = (smart_contracts::last_block_time() / 86400) + 14;

            wallet_stakes.liquid_stake.unstake_day = unstake_day;

            save_state(&wallet_stake_key, &wallet_stakes);

            let emit1 = format!(
                "Success: Liquid stake will be released in 14 days for wallet: {}",
                sender_wallet.clone()
            );
            smart_contracts::emit(emit1.clone());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_wallet(wallet_address: String, bump_id: String) {
        unsafe {
            if !check_auth() {
                return;
            }

            if wallet_address == "" {
                return;
            }

            let mut reward_manager_state: RewardManagerState = load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }

            let sender_wallet = smart_contracts::wallet_address();

            let mut staker_states: AllEarlyStakerStates =
                load_state(EARLY_STAKER_STATE_KEY).unwrap();

            // Check if the new wallet_address already exists
            if staker_states
                .staker_states
                .iter()
                .any(|s| s.staker_address == wallet_address)
            {
                return;
            }

            // Update the early staker state
            if let Some(s) = staker_states.staker_states.iter_mut().find(|s| {
                s.staker_address == sender_wallet && s.bump_id == bump_id.parse::<u64>().unwrap()
            }) {
                s.staker_address = wallet_address.clone();
                save_state(EARLY_STAKER_STATE_KEY, &staker_states);
                let emit1 = format!(
                    "Success: Early Backer wallet updated from {} to {}",
                    sender_wallet.clone(),
                    wallet_address.clone()
                );
                smart_contracts::emit(emit1.clone());
                return;
            }

            // Get the wallet stakes
            let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, sender_wallet.clone());
            let mut wallet_stakes: AllWalletStakes = match load_state(&wallet_stake_key) {
                Ok(stakes) => stakes,
                Err(_) => {
                    return;
                }
            };

            // Check if the stake exists
            if let Some(stake) = wallet_stakes.staker_states.get(&bump_id.to_string()) {
                // stake exists, use it
                let updated_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                let mut updated_wallet_stakes: AllWalletStakes = load_state(&updated_stake_key)
                    .unwrap_or(AllWalletStakes {
                        staker_states: HashMap::new(),
                        liquid_stake: LiquidStake {
                            bump_id: 0,
                            principle: 0,
                            last_reward_day: 0,
                            daily_release: 0,
                            unstake_day: 0,
                        },
                    });

                let mut all_stakers: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();

                updated_wallet_stakes
                    .staker_states
                    .insert(bump_id.to_string(), stake.clone());
                wallet_stakes.staker_states.remove(&bump_id.to_string());

                if wallet_stakes.liquid_stake.bump_id == 0 && wallet_stakes.staker_states.is_empty()
                {
                    all_stakers.staker_states.remove(&sender_wallet.clone());
                    smart_contracts::clear_state(wallet_stake_key.clone());
                }

                all_stakers.staker_states.insert(wallet_address.clone(), 1);

                save_state(ALL_STAKERS_KEY, &all_stakers);
                save_state(&updated_stake_key, &updated_wallet_stakes);
                save_state(&wallet_stake_key, &wallet_stakes);
                let emit1 = format!(
                    "Success: Stake id {} updated for wallet: {}",
                    bump_id.clone(),
                    wallet_address.clone()
                );
                smart_contracts::emit(emit1.clone());
                return;
            } else {
                if wallet_stakes.liquid_stake.bump_id == bump_id.parse::<u64>().unwrap() {
                    let updated_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                    let mut updated_wallet_stakes: AllWalletStakes = load_state(&updated_stake_key)
                        .unwrap_or(AllWalletStakes {
                            staker_states: HashMap::new(),
                            liquid_stake: LiquidStake {
                                bump_id: 0,
                                principle: 0,
                                last_reward_day: 0,
                                daily_release: 0,
                                unstake_day: 0,
                            },
                        });

                    updated_wallet_stakes.liquid_stake.bump_id = bump_id.parse::<u64>().unwrap();
                    updated_wallet_stakes.liquid_stake.principle += wallet_stakes.liquid_stake.principle;
                    updated_wallet_stakes.liquid_stake.last_reward_day = smart_contracts::last_block_time() / 86400;
                    updated_wallet_stakes.liquid_stake.daily_release += wallet_stakes.liquid_stake.daily_release;
                    updated_wallet_stakes.liquid_stake.unstake_day = u64::MAX;


                    wallet_stakes.liquid_stake.bump_id = 0;
                    wallet_stakes.liquid_stake.principle = 0;
                    wallet_stakes.liquid_stake.last_reward_day = 0;
                    wallet_stakes.liquid_stake.daily_release = 0;
                    wallet_stakes.liquid_stake.unstake_day = 0;

                    let mut all_stakers: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();
                    
                    if wallet_stakes.staker_states.is_empty() && wallet_stakes.liquid_stake.bump_id == 0{
                        all_stakers.staker_states.remove(&sender_wallet.clone());
                        smart_contracts::clear_state(wallet_stake_key.clone());
                    }

                    all_stakers.staker_states.insert(wallet_address.clone(), 1);
                    save_state(ALL_STAKERS_KEY, &all_stakers);
                    save_state(&updated_stake_key, &updated_wallet_stakes);
                    save_state(&wallet_stake_key, &wallet_stakes);
                    let emit1 = format!(
                        "Success: Liquid stake updated from {} to {}",
                        sender_wallet.clone(),
                        wallet_address.clone()
                    );
                    smart_contracts::emit(emit1.clone());
                    return;
                } else {
                    return;
                }
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

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllEarlyStakerStates {
        pub staker_states: Vec<EarlyStakerState>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct EarlyStakerState {
        pub bump_id: u64,
        pub staker_address: String,
        pub total_reward: u64,
        pub daily_release: u64,
        pub total_released: u64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllStakers {
        pub staker_states: HashMap<String, u8>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllWalletStakes {
        pub staker_states: HashMap<String, WalletStake>,
        pub liquid_stake: LiquidStake,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct LiquidStake {
        pub bump_id: u64,
        pub principle: u64,
        pub last_reward_day: u64,
        pub daily_release: u64,
        pub unstake_day: u64,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WalletStake {
        pub principle: u64,
        pub total_reward: u64,
        pub daily_release: u64,
        pub total_released: u64,
        pub last_reward_day: u64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RewardManagerState {
        pub total_supply: u64,
        pub used_supply: u64,
        pub last_reward_day: u64,
        pub exploit: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct IdBumpState {
        pub id: u64,
    }
}
