pub mod bootstrapping_v1 {
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

    //86400 seconds in a day
    const start_date2 :u64 = 2592000; //done 2026-03-03
    const start_date3 :u64 = 6566400; //done 2026-04-18
    const start_date4 :u64 = 12700800; //done 2026-06-28
    const start_date5 :u64 = 22204800; //done 2026-10-16
    const start_date6 :u64 = 36892800; //done 2027-04-04
    const start_date7 :u64 = 59616000; //done 2027-12-23
    const start_date8 :u64 = 94780800; //done 2029-02-02
    const start_date9 :u64 = 149212800; //done 2030-10-25
    const start_date10 :u64 = 233539200; //done 2033-06-27
    const end_date :u64 = 364176000; //done 2037-08-17

    const release1 :u64 = 23333333333333;
    const release2 :u64 = 15217391304348;
    const release3 :u64 = 9859154929577;
    const release4 :u64 = 6363636363636;
    const release5 :u64 = 4117647058824;
    const release6 :u64 = 2661596958175;
    const release7 :u64 = 1719901719902;
    const release8 :u64 = 1111111111111;
    const release9 :u64 = 717213114754;
    const release10 :u64 = 462962962963;

    const sol_multi :u64 = 31623;
    const sol_multi_scale :u64 = 1000;

    const SOLANA_LP_KEY: &str = "SOLANA_LP";
    const ZERA_LP_TOKEN: &str = "$dex-ZRA25sol-USDC+0000000000";
    const BOOT_MANAGER_KEY: &str = "BOOT_MANAGER";
    const ID_BUMP_KEY: &str = "ID_BUMP_";
    const PROXY_CONTRACT: &str = "bootstrapping_proxy_1";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const PRINCIPLE_SEED: &str = "principle";
    const WALLET_STAKES_KEY: &str = "WALLET_STAKES_";
    const ALL_STAKERS_KEY: &str = "ALL_STAKERS_";
    const PROXY_WALLET: &str = "2nuEvMULK77BCZPyLLThtUn9kvkJkjsSyky7Nb67FMC1"; //sc_bootstrapping_proxy_1
    const SOLANA_LP_TOKEN: &str = "$sol-8miyE+000000";
    const RELEASE_DAYS_KEY: &str = "RELEASE_DAYS";
    const SOL_TOKEN_KEY: &str = "SOL_TOKEN";
    const GOV_KEYS_KEY: &str = "GOV_KEYS_";


    const ONE_TIME_RELEASE_KEY: &str = "ONE_TIME_RELEASE";

    enum TermBooster{
        THIRTY_DAYS,
        NINETY_DAYS,
        SIX_MONTHS,
        ONE_YEAR,
        TWO_YEARS,
        THREE_YEARS,
        FOUR_YEARS,
        FIVE_YEARS,
        SIX_YEARS,
        SEVEN_YEARS,
    }

    impl TermBooster {
        fn as_str(&self) -> &str {
            match self {
                TermBooster::THIRTY_DAYS => "30_days",
                TermBooster::NINETY_DAYS => "90_days",
                TermBooster::SIX_MONTHS => "6_months",
                TermBooster::ONE_YEAR => "1_year",
                TermBooster::TWO_YEARS => "2_years",
                TermBooster::THREE_YEARS => "3_years",
                TermBooster::FOUR_YEARS => "4_years",
                TermBooster::FIVE_YEARS => "5_years",
                TermBooster::SIX_YEARS => "6_years",
                TermBooster::SEVEN_YEARS => "7_years",
            }
        }
    }

    fn get_start_term_and_release(current_timestamp: u64) -> (u64, U256) {

        let release_days: ReleaseDays = unsafe {
            match load_state::<ReleaseDays>("RELEASE_DAYS") {
                Ok(existing) => existing,
                Err(_) => {
                    let new_release_days = ReleaseDays {
                        date1: current_timestamp,
                        date2: current_timestamp + start_date2,
                        date3: current_timestamp + start_date3,
                        date4: current_timestamp + start_date4,
                        date5: current_timestamp + start_date5,
                        date6: current_timestamp + start_date6,
                        date7: current_timestamp + start_date7,
                        date8: current_timestamp + start_date8,
                        date9: current_timestamp + start_date9,
                        date10: current_timestamp + start_date10,
                        end_date: current_timestamp + end_date,
                    };
                    save_state(RELEASE_DAYS_KEY, &new_release_days);
                    new_release_days
                }
            }
        };


        if current_timestamp < release_days.date2{ //done 2026-03-03
            return (1, U256::from(release1));
        } else if current_timestamp >= release_days.date2 && current_timestamp < release_days.date3 { 
            return (2, U256::from(release2));
        } else if current_timestamp >= release_days.date3 && current_timestamp < release_days.date4 {
            return (3, U256::from(release3));
        } else if current_timestamp >= release_days.date4 && current_timestamp < release_days.date5 {
            return (4, U256::from(release4));
        } else if current_timestamp >= release_days.date5 && current_timestamp < release_days.date6 { 
            return (5, U256::from(release5));
        } else if current_timestamp >= release_days.date6 && current_timestamp < release_days.date7 { 
            return (6, U256::from(release6));
        } else if current_timestamp >= release_days.date7 && current_timestamp < release_days.date8 {
            return (7, U256::from(release7));
        } else if current_timestamp >= release_days.date8 && current_timestamp < release_days.date9 {
            return (8, U256::from(release8));
        } else if current_timestamp >= release_days.date9 && current_timestamp < release_days.date10 {
            return (9, U256::from(release9));
        } else if current_timestamp >= release_days.date10 && current_timestamp < release_days.end_date {
            return (10, U256::from(release10));
        }
        else {
            return (0, U256::zero());
        }
    }

    fn get_max_exploit(days_elapsed: u64, current_day: u64) -> U256 {
        let mut max_exploit = U256::zero();
        for i in 0..days_elapsed {
            let release_day: u64 = current_day - (days_elapsed - (i + 1));
            let release_timestamp: u64 = release_day * 86400;
            let (term, full_release) = get_start_term_and_release(release_timestamp);
            if term == 0 || full_release == U256::zero() {
                continue;
            }
            max_exploit += full_release;
        }
        return max_exploit;
    }

    fn get_release_day_and_booster(term: String, current_timestamp: u64) -> (u64, U256) {
        unsafe {
            let current_day: u64 = current_timestamp / 86400;

            if term == TermBooster::THIRTY_DAYS.as_str() {
                return (current_day + 30, U256::from(100));
            } else if term == TermBooster::NINETY_DAYS.as_str() {
                return (current_day + 90, U256::from(116));
            } else if term == TermBooster::SIX_MONTHS.as_str() {
                return (current_day + 182, U256::from(140));
            } else if term == TermBooster::ONE_YEAR.as_str() {
                return (current_day + 365, U256::from(167));
            } else if term == TermBooster::TWO_YEARS.as_str() {
                return (current_day + 730, U256::from(201));
            } else if term == TermBooster::THREE_YEARS.as_str() {
                return (current_day + 1095, U256::from(241));
            } else if term == TermBooster::FOUR_YEARS.as_str() {
                return (current_day + 1460, U256::from(289));
            } else if term == TermBooster::FIVE_YEARS.as_str() {
                return (current_day + 1825, U256::from(347));
            } else if term == TermBooster::SIX_YEARS.as_str() {
                return (current_day + 2190, U256::from(417));
            } else if term == TermBooster::SEVEN_YEARS.as_str() {
                return (current_day + 2555, U256::from(500));
            }
            else {
                return (0, U256::zero());
            }
        }
    }

    fn get_weight(booster: U256, principle: U256, contract_id: String, solana_lp_token: String) -> U256 {

        let mut weight = (booster * principle) / U256::from(100);

        if contract_id == solana_lp_token {
            weight = (weight * U256::from(sol_multi)) / U256::from(sol_multi_scale);
        }
        return weight;
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

    fn get_total_weight_per_day(day_diff: u64, AllStakers: &AllStakers, current_day: u64) -> Vec<U256> {
        unsafe {
            let mut total_weight_per_day: Vec<U256> = Vec::new();
            for i in 0..day_diff {
                let mut total_weight = U256::zero();
                for (wallet_address, _) in &AllStakers.staker_states {
                    let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                    let wallet_stakes: AllWalletStakes = load_state(&wallet_stake_key).unwrap();
                    let release_day: u64 = current_day - (day_diff - (i + 1));
                    for (id_str, stake) in &wallet_stakes.wallet_stakes {
                        if release_day > stake.term_end || stake.last_reward_day >= release_day {
                            continue;
                        }
                        total_weight += types::string_to_u256(stake.weight.clone());
                    }
                }
                total_weight_per_day.push(total_weight);
            }
            return total_weight_per_day;
        }
    }
    
    fn process_wallet_stakes(
        wallet_stake: AllWalletStakes,
        current_day: u64,
        total_weight_per_day: &Vec<U256>,
    ) -> (AllWalletStakes, U256, Vec<(String, String, U256)>) {
        unsafe {
            let mut total_amount: U256 = U256::zero();
            let mut principle_releases: Vec<(String, String, U256)> = Vec::new();

            let mut new_wallet_stakes: AllWalletStakes = AllWalletStakes {
                wallet_stakes: HashMap::new(),
            };

            for (id_str, stake) in &wallet_stake.wallet_stakes {
                let mut new_stake = WalletStake {
                    principle: stake.principle.clone(),
                    weight: stake.weight.clone(),
                    term: stake.term.clone(),
                    term_end: stake.term_end,
                    last_reward_day: stake.last_reward_day,
                    lp_token: stake.lp_token.clone(),
                };

                let day_diff: u64 = current_day.saturating_sub(new_stake.last_reward_day);

                if day_diff == 0 {
                    new_wallet_stakes
                        .wallet_stakes
                        .insert(id_str.clone(), new_stake);
                    continue;
                }

                let stake_weight = types::string_to_u256(new_stake.weight.clone());
                let offset = (total_weight_per_day.len() as u64).saturating_sub(day_diff);

                for x in 0..day_diff {
                    let weight_index = (x + offset) as usize;
                    if weight_index >= total_weight_per_day.len() {
                        break;
                    }
                    if total_weight_per_day[weight_index] == U256::zero() {
                        continue;
                    }

                    let release_day: u64 = current_day - (day_diff - (x + 1));
                    let release_timestamp: u64 = release_day * 86400;
                    if release_day > new_stake.term_end {
                        break;
                    }

                    if new_stake.last_reward_day >= release_day {
                        continue;
                    }
                    let (term, full_release) = get_start_term_and_release(release_timestamp);

                    if term == 0 || full_release == U256::zero() {
                        break;
                    }
                    let release_amount = (stake_weight * full_release) / total_weight_per_day[weight_index];
                    total_amount += release_amount;
                }

                if current_day >= new_stake.term_end {
                    principle_releases.push((id_str.clone(), new_stake.lp_token.clone(), types::string_to_u256(new_stake.principle.clone())));
                } else {
                    new_stake.last_reward_day = current_day;
                    new_wallet_stakes
                        .wallet_stakes
                        .insert(id_str.clone(), new_stake);
                }
            }

            (
                new_wallet_stakes,
                total_amount,
                principle_releases,
            )
        }
    }

    fn process_all_wallet_stakes(
        wallets_released_map: &mut HashMap<String, U256>,
        input_amount: &mut U256,
        remove_wallet: &mut Vec<String>,
        day_diff: u64,
    ) -> (HashMap<String, AllWalletStakes>, Vec<(String, String, String, U256)>) {
        unsafe {
            let mut new_all_wallet_stakes: HashMap<String, AllWalletStakes> = HashMap::new();
            let mut principle_releases: Vec<(String, String, String, U256)> = Vec::new();
            let all_wallet_stakes: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();
            let current_day: u64 = (smart_contracts::last_block_time() / 86400);
            let total_weight_per_day = get_total_weight_per_day(day_diff, &all_wallet_stakes, current_day);

            for (wallet_address, _) in &all_wallet_stakes.staker_states {
                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                let wallet_stakes: AllWalletStakes = match load_state(&wallet_stake_key) {
                    Ok(stakes) => stakes,
                    Err(_) => {
                        continue;
                    }
                };

                let (new_wallet_stakes, total_amount_released, wallet_principle_releases) =
                    process_wallet_stakes(wallet_stakes, current_day, &total_weight_per_day);

                if total_amount_released > U256::zero() {
                    if let Some(existing_amount) = wallets_released_map.get_mut(wallet_address) {
                        *existing_amount += total_amount_released;
                    } else {
                        wallets_released_map.insert(wallet_address.clone(), total_amount_released);
                    }

                    *input_amount += total_amount_released;
                }

                for (bump_id, lp_token, amount) in wallet_principle_releases {
                    principle_releases.push((wallet_address.clone(), bump_id, lp_token, amount));
                }

                if new_wallet_stakes.wallet_stakes.is_empty() {
                    remove_wallet.push(wallet_address.clone());
                } else {
                    new_all_wallet_stakes.insert(wallet_address.clone(), new_wallet_stakes);
                }
            }
            (new_all_wallet_stakes, principle_releases)
        }
    }

    #[wasmedge_bindgen]
    pub fn init() {}

    #[wasmedge_bindgen]
    pub fn one_time_release() {
        unsafe {
            if !check_auth() {
                return;
            }

            let one_time_release = smart_contracts::delegate_retrieve_state(ONE_TIME_RELEASE_KEY.to_string(), PROXY_CONTRACT.to_string());

            if one_time_release == "true" {
                smart_contracts::emit("Failed: One time release already done".to_string());
                return;
            }

            let stakers = vec![
                "5dq5c2ZhfJf895RbdTXqC9qo7byTBKVD5A9RseH2knB4",
                "ChE9GrV5MpJobwAN5kAt3YwYgvS2duwbhFRqJ97HeUpQ",
                "ChuoiNUDy2uRuDuYtztCdthraM7UquB1mwkKW8YuL37B",
            ];

            let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());
            let mut all_stakers: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();

            for staker in &stakers {
                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, staker);
                let mut wallet_stakes: AllWalletStakes = match load_state(&wallet_stake_key) {
                    Ok(stakes) => stakes,
                    Err(_) => {
                        smart_contracts::emit(format!("No stakes found for: {}", staker));
                        continue;
                    }
                };

                let mut sol_lp_bump_ids: Vec<String> = Vec::new();

                for (bump_id, stake) in &wallet_stakes.wallet_stakes {
                    if stake.lp_token == SOLANA_LP_TOKEN {
                        let principle = types::string_to_u256(stake.principle.clone());
                        if !smart_contracts::derived_send(
                            SOLANA_LP_TOKEN.to_string(),
                            principle.to_string(),
                            staker.to_string(),
                            derived_wallet.clone(),
                        ) {
                            panic!("Failed to send SOL LP back to {}", staker);
                        }
                        smart_contracts::emit(format!(
                            "SOL_LP_RELEASE wallet: {} bump_id: {} amount: {}",
                            staker, bump_id, principle
                        ));
                        sol_lp_bump_ids.push(bump_id.clone());
                    }
                }

                for bump_id in &sol_lp_bump_ids {
                    wallet_stakes.wallet_stakes.remove(bump_id);
                }

                if wallet_stakes.wallet_stakes.is_empty() {
                    all_stakers.staker_states.remove(&staker.to_string());
                    smart_contracts::delegate_clear_state(wallet_stake_key, PROXY_CONTRACT.to_string());
                } else {
                    save_state(&wallet_stake_key, &wallet_stakes);
                }
            }

            save_state(ALL_STAKERS_KEY, &all_stakers);
            smart_contracts::delegate_store_state(ONE_TIME_RELEASE_KEY.to_string(), "true".to_string(), PROXY_CONTRACT.to_string());
            smart_contracts::emit("Success: One time SOL LP release complete".to_string());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_sol_token(sol_token: String) {
        unsafe {
            if !check_auth() {
                return;
            }

            let pub_key_ = smart_contracts::public_key();
            let pub_key = pub_key_.clone();

            let gov_keys : GovKeys = load_state(GOV_KEYS_KEY).unwrap();

            if pub_key != gov_keys.update_key.to_string() {
                return;
            }

            smart_contracts::delegate_store_state(SOL_TOKEN_KEY.to_string(), sol_token.clone(), PROXY_CONTRACT.to_string());
            smart_contracts::emit(format!("Success: SOL LP token updated to {}", sol_token));
        }
    }
    #[wasmedge_bindgen]
    pub fn process_rewards() {
        unsafe {
            if !check_auth() {
                return;
            }

            let current_day: u64 = smart_contracts::last_block_time() / 86400;
            let mut boot_manager: BootstrappingManager = load_state(BOOT_MANAGER_KEY).unwrap();

            if current_day <= boot_manager.last_reward_day {
                smart_contracts::emit("Failed: Not ready to process rewards".to_string());
                return;
            }

            let days_elapsed = current_day - boot_manager.last_reward_day;

            let mut wallets_released_map = HashMap::<String, U256>::new();
            let mut input_amount: U256 = U256::zero();
            let mut remove_wallet = Vec::<String>::new();

            let (new_all_wallet_stakes, principle_releases) = process_all_wallet_stakes(
                &mut wallets_released_map,
                &mut input_amount,
                &mut remove_wallet,
                days_elapsed,
            );

            let max_exploit = get_max_exploit(days_elapsed, current_day);

            if input_amount > max_exploit {
                boot_manager.exploit = true;
                save_state(BOOT_MANAGER_KEY, &boot_manager);
                smart_contracts::emit(format!("Failed: Exploit detected, input_amount: {}, max_exploit: {}", input_amount, max_exploit));
                return;
            }

            if input_amount > U256::zero() {
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
                    panic!("Failed to send multi");
                }
            }

            // Send LP token principles back from derived wallet
            if !principle_releases.is_empty() {
                let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());
                for (wallet_address, bump_id, lp_token, amount) in &principle_releases {
                    if !smart_contracts::derived_send(
                        lp_token.clone(),
                        amount.to_string(),
                        wallet_address.clone(),
                        derived_wallet.clone(),
                    ) {
                        panic!("Failed to send LP principle back");
                    }
                    smart_contracts::emit(format!("PRINCIPLE_RELEASE wallet: {} bump_id: {} lp_token: {} amount: {}", wallet_address, bump_id, lp_token, amount));
                }
            }

            smart_contracts::emit(format!("Success: Rewards sent to stakers: {}", input_amount));

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
                smart_contracts::delegate_clear_state(wallet_stake_key, PROXY_CONTRACT.to_string());
            }

            boot_manager.last_reward_day = current_day;

            save_state(BOOT_MANAGER_KEY, &boot_manager);
            save_state(ALL_STAKERS_KEY, &new_all_stakers);

            smart_contracts::emit("Success: Rewards sent to stakers".to_string());
        }
    }

   
    #[wasmedge_bindgen]
    pub fn stake(amount: String, term: String, contract_id: String) {
        unsafe {
            smart_contracts::emit(format!("Staking amount: {}, term: {}, contract_id: {}", amount, term, contract_id));
            if !check_auth() {
                return;
            }

            if !types::is_valid_u256(amount.clone()) {
                smart_contracts::emit(format!("Failed: Invalid amount: {}", amount));
                return;
            }

            let principle: U256 = types::string_to_u256(amount.clone());

            let solana_lp_token = smart_contracts::delegate_retrieve_state(SOL_TOKEN_KEY.to_string(), PROXY_CONTRACT.to_string());

            if((contract_id != ZERA_LP_TOKEN.to_string() && contract_id != solana_lp_token) || contract_id.is_empty()){
                smart_contracts::emit(format!("Failed: Invalid contract id: {}", contract_id));
                return;
            }

            let wallet_address = smart_contracts::wallet_address();
            let balance = smart_contracts::wallet_balance(contract_id.clone(), wallet_address.clone());
            if balance < principle {
                smart_contracts::emit(format!("Failed: Insufficient balance: {}", balance));
                return;
            }

            let current_timestamp: u64 = smart_contracts::last_block_time();
            let current_day: u64 = current_timestamp / 86400;
            let (release_day, term_booster) = get_release_day_and_booster(term.clone(), current_timestamp);
            if release_day == 0 || term_booster == U256::zero() {
                smart_contracts::emit(format!("Failed: Invalid term: {}", term));
                return;
            }
            let weight = get_weight(term_booster, principle, contract_id.clone(), solana_lp_token.clone());

            let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());
            if !smart_contracts::transfer(
                contract_id.clone(),
                principle.to_string(),
                derived_wallet.clone(),
            ) {
                smart_contracts::emit(format!("Failed to transfer: {}", principle));
                return;
            }

            let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
            let mut wallet_stakes: AllWalletStakes =
                load_state(&wallet_stake_key).unwrap_or(AllWalletStakes {
                    wallet_stakes: HashMap::new(),
                });

            let mut id_bump_state: IdBumpState = load_state(ID_BUMP_KEY).unwrap_or(IdBumpState { id: 0 });
            id_bump_state.id = id_bump_state.id + 1;

            wallet_stakes.wallet_stakes.insert(
                id_bump_state.id.to_string(),
                WalletStake {
                    principle: principle.to_string(),
                    weight: weight.to_string(),
                    term: term.clone(),
                    term_end: release_day,
                    last_reward_day: current_day,
                    lp_token: contract_id.clone(),
                },
            );

            let mut all_stakers: AllStakers =
                load_state(ALL_STAKERS_KEY).unwrap_or(AllStakers {
                    staker_states: HashMap::new(),
                });

            all_stakers.staker_states.insert(wallet_address.clone(), 1);

            let mut boot_manager: BootstrappingManager = load_state(BOOT_MANAGER_KEY).unwrap_or(BootstrappingManager { last_reward_day: 0, exploit: false });

            if boot_manager.last_reward_day == 0 {
                boot_manager.last_reward_day = current_day;
            }

            let return_id = id_bump_state.id;

            save_state(BOOT_MANAGER_KEY, &boot_manager);
            save_state(ALL_STAKERS_KEY, &all_stakers);
            save_state(ID_BUMP_KEY, &id_bump_state);
            save_state(&wallet_stake_key, &wallet_stakes);

            smart_contracts::emit("STAKE_SUCCESS".to_string());
            smart_contracts::emit(format!("wallet: {}", wallet_address));
            smart_contracts::emit(format!("term: {}", term));
            smart_contracts::emit(format!("weight: {}", weight));
            smart_contracts::emit(format!("bump_id: {}", return_id));
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
            if !is_valid_wallet_address(&wallet_address) {
                smart_contracts::emit(format!("Failed: Invalid wallet address: {}", wallet_address));
                return;
            }

            let sender_wallet = smart_contracts::wallet_address();

            if wallet_address == sender_wallet {
                smart_contracts::emit(format!("Failed: Cannot update to same wallet: {}", wallet_address));
                return;
            }

            let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, sender_wallet.clone());
            let mut wallet_stakes: AllWalletStakes = match load_state(&wallet_stake_key) {
                Ok(stakes) => stakes,
                Err(_) => {
                    return;
                }
            };

            if let Some(stake) = wallet_stakes.wallet_stakes.get(&bump_id.to_string()) {
                let stake_term = stake.term.clone();
                let updated_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                let mut updated_wallet_stakes: AllWalletStakes = load_state(&updated_stake_key)
                    .unwrap_or(AllWalletStakes {
                        wallet_stakes: HashMap::new(),
                    });

                let mut all_stakers: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();

                updated_wallet_stakes
                    .wallet_stakes
                    .insert(bump_id.to_string(), stake.clone());
                wallet_stakes.wallet_stakes.remove(&bump_id.to_string());

                if wallet_stakes.wallet_stakes.is_empty() {
                    all_stakers.staker_states.remove(&sender_wallet.clone());
                    smart_contracts::delegate_clear_state(wallet_stake_key.clone(), PROXY_CONTRACT.to_string());
                }

                all_stakers.staker_states.insert(wallet_address.clone(), 1);

                save_state(ALL_STAKERS_KEY, &all_stakers);
                save_state(&wallet_stake_key, &wallet_stakes);
                save_state(&updated_stake_key, &updated_wallet_stakes);

                smart_contracts::emit("STAKE_UPDATED".to_string());
                smart_contracts::emit(format!("old_wallet: {}", sender_wallet));
                smart_contracts::emit(format!("new_wallet: {}", wallet_address));
                smart_contracts::emit(format!("term: {}", stake_term));
                smart_contracts::emit(format!("bump_id: {}", bump_id));
                return;
            } else {
                smart_contracts::emit(format!("Failed: Invalid bump_id: {} for wallet: {}", bump_id, wallet_address));
                return;   
            }
        }
    }

    fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
        let bytes = postcard::to_allocvec(data).unwrap();
        let b64 = base64::encode(bytes);
        unsafe { smart_contracts::delegate_store_state(key.to_string(), b64, PROXY_CONTRACT.to_string()) }
    }
    fn load_state<T: DeserializeOwned>(key: &str) -> Result<T, bool> {
        let b64 = unsafe { smart_contracts::delegate_retrieve_state(key.to_string(), PROXY_CONTRACT.to_string()) };
        let bytes = base64::decode(b64).map_err(|_| false)?;
        postcard::from_bytes(&bytes).map_err(|_| false)
    }

    fn is_valid_wallet_address(address: &str) -> bool {
        const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        
        if address.len() < 32 || address.len() > 44 {
            return false;
        }
        
        for c in address.chars() {
            if !BASE58_ALPHABET.contains(c) {
                return false;
            }
        }
        
        match decode_base58(address) {
            Some(decoded) => decoded.len() == 32,
            None => false,
        }
    }

    fn decode_base58(input: &str) -> Option<Vec<u8>> {
        const BASE58_ALPHABET: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        
        let mut result: Vec<u8> = vec![0];
        
        for byte in input.bytes() {
            let mut carry = BASE58_ALPHABET.iter().position(|&x| x == byte)? as u32;
            
            for result_byte in result.iter_mut() {
                carry += (*result_byte as u32) * 58;
                *result_byte = (carry & 0xFF) as u8;
                carry >>= 8;
            }
            
            while carry > 0 {
                result.push((carry & 0xFF) as u8);
                carry >>= 8;
            }
        }
        
        for byte in input.bytes() {
            if byte == b'1' {
                result.push(0);
            } else {
                break;
            }
        }
        
        result.reverse();
        Some(result)
    }

    #[derive(Serialize, Deserialize)]
    pub struct GovKeys{
        pub update_key: String,
        pub send_all_key: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllStakers {
        pub staker_states: HashMap<String, u8>,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllWalletStakes {
        pub wallet_stakes: HashMap<String, WalletStake>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct WalletStake {
        pub principle: String,
        pub weight: String,
        pub term: String,
        pub term_end: u64,
        pub last_reward_day: u64,
        pub lp_token: String
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct SolanaLP{
        pub solana_mint_id: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct BootstrappingManager {
        pub last_reward_day: u64,
        pub exploit: bool,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct IdBumpState {
        pub id: u64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct ReleaseDays{
        date1: u64,
        date2: u64,
        date3: u64,
        date4: u64,
        date5: u64,
        date6: u64,
        date7: u64,
        date8: u64,
        date9: u64,
        date10: u64,
        end_date: u64,
    }
}
