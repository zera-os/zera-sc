pub mod staking_v2 {
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
    const INSTANT_STAKES_KEY: &str = "INSTANT_STAKES_";
    const ALL_INSTANT_STAKERS_KEY: &str = "ALL_INSTANT_STAKERS";
    const MIGRATED_KEY: &str = "MIGRATED";
    const exploit_limit: u64 = 50_000_000_000_000; //50k ZRA
    const instant_stake_rate: u64 = 66666666666;
    const instant_stake_const: u64 = 100000000000;
    const PRINCIPLE_SEED: &str = "principle";
    const PROXY_CONTRACT: &str = "staking_proxy_1";

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

    fn get_release_day(term: String) -> u64 {
        unsafe {
            let current_day: u64 = (smart_contracts::last_block_time() / 86400);

            if term == StakingType::SIX_MONTHS.as_str() {
                return current_day + 182;
            } else if term == StakingType::ONE_YEAR.as_str() {
                return current_day + 365;
            } else if term == StakingType::TWO_YEARS.as_str() {
                return current_day + 730;
            } else if term == StakingType::THREE_YEARS.as_str() {
                return current_day + 1095;
            } else if term == StakingType::FOUR_YEARS.as_str() {
                return current_day + 1460;
            } else if term == StakingType::FIVE_YEARS.as_str() {
                return current_day + 1825;
            } else {
                return 0;
            }
        }
    }

    fn get_reward(term: String, principle: u64) -> (u64, u64) {
        let mut total_reward: u64 = 0;
        let mut daily_release: u64 = 0;

        if term == StakingType::SIX_MONTHS.as_str() {
            let yearly_reward: u64 = (principle * 2) / 100;
            total_reward = yearly_reward / 2;
            daily_release = total_reward / 182;
        } else if term == StakingType::ONE_YEAR.as_str() {
            total_reward = (principle * 6) / 100;
            daily_release = total_reward / 365;
        } else if term == StakingType::TWO_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 8) / 100;
            total_reward = yearly_reward * 2;
            daily_release = total_reward / 730;
        } else if term == StakingType::THREE_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 7) / 100;
            total_reward = yearly_reward * 3;
            daily_release = total_reward / 1095;
        } else if term == StakingType::FOUR_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 7) / 100;
            total_reward = yearly_reward * 4;
            daily_release = total_reward / 1460;
        } else if term == StakingType::FIVE_YEARS.as_str() {
            let yearly_reward: u64 = (principle * 7) / 100;
            total_reward = yearly_reward * 5;
            daily_release = total_reward / 1825;
        } else if term == StakingType::LIQUID.as_str() {
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

            let migrated = smart_contracts::retrieve_state(MIGRATED_KEY.to_string());

            if migrated != "true"{
                smart_contracts::emit(format!("Failed: Not migrated"));
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
                        wallets_released_map
                            .insert(staker_state.staker_address.clone(), finish_release);
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
        unsafe {
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
                    term: stake.term.clone(),
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
    pub fn init() {}

    #[wasmedge_bindgen]
    pub fn init_v2() {
        unsafe {
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
            let sc_wallet = sc_wallet_.clone();

            if sc_wallet != PROXY_WALLET.to_string() {
                return;
            }

            let migrated = smart_contracts::retrieve_state(MIGRATED_KEY.to_string());

            if migrated == "true"{
                smart_contracts::emit(format!("Failed: Already migrated"));
                return;
            }

            let wallet_balance = smart_contracts::wallet_balance(
                ZRA_CONTRACT.to_string(),
                PRINCIPLE_WALLET.to_string(),
            );

            let parameters_vec: Vec<String> =
                [PRINCIPLE_FUNCTION.to_string(), wallet_balance.to_string()].to_vec();

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

            let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());

            if !smart_contracts::send(
                ZRA_CONTRACT.to_string(),
                wallet_balance.to_string(),
                derived_wallet.clone(),
            ) {
                panic!("Failed to send");
            }

            let (keys, values) = smart_contracts::get_all_states("staking_v1".to_string(), "1".to_string());

            for (key, value) in keys.iter().zip(values.iter()) {
                smart_contracts::delegate_store_state(key.clone(), value.clone(), PROXY_CONTRACT.to_string());
            }

            let all_wallet_stakes: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();
            for (wallet_address, _) in &all_wallet_stakes.staker_states {
                let wallet_stake_key = format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
                let mut wallet_stakes: OldAllWalletStakes = load_state(&wallet_stake_key).unwrap();
                let mut new_wallet_stakes: AllWalletStakes = AllWalletStakes {
                    staker_states: HashMap::new(),
                    liquid_stake: LiquidStake {
                        bump_id: 0,
                        principle: 0,
                        last_reward_day: 0,
                        daily_release: 0,
                        unstake_day: 0,
                    },
                };

                for (id, stake) in &wallet_stakes.staker_states {
                    let mut new_stake: WalletStake = WalletStake {
                        principle: stake.principle,
                        total_reward: stake.total_reward,
                        daily_release: stake.daily_release,
                        total_released: stake.total_released,
                        last_reward_day: stake.last_reward_day,
                        term: "5_years".to_string(),
                    };
                    new_wallet_stakes.staker_states.insert(id.clone(), new_stake);
                }
                save_state(&wallet_stake_key, &new_wallet_stakes);
            }

            if !smart_contracts::store_state(MIGRATED_KEY.to_string(), "true".to_string())
            {
                panic!("Failed to store migrated state");
            }

            smart_contracts::emit(format!("Success: Principle sent to derived wallet, and staking v1 states migrated"));
        }
    }

    #[wasmedge_bindgen]
    pub fn process_rewards() {
        unsafe {
            if !check_auth() {
                return;
            }

            let current_day: u64 = smart_contracts::last_block_time() / 86400 as u64;
            let mut reward_manager_state: RewardManagerState =
                load_state(REWARD_MANAGER_STATE_KEY).unwrap();

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

            let (new_staker_states) =
                process_early_staking(days_elapsed, &mut wallets_released_map, &mut input_amount);

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
                let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());

                if !smart_contracts::derived_send(
                    ZRA_CONTRACT.to_string(),
                    total_principle_amount.to_string(),
                    PROXY_WALLET.to_string(),
                    derived_wallet.clone(),
                ) {
                    panic!("Failed to send");
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
                panic!("Failed to send multi");
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

            smart_contracts::emit(format!("Success: Rewards sent to stakers"));
        }
    }

    #[wasmedge_bindgen]
    pub fn instant_stake(amount: String, term: String) {
        unsafe {
            if !check_auth() {
                return;
            }

            let mut reward_manager_state: RewardManagerState =
                load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }

            if !types::is_valid_u256(amount.to_string()) {
                return;
            }

            if term == StakingType::LIQUID.as_str() {
                return;
            }

            let principle = types::string_to_u256(amount.clone()).low_u64();
            let mut return_id: u64 = 0;

            let (mut total_reward, daily_release) = get_reward(term.clone(), principle);

            if total_reward == 0 && daily_release == 0 {
                return;
            }

            let mut release_day = get_release_day(term.clone());

            if release_day == 0 {
                return;
            }

            let wallet_address = smart_contracts::wallet_address();

            let instant_stake_percent: U256 = U256::from(instant_stake_rate);
            let instant_stake_constant: U256 = U256::from(instant_stake_const);
            let mut instant_stake_reward: U256 = U256::from(total_reward);
            instant_stake_reward = (instant_stake_reward * instant_stake_percent) / instant_stake_constant;

            if instant_stake_reward == U256::from(0) {
                smart_contracts::emit(format!("Failed: Instant stake reward is 0"));
                return;
            }

            total_reward = instant_stake_reward.to_string().parse::<u64>().unwrap();

            let used_supply: u64 = reward_manager_state.used_supply + total_reward;

            if used_supply > reward_manager_state.total_supply {
                smart_contracts::emit(format!(
                    "Failed: Insufficient supply: {} > {}",
                    used_supply.to_string(),
                    reward_manager_state.total_supply.to_string()
                ));
                return;
            }

            reward_manager_state.used_supply = used_supply;
            let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());

            if !smart_contracts::transfer(
                ZRA_CONTRACT.to_string(),
                principle.to_string(),
                derived_wallet.to_string(),
            ) {
                return;
            }

            let instant_stake_key = format!("{}_{}", INSTANT_STAKES_KEY, wallet_address.clone());
            let mut instant_stakes: AllWalletInstantStakes = load_state(&instant_stake_key)
                .unwrap_or(AllWalletInstantStakes {
                    staker_states: HashMap::new(),
                });

            let mut id_bump_state: IdBumpState = load_state(ID_BUMP_KEY).unwrap();

            instant_stakes.staker_states.insert(
                id_bump_state.id.to_string(),
                InstantStake {
                    principle: principle,
                    total_reward: total_reward,
                    release_day: release_day,
                    term: term.clone(),
                },
            );

            return_id = id_bump_state.id;
            id_bump_state.id = id_bump_state.id + 1;

            let mut all_instant_stakers: AllInstantStakers = load_state(ALL_INSTANT_STAKERS_KEY)
                .unwrap_or(AllInstantStakers {
                    staker_states: HashMap::new(),
                    earliest_release_day: u64::MAX,
                });

            all_instant_stakers
                .staker_states
                .insert(wallet_address.clone(), 1);

            if release_day < all_instant_stakers.earliest_release_day {
                all_instant_stakers.earliest_release_day = release_day;
            }

            if !smart_contracts::send(
                ZRA_CONTRACT.to_string(),
                total_reward.to_string(),
                wallet_address.clone(),
            ) {
                panic!("Failed to send reward.");
            }

            save_state(REWARD_MANAGER_STATE_KEY, &reward_manager_state);
            save_state(ID_BUMP_KEY, &id_bump_state);
            save_state(ALL_INSTANT_STAKERS_KEY, &all_instant_stakers);
            save_state(&instant_stake_key, &instant_stakes);

            smart_contracts::emit("INSTANT_STAKE_SUCCESS".to_string());
            smart_contracts::emit(format!("wallet: {}", wallet_address));
            smart_contracts::emit(format!("term: {}", term));
            smart_contracts::emit(format!("bump_id: {}", return_id));
        }
    }

    #[wasmedge_bindgen]
    pub fn release_instant() {
        unsafe {
            if !check_auth() {
                return;
            }

            let reward_manager_state: RewardManagerState =
                load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }
            let mut all_instant_stakers: AllInstantStakers =
                load_state(ALL_INSTANT_STAKERS_KEY).unwrap();

            let current_day: u64 = (smart_contracts::last_block_time() / 86400);

            if current_day < all_instant_stakers.earliest_release_day {
                let release_day = all_instant_stakers.earliest_release_day - current_day;
                smart_contracts::emit(format!("Failed: Not enough time has passed to release instant stakes, {} days remaining", release_day));
                return;
            }
            let mut wallets_to_release: Vec<String> = Vec::new();
            let mut amounts_released: Vec<String> = Vec::new();
            let mut total_amount: u64 = 0;

            let mut new_all_instant_stakers: AllInstantStakers = AllInstantStakers {
                staker_states: HashMap::new(),
                earliest_release_day: u64::MAX,
            };

            for (wallet_address, _) in all_instant_stakers.staker_states.iter() {
                //get all instant stakes for the wallet
                let instant_stake_key =
                    format!("{}_{}", INSTANT_STAKES_KEY, wallet_address.to_string());
                let mut instant_stakes: AllWalletInstantStakes =
                    load_state(&instant_stake_key).unwrap();

                //create new instant stakes for the wallet, reconstruct new instant stakes
                let mut new_instant_stakes: AllWalletInstantStakes = AllWalletInstantStakes {
                    staker_states: HashMap::new(),
                };

                for (id_str, stake) in &instant_stakes.staker_states {
                    //if the stake is ready to be released, add to the lists
                    //else, add back to the new instant stakes
                    if stake.release_day <= current_day {
                        wallets_to_release.push(wallet_address.to_string());
                        amounts_released.push(stake.principle.to_string());
                        total_amount += stake.principle;
                    } else {
                        new_instant_stakes.staker_states.insert(
                            id_str.clone(),
                            InstantStake {
                                principle: stake.principle,
                                total_reward: stake.total_reward,
                                release_day: stake.release_day,
                                term: stake.term.clone(),
                            },
                        );

                        if stake.release_day < new_all_instant_stakers.earliest_release_day {
                            new_all_instant_stakers.earliest_release_day = stake.release_day;
                        }
                    }
                }

                if !new_instant_stakes.staker_states.is_empty() {
                    new_all_instant_stakers
                        .staker_states
                        .insert(wallet_address.to_string(), 1);
                    save_state(&instant_stake_key, &new_instant_stakes);
                } else {
                    smart_contracts::clear_state(instant_stake_key.clone());
                }
            }

            if total_amount == 0 {
                return;
            }

            let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());

            if !smart_contracts::derived_send_multi(
                ZRA_CONTRACT.to_string(),
                total_amount.to_string(),
                amounts_released,
                wallets_to_release,
                derived_wallet.clone(),
            ) {
                panic!("Failed to send multi");
            }

            save_state(ALL_INSTANT_STAKERS_KEY, &new_all_instant_stakers);

            smart_contracts::emit("INSTANT_STAKE_RELEASED".to_string());
            smart_contracts::emit(format!("total_amount_released: {}", total_amount));
        }
    }

    #[wasmedge_bindgen]
    pub fn stake(amount: String, wallet_address: String, term: String) {
        unsafe {
            if !check_auth() {
                return;
            }

            let mut reward_manager_state: RewardManagerState =
                load_state(REWARD_MANAGER_STATE_KEY).unwrap();

            if reward_manager_state.exploit {
                return;
            }

            if !types::is_valid_u256(amount.to_string()) {
                return;
            }

            let principle: u64 = amount.parse::<u64>().unwrap();

            let (total_reward, daily_release) = get_reward(term.clone(), principle);

            if total_reward == 0 && daily_release == 0 {
                return;
            }
            let derived_wallet = smart_contracts::derive_wallet(PRINCIPLE_SEED.to_string());
            let mut return_id = 0;

            if term != StakingType::LIQUID.as_str() {
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
                    derived_wallet.to_string(),
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
                        term: term.clone(),
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
                    derived_wallet.to_string(),
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

            smart_contracts::emit("STAKE_SUCCESS".to_string());
            smart_contracts::emit(format!("wallet: {}", wallet_address));
            smart_contracts::emit(format!("term: {}", term));
            smart_contracts::emit(format!("return_id: {}", return_id));
        }
    }

    #[wasmedge_bindgen]
    pub fn release_liquid_stake() {
        unsafe {
            if !check_auth() {
                return;
            }

            let mut reward_manager_state: RewardManagerState =
                load_state(REWARD_MANAGER_STATE_KEY).unwrap();

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

            smart_contracts::emit("LIQUID_STAKE_RELEASED".to_string());
            smart_contracts::emit(format!("wallet: {}", sender_wallet));
            smart_contracts::emit("days_until_release: 14".to_string());
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

            let mut reward_manager_state: RewardManagerState =
                load_state(REWARD_MANAGER_STATE_KEY).unwrap();

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
                let stake_term = stake.term.clone();
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
                smart_contracts::emit("STAKE_UPDATED".to_string());
                smart_contracts::emit(format!("old_wallet: {}", sender_wallet));
                smart_contracts::emit(format!("new_wallet: {}", wallet_address));
                smart_contracts::emit(format!("term: {}", stake_term));
                smart_contracts::emit(format!("bump_id: {}", bump_id));
                return;
            } else {
                if wallet_stakes.liquid_stake.bump_id == bump_id.parse::<u64>().unwrap() {
                    let updated_stake_key =
                        format!("{}_{}", WALLET_STAKES_KEY, wallet_address.clone());
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
                    updated_wallet_stakes.liquid_stake.principle +=
                        wallet_stakes.liquid_stake.principle;
                    updated_wallet_stakes.liquid_stake.last_reward_day =
                        smart_contracts::last_block_time() / 86400;
                    updated_wallet_stakes.liquid_stake.daily_release +=
                        wallet_stakes.liquid_stake.daily_release;
                    updated_wallet_stakes.liquid_stake.unstake_day = u64::MAX;

                    wallet_stakes.liquid_stake.bump_id = 0;
                    wallet_stakes.liquid_stake.principle = 0;
                    wallet_stakes.liquid_stake.last_reward_day = 0;
                    wallet_stakes.liquid_stake.daily_release = 0;
                    wallet_stakes.liquid_stake.unstake_day = 0;

                    let mut all_stakers: AllStakers = load_state(ALL_STAKERS_KEY).unwrap();

                    if wallet_stakes.staker_states.is_empty()
                        && wallet_stakes.liquid_stake.bump_id == 0
                    {
                        all_stakers.staker_states.remove(&sender_wallet.clone());
                        smart_contracts::clear_state(wallet_stake_key.clone());
                    }

                    all_stakers.staker_states.insert(wallet_address.clone(), 1);
                    save_state(ALL_STAKERS_KEY, &all_stakers);
                    save_state(&updated_stake_key, &updated_wallet_stakes);
                    save_state(&wallet_stake_key, &wallet_stakes);

                    smart_contracts::emit("LIQUID_STAKE_UPDATED".to_string());
                    smart_contracts::emit(format!("old_wallet: {}", sender_wallet));
                    smart_contracts::emit(format!("new_wallet: {}", wallet_address));
                    return;
                } else {
                    return;
                }
            }
        }
    }

    #[wasmedge_bindgen]
    pub fn update_instant_wallet(wallet_address: String, bump_id: String) {
        unsafe {
            if !check_auth() {
                return;
            }

            if !is_valid_wallet_address(&wallet_address) {
                return;
            }

            if bump_id.parse::<u64>().is_err() {
                smart_contracts::emit("Failed: Invalid bump_id, must be a valid u64".to_string());
                return;
            }

            let original_wallet = smart_contracts::wallet_address();

            let mut all_instant_stakers: AllInstantStakers = load_state(ALL_INSTANT_STAKERS_KEY).unwrap();

            if !all_instant_stakers.staker_states.contains_key(&original_wallet.to_string()) {
                smart_contracts::emit("Failed: Original wallet not found in instant stakers".to_string());
                return;
            }

            let original_instant_stake_key = format!("{}_{}", INSTANT_STAKES_KEY, original_wallet.to_string());

            let mut original_instant_stakes: AllWalletInstantStakes = load_state(&original_instant_stake_key).unwrap();

            let stake : InstantStake = match original_instant_stakes.staker_states.get(&bump_id.to_string()) {
                Some(s) => s.clone(),
                None => {
                    smart_contracts::emit("Failed: Bump ID not found in original wallet's instant stakes".to_string());
                    return;
                }
            };

            let new_instant_stake_key = format!("{}_{}", INSTANT_STAKES_KEY, wallet_address.to_string());

            let mut new_instant_stakes: AllWalletInstantStakes = load_state(&new_instant_stake_key)
                .unwrap_or(AllWalletInstantStakes {
                    staker_states: HashMap::new(),
                });

            let term = stake.term.clone();
            new_instant_stakes.staker_states.insert(bump_id.to_string(), stake);
            original_instant_stakes.staker_states.remove(&bump_id.to_string());
            
            if original_instant_stakes.staker_states.is_empty() {
                smart_contracts::delegate_clear_state(original_instant_stake_key.clone(), PROXY_CONTRACT.to_string());
                all_instant_stakers.staker_states.remove(&original_wallet.to_string());
            }
            else{
                save_state(&original_instant_stake_key, &original_instant_stakes);
            }

            if !all_instant_stakers.staker_states.contains_key(&wallet_address) {
                all_instant_stakers.staker_states.insert(wallet_address.clone(), 1);
            }

            save_state(&new_instant_stake_key, &new_instant_stakes);
            save_state(ALL_INSTANT_STAKERS_KEY, &all_instant_stakers);

            smart_contracts::emit("INSTANT_STAKE_UPDATED".to_string());
            smart_contracts::emit(format!("old_wallet: {}", original_wallet));
            smart_contracts::emit(format!("new_wallet: {}", wallet_address));
            smart_contracts::emit(format!("term: {}", term));
            smart_contracts::emit(format!("bump_id: {}", bump_id));
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

     // Validates that a string is a valid Solana base58 address
    // Zera addresses are 32-byte public keys encoded in base58
    fn is_valid_wallet_address(address: &str) -> bool {
        // Base58 alphabet (Bitcoin/Solana style - excludes 0, O, I, l)
        const BASE58_ALPHABET: &str = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
        
        // Check length (base58 encoded 32 bytes is typically 32-44 characters)
        if address.len() < 32 || address.len() > 44 {
            return false;
        }
        
        // Check all characters are valid base58
        for c in address.chars() {
            if !BASE58_ALPHABET.contains(c) {
                return false;
            }
        }
        
        // Additional validation: decode and verify it's exactly 32 bytes
        match decode_base58(address) {
            Some(decoded) => decoded.len() == 32,
            None => false,
        }
    }

     // Decodes a base58 string to bytes
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
        
        // Add leading zeros
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
        pub term: String,
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

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllInstantStakers {
        pub staker_states: HashMap<String, u8>,
        pub earliest_release_day: u64,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct AllWalletInstantStakes {
        pub staker_states: HashMap<String, InstantStake>,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct InstantStake {
        pub principle: u64,
        pub total_reward: u64,
        pub release_day: u64,
        pub term: String,
    }

    // need to migrate this to the new wallet stake struct
    #[derive(Serialize, Deserialize, Debug)]
    pub struct OldAllWalletStakes {
        pub staker_states: HashMap<String, OldWalletStake>,
        pub liquid_stake: LiquidStake,
    }

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct OldWalletStake {
        pub principle: u64,
        pub total_reward: u64,
        pub daily_release: u64,
        pub total_released: u64,
        pub last_reward_day: u64,
    }

}
