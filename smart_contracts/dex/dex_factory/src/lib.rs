pub mod zera_dex {
    use base64::{decode, encode};
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use native_functions::zera::wasmedge_bindgen;
    use postcard::{from_bytes, to_allocvec};
    use serde::de::DeserializeOwned;
    use serde::{Deserialize, Serialize};

    const SMART_CONTRACT_KEY: &str = "SMART_CONTRACT_";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const LIQUIDITY_POOL_KEY: &str = "LP_";
    const FEE_KEY: &str = "FEE_";
    const PROXY_WALLET: &str = "3uct7y6rcxW3KA8o8b2gqtaygw7hA39P3SyjV466fXWP";
    const SCALE: u64 = 1000000000; //10^9
    const DEX_BURN: &str = "DeXBurnDexBurnDexBurnDexBurnDexBurnDex";
    const LOCK_SEED: &str = "lock";
    const LOCK_KEY: &str = "LOCK_";
    const GOV_AUTH: &str = "gov_$ACE+0000";
    const PROXY_CONTRACT: &str = "zera_dex_proxy_1";
    const ACE_KEY: &str = "ACE_";
    const ONE_DOLLA: &str = "1000000000000000000"; //10^18

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe {
            let locked_derived = smart_contracts::derive_wallet(LOCK_SEED.to_string());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_fees(treasury_fee: String, treasury_wallet: String) {
        unsafe {
            if !check_auth() {
                smart_contracts::emit("Failed: Must be called by proxy wallet".to_string());
                return;
            }

            let pub_key_ = smart_contracts::public_key();

            if pub_key_ != GOV_AUTH.to_string() {
                smart_contracts::emit("Failed: Must be called by gov auth".to_string());
                return;
            }

            if !treasury_fee.parse::<u64>().is_ok() {
                smart_contracts::emit("Failed: Invalid treasury fee".to_string());
                return;
            }

            let treasury_fee_u64 = treasury_fee.parse::<u64>().unwrap();

            if treasury_fee_u64 > 1000 {
                smart_contracts::emit("Failed: Treasury fee cannot be greater than 1000".to_string());
                return;
            }

            if !is_valid_wallet_address(&treasury_wallet) {
                smart_contracts::emit("Failed: Invalid treasury wallet".to_string());
                return;
            }

            let fees = Fees {
                treasury_fee: treasury_fee_u64,
                treasury_wallet: treasury_wallet.clone(),
            };

            if !save_state(FEE_KEY, &fees) {
                smart_contracts::emit("Failed: Failed to save fees".to_string());
                return;
            }

            smart_contracts::emit("FEES_UPDATED".to_string());
            smart_contracts::emit(format!("treasury_fee: {}", treasury_fee_u64.to_string()));
            smart_contracts::emit(format!("treasury_wallet: {}", treasury_wallet.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn unlock_liquidity_pool_tokens(token1: String, token2: String, fee_bps: String) {
        unsafe {
            if !check_auth() {
                smart_contracts::emit("Failed: Must be called by proxy wallet".to_string());
                return;
            }

            if !valid_fee_bps(fee_bps.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid fee bps: {}. Must be 10, 25, 50, 100, 200, 400, or 800",
                    fee_bps.clone()
                ));
                return;
            }

            let mut lp_key = format!(
                "{}{}{}{}",
                LIQUIDITY_POOL_KEY,
                fee_bps.clone(),
                token1.clone(),
                token2.clone()
            );
            let mut token1_calc = token1.clone();
            let mut token2_calc = token2.clone();
            let mut pool: LiquidityPool = match load_state::<LiquidityPool>(&lp_key) {
                Ok(pool) => pool,
                Err(_) => {
                    // First order failed, try swapped order
                    lp_key = format!(
                        "{}{}{}{}",
                        LIQUIDITY_POOL_KEY,
                        fee_bps.clone(),
                        token2.clone(),
                        token1.clone()
                    );
                    match load_state::<LiquidityPool>(&lp_key) {
                        Ok(pool) => {
                            token1_calc = token2.clone();
                            token2_calc = token1.clone();
                            pool
                        }
                        Err(_) => {
                            smart_contracts::emit(format!("Failed to unlock liquidity pool tokens for {} : Pool does not exist", token1.clone()));
                            return;
                        }
                    }
                }
            };

            let wallet_address = smart_contracts::wallet_address();
            let lock_key = format!(
                "{}{}{}{}{}",
                LOCK_KEY,
                wallet_address.clone(),
                token1_calc.clone(),
                token2_calc.clone(),
                fee_bps.clone()
            );

            let locked_lp: LockedLiquidityPool = match load_state::<LockedLiquidityPool>(&lock_key)
            {
                Ok(locked_lp) => locked_lp,
                Err(_) => {
                    smart_contracts::emit(format!("Failed to unlock liquidity pool tokens for Pairing {} - {} : Invalid Wallet {}", token1_calc.clone(), token2_calc.clone(), wallet_address.clone()));
                    return;
                }
            };

            let timestamp: u64 = smart_contracts::last_block_time();

            if timestamp < locked_lp.lock_timestamp {
                smart_contracts::emit(format!(
                    "Failed to unlock liquidity pool tokens: {} Unlocked at {}",
                    locked_lp.lp_tokens.clone(),
                    locked_lp.lock_timestamp
                ));
                return;
            }

            let locked_derived = smart_contracts::derive_wallet(LOCK_SEED.to_string());

            if !smart_contracts::derived_send(
                pool.lp_token_id.clone(),
                locked_lp.lp_tokens.clone(),
                wallet_address.clone(),
                locked_derived.clone(),
            ) {
                smart_contracts::emit(format!(
                    "Failed to transfer liquidity pool tokens: {}",
                    locked_lp.lp_tokens.clone()
                ));
                return;
            }

            smart_contracts::clear_state(lock_key);

            smart_contracts::emit("LIQUIDITY_UNLOCKED".to_string());
            smart_contracts::emit(format!("wallet_address: {}", wallet_address.clone()));
            smart_contracts::emit(format!("token1: {}", token1_calc.clone()));
            smart_contracts::emit(format!("token2: {}", token2_calc.clone()));
            smart_contracts::emit(format!("fee_bps: {}", fee_bps.clone()));
            smart_contracts::emit(format!(
                "lp_tokens_unlocked: {}",
                locked_lp.lp_tokens.to_string()
            ));
            smart_contracts::emit(format!("lp_contract_id: {}", pool.lp_token_id.clone()));
            smart_contracts::emit(format!(
                "lock_timestamp: {}",
                locked_lp.lock_timestamp.to_string()
            ));
        }
    }
    #[wasmedge_bindgen]
    pub fn create_liquidity_pool(
        token1: String,
        token2: String,
        token1_volume: String,
        token2_volume: String,
        fee_bps: String,
        lock_timestamp: String,
    ) {
        unsafe {
            if !check_auth() {
                return;
            }
            let mut token1_calc = token1.clone();
            let mut token2_calc = token2.clone();

            if token1_calc == token2_calc {
                smart_contracts::emit(format!(
                    "Failed to create liquidity pool: Token 1 and Token 2 cannot be the same"
                ));
                return;
            }

            let mut token1_volume_str = token1_volume.clone();
            let mut token2_volume_str = token2_volume.clone();
            
            if token2_calc == ZRA_CONTRACT.to_string() {
                token1_calc = token2.clone();
                token2_calc = token1.clone();
                token1_volume_str = token2_volume.clone();
                token2_volume_str = token1_volume.clone();
            }


            //check to see if volumes are bigger than 0 and is a valid u256
            if token1_volume_str.clone() == "0"
                || token2_volume_str.clone() == "0"
                || !types::is_valid_u256(token1_volume_str.clone())
                || !types::is_valid_u256(token2_volume_str.clone())
                || !lock_timestamp.parse::<u64>().is_ok()
            {
                smart_contracts::emit("Failed: Invalid parameters".to_string());
                return;
            }

            let token1_volume_u256 = types::string_to_u256(token1_volume_str.clone());
            let token2_volume_u256 = types::string_to_u256(token2_volume_str.clone());
            let mut wallet_address = smart_contracts::wallet_address();

            // Validate fee_bps matches one of the FeePercent enum values
            if !valid_fee_bps(fee_bps.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid fee bps: {}. Must be 10, 25, 50, 100, 200, 400, or 800",
                    fee_bps.clone()
                ));
                return;
            }

            //check to see if token balance is greater than token volume this will also determine if the contract exists
            let token1_balance =
                smart_contracts::wallet_balance(token1_calc.clone(), wallet_address.clone());
            if token1_balance < token1_volume_u256 {
                return;
            }

            //check to see if zera balance is greater than zera volume
            let token2_balance =
                smart_contracts::wallet_balance(token2_calc.clone(), wallet_address.clone());
            if token2_balance < token2_volume_u256 {
                return;
            }

            let lp_key = format!(
                "{}{}{}{}",
                LIQUIDITY_POOL_KEY,
                fee_bps.clone(),
                token1_calc.clone(),
                token2_calc.clone()
            );
            let lp_key_2 = format!(
                "{}{}{}{}",
                LIQUIDITY_POOL_KEY,
                fee_bps.clone(),
                token2_calc.clone(),
                token1_calc.clone()
            );

            // Try to load existing pool - if it succeeds, pool already exists, so return/fail
            if load_state::<LiquidityPool>(&lp_key).is_ok() {
                return; // Pool already exists, abort
            }

            // Try to load existing pool - if it succeeds, pool already exists, so return/fail
            if load_state::<LiquidityPool>(&lp_key_2).is_ok() {
                return; // Pool already exists, abort
            }

            let derived = smart_contracts::derive_wallet(format!(
                "{}{}{}",
                token1_calc.clone(),
                token2_calc.clone(),
                fee_bps.clone()
            ));

            if !smart_contracts::transfer(
                token1_calc.clone(),
                token1_volume_str.clone(),
                derived.clone(),
            ) {
                return;
            }
            if !smart_contracts::transfer(
                token2_calc.clone(),
                token2_volume_str.clone(),
                derived.clone(),
            ) {
                panic!("transfer failed");
            }

            let token1_denom = smart_contracts::contract_denomination(token1_calc.clone());
            let token1_scaled: U256 = (token1_volume_u256 * SCALE) / token1_denom;

            let token2_denom = smart_contracts::contract_denomination(token2_calc.clone());
            let token2_scaled: U256 = (token2_volume_u256 * SCALE) / token2_denom;

            // Calculate LP tokens for first liquidity: sqrt(amount_zera * amount_token)
            let lp_tokens_amount = U256::sqrt(token1_scaled * token2_scaled);

            let timestamp: u64 = smart_contracts::last_block_time();
            let lock_timestamp_u64 = lock_timestamp.parse::<u64>().unwrap();

            if timestamp < lock_timestamp_u64 {
                let locked_lp = LockedLiquidityPool {
                    token1: token1_calc.clone(),
                    token2: token2_calc.clone(),
                    lp_tokens: lp_tokens_amount.to_string(),
                    lock_timestamp: lock_timestamp_u64.clone(),
                };
                let locked_derived = smart_contracts::derive_wallet(LOCK_SEED.to_string());
                let lock_key = format!(
                    "{}{}{}{}{}",
                    LOCK_KEY,
                    wallet_address.clone(),
                    token1_calc.clone(),
                    token2_calc.clone(),
                    fee_bps.clone()
                );
                save_state(&lock_key, &locked_lp);

                wallet_address = locked_derived.clone();
            }

            let lp_token_id = smart_contracts::instrument_contract_dex(
                token1_calc.clone(),
                token2_calc.clone(),
                wallet_address.clone(),
                lp_tokens_amount.to_string(),
                fee_bps.clone(),
            );

            if lp_token_id == "0" {
                panic!("instrument contract dex failed");
            }

            let liquidity_pool = LiquidityPool {
                token1: token1_calc.clone(),
                token2: token2_calc.clone(),
                lp_token_id: lp_token_id.clone(),
                token1_volume: token1_volume_str.clone(),
                token2_volume: token2_volume_str.clone(),
                circulating_lp_tokens: lp_tokens_amount.to_string(),
                active: true,
                derived_wallet: derived.clone(),
                redeemed_lp_tokens: "0".to_string(),
                fee_bps: fee_bps.parse::<u64>().unwrap(),
            };

            save_state(&lp_key, &liquidity_pool);

            if token1_calc == ZRA_CONTRACT.to_string() && fee_bps.clone() == "25".to_string()
            {
                calc_ace_value(token2_calc.clone(), token1_volume_u256.clone(), token2_volume_u256.clone(), token2_denom.clone());
            }

            smart_contracts::emit("LIQUIDITY_CREATED".to_string());
            smart_contracts::emit(format!("token1: {}", token1_calc.clone()));
            smart_contracts::emit(format!("token2: {}", token2_calc.clone()));
            smart_contracts::emit(format!("fee_bps: {}", fee_bps.clone()));
            smart_contracts::emit(format!("lp_token_id: {}", lp_token_id.clone()));
            smart_contracts::emit(format!(
                "lp_tokens_minted: {}",
                lp_tokens_amount.to_string()
            ));
            smart_contracts::emit(format!("amount_token1: {}", token1_volume_str.clone()));
            smart_contracts::emit(format!("amount_token2: {}", token2_volume_str.clone()));
        }
    }

    #[wasmedge_bindgen]
    pub fn add_liquidity(
        token1: String,
        token2: String,
        token1_volume: String,
        token2_volume: String,
        fee_bps: String,
        lock_timestamp: String,
    ) {
        unsafe {
            if !check_auth() {
                smart_contracts::emit("Failed: Must be called by proxy wallet".to_string());
                return;
            }

            let mut wallet_address = smart_contracts::wallet_address();

            //check to see if volumes are bigger than 0 and is a valid u256
            if token1_volume.clone() == "0"
                || token2_volume.clone() == "0"
                || !types::is_valid_u256(token1_volume.clone())
                || !types::is_valid_u256(token2_volume.clone())
                || !lock_timestamp.parse::<u64>().is_ok()
            {
                smart_contracts::emit("Failed: Invalid parameters".to_string());
                return;
            }

            if !valid_fee_bps(fee_bps.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid fee bps: {}. Must be 10, 25, 50, 100, 200, 400, or 800",
                    fee_bps.clone()
                ));
                return;
            }

            let mut lp_key = format!(
                "{}{}{}{}",
                LIQUIDITY_POOL_KEY,
                fee_bps.clone(),
                token1.clone(),
                token2.clone()
            );

            let mut token1_calc = token1.clone();
            let mut token2_calc = token2.clone();
            let mut token1_volume_calc = token1_volume.clone();
            let mut token2_volume_calc = token2_volume.clone();

            // Load existing pool - try first order
            let mut pool: LiquidityPool = match load_state::<LiquidityPool>(&lp_key) {
                Ok(pool) => pool,
                Err(_) => {
                    // First order failed, try swapped order
                    lp_key = format!(
                        "{}{}{}{}",
                        LIQUIDITY_POOL_KEY,
                        fee_bps.clone(),
                        token2.clone(),
                        token1.clone()
                    );
                    match load_state::<LiquidityPool>(&lp_key) {
                        Ok(pool) => {
                            token1_calc = token2.clone();
                            token2_calc = token1.clone();
                            token1_volume_calc = token2_volume.clone();
                            token2_volume_calc = token1_volume.clone();
                            pool
                        }
                        Err(_) => return, // Pool doesn't exist in either order, fail
                    }
                }
            };

            //convert volumes to u256
            let token1_volume_u256 = types::string_to_u256(token1_volume_calc.clone());
            let token2_volume_u256 = types::string_to_u256(token2_volume_calc.clone());

            //check to see if token balance is greater than token volume this will also determine if the contract exists
            let token1_balance =
                smart_contracts::wallet_balance(token1_calc.clone(), wallet_address.clone());
            if token1_balance < token1_volume_u256 {
                smart_contracts::emit(format!(
                    "Failed: Insufficient {} balance: {}",
                    token1_calc.clone(),
                    token1_balance.to_string()
                ));
                return;
            }

            //check to see if zera balance is greater than zera volume
            let token2_balance =
                smart_contracts::wallet_balance(token2_calc.clone(), wallet_address.clone());
            if token2_balance < token2_volume_u256 {
                smart_contracts::emit(format!(
                    "Failed: Insufficient {} balance: {}",
                    token2_calc.clone(),
                    token2_balance.to_string()
                ));
                return;
            }

            let token1_denom = smart_contracts::contract_denomination(token1_calc.clone());
            let token2_denom = smart_contracts::contract_denomination(token2_calc.clone());

            let token1_scaled: U256 = (token1_volume_u256 * SCALE) / token1_denom;
            let token2_scaled: U256 = (token2_volume_u256 * SCALE) / token2_denom;

            let pool_token1_scaled: U256 =
                (types::string_to_u256(pool.token1_volume.clone()) * SCALE) / token1_denom;
            let pool_token2_scaled: U256 =
                (types::string_to_u256(pool.token2_volume.clone()) * SCALE) / token2_denom;

            // Calculate LP tokens to mint and actual amounts to transfer
            let (lp_tokens_to_mint, token1_to_use_scaled, token2_to_use_scaled) =
                calculate_lp_tokens_to_mint(
                    token1_scaled,
                    token2_scaled,
                    pool_token1_scaled,
                    pool_token2_scaled,
                    types::string_to_u256(pool.circulating_lp_tokens.clone()),
                );

            // Denormalize token amount back to raw
            let token1_to_use = (token1_to_use_scaled * token1_denom) / SCALE;
            let token2_to_use = (token2_to_use_scaled * token2_denom) / SCALE;

            if !smart_contracts::transfer(
                token1_calc.clone(),
                token1_to_use.to_string(),
                pool.derived_wallet.clone(),
            ) {
                return;
            }
            if !smart_contracts::transfer(
                token2_calc.clone(),
                token2_to_use.to_string(),
                pool.derived_wallet.clone(),
            ) {
                panic!("transfer failed");
            }

            let timestamp: u64 = smart_contracts::last_block_time();
            let lock_timestamp_u64 = lock_timestamp.parse::<u64>().unwrap();

            if timestamp < lock_timestamp_u64 {
                let mut lock_key = format!(
                    "{}{}{}{}{}",
                    LOCK_KEY,
                    wallet_address.clone(),
                    token1_calc.clone(),
                    token2_calc.clone(),
                    fee_bps.clone()
                );

                // Load existing locked pool or create new one
                let mut locked_lp: LockedLiquidityPool =
                    match load_state::<LockedLiquidityPool>(&lock_key) {
                        Ok(mut existing) => {
                            // Update existing pool - add to lp_tokens
                            let current_lp = types::string_to_u256(existing.lp_tokens.clone());
                            existing.lp_tokens = (current_lp + lp_tokens_to_mint).to_string();
                            existing.lock_timestamp = lock_timestamp_u64.clone();
                            existing
                        }
                        Err(_) => {
                            // Create new pool
                            LockedLiquidityPool {
                                token1: token1_calc.clone(),
                                token2: token2_calc.clone(),
                                lp_tokens: lp_tokens_to_mint.to_string(),
                                lock_timestamp: lock_timestamp_u64.clone(),
                            }
                        }
                    };
                let locked_derived = smart_contracts::derive_wallet(LOCK_SEED.to_string());
                save_state(&lock_key, &locked_lp);

                wallet_address = locked_derived.clone();
            }
            if !smart_contracts::mint(
                pool.lp_token_id.clone(),
                lp_tokens_to_mint.to_string(),
                wallet_address.clone(),
            ) {
                panic!("mint failed");
            }

            let new_circulating_lp_tokens =
                types::string_to_u256(pool.circulating_lp_tokens.clone()) + lp_tokens_to_mint;
            let new_token1_volume =
                types::string_to_u256(pool.token1_volume.clone()) + token1_to_use;
            let new_token2_volume =
                types::string_to_u256(pool.token2_volume.clone()) + token2_to_use;

            pool.circulating_lp_tokens = new_circulating_lp_tokens.to_string();
            pool.token1_volume = new_token1_volume.to_string();
            pool.token2_volume = new_token2_volume.to_string();

            save_state(&lp_key, &pool);

            smart_contracts::emit("LIQUIDITY_ADDED".to_string());
            smart_contracts::emit(format!("token1: {}", token1_calc.clone()));
            smart_contracts::emit(format!("token2: {}", token2_calc.clone()));
            smart_contracts::emit(format!("fee_bps: {}", fee_bps.clone()));
            smart_contracts::emit(format!("lp_token_id: {}", pool.lp_token_id.clone()));
            smart_contracts::emit(format!(
                "lp_tokens_minted: {}",
                lp_tokens_to_mint.to_string()
            ));
            smart_contracts::emit(format!("amount_token1: {}", token1_to_use.to_string()));
            smart_contracts::emit(format!("amount_token2: {}", token2_to_use.to_string()));
            smart_contracts::emit(format!(
                "reserve_token1: {}",
                pool.token1_volume.to_string()
            ));
            smart_contracts::emit(format!(
                "reserve_token2: {}",
                pool.token2_volume.to_string()
            ));
            smart_contracts::emit(format!(
                "circulating_lp_tokens: {}",
                pool.circulating_lp_tokens.to_string()
            ));
            smart_contracts::emit(format!(
                "lp_tokens_redeemed_total: {}",
                pool.redeemed_lp_tokens.to_string()
            ));
        }
    }

    #[wasmedge_bindgen]
    pub fn remove_liquidity(
        token1: String,
        token2: String,
        lp_tokens: String,
        fee_bps: String,
    ) {
        unsafe {
            if !check_auth() {
                smart_contracts::emit("Failed: Must be called by proxy wallet".to_string());
                return;
            }

            if !types::is_valid_u256(lp_tokens.clone()) {
                smart_contracts::emit(format!("Failed: Invalid lp tokens: {}", lp_tokens.clone()));
                return;
            }

            if !valid_fee_bps(fee_bps.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid fee bps: {}. Must be 10, 25, 50, 100, 200, 400, or 800",
                    fee_bps.clone()
                ));
                return;
            }

            let mut token1_calc = token1.clone();
            let mut token2_calc = token2.clone();

            let mut lp_key = format!(
                "{}{}{}{}",
                LIQUIDITY_POOL_KEY,
                fee_bps.clone(),
                token1.clone(),
                token2.clone()
            );

            // Load existing pool - if it doesn't exist, fail
            let mut pool: LiquidityPool = match load_state::<LiquidityPool>(&lp_key) {
                Ok(pool) => pool,
                Err(_) => {
                    // First order failed, try swapped order
                    lp_key = format!(
                        "{}{}{}{}",
                        LIQUIDITY_POOL_KEY,
                        fee_bps.clone(),
                        token2.clone(),
                        token1.clone()
                    );
                    match load_state::<LiquidityPool>(&lp_key) {
                        Ok(pool) => {
                            token1_calc = token2.clone();
                            token2_calc = token1.clone();
                            pool
                        }
                        Err(_) => return, // Pool doesn't exist in either order, fail
                    }
                }
            };

            let wallet_address = smart_contracts::wallet_address();

            if !types::is_valid_u256(lp_tokens.clone()) {
                smart_contracts::emit(format!("Failed: Invalid LP tokens: {}", lp_tokens.clone()));
                return;
            }
            let lp_tokens_u256 = types::string_to_u256(lp_tokens.clone());
            let lp_token_balance =
                smart_contracts::wallet_balance(pool.lp_token_id.clone(), wallet_address.clone());

            if lp_token_balance < lp_tokens_u256 {
                smart_contracts::emit(format!(
                    "Failed: Insufficient LP tokens: {}",
                    lp_token_balance.to_string()
                ));
                return;
            }
            let token1_denom = smart_contracts::contract_denomination(token1_calc.clone());
            let token2_denom = smart_contracts::contract_denomination(token2_calc.clone());

            let token1_scaled: U256 =
                (types::string_to_u256(pool.token1_volume.clone()) * SCALE) / token1_denom;
            let token2_scaled: U256 =
                (types::string_to_u256(pool.token2_volume.clone()) * SCALE) / token2_denom;

            let (mut token1_out, mut token2_out) = calculate_remove_liquidity(
                lp_tokens_u256,
                token1_scaled,
                token2_scaled,
                types::string_to_u256(pool.circulating_lp_tokens.clone()),
            );

            if !smart_contracts::transfer(
                pool.lp_token_id.clone(),
                lp_tokens.clone(),
                DEX_BURN.to_string(),
            ) {
                return;
            }

            token1_out = (token1_out * token1_denom) / SCALE;
            token2_out = (token2_out * token2_denom) / SCALE;

            if !smart_contracts::derived_send(
                token1_calc.clone(),
                token1_out.to_string(),
                wallet_address.clone(),
                pool.derived_wallet.clone(),
            ) {
                panic!("derived send failed");
            }

            if !smart_contracts::derived_send(
                token2_calc.clone(),
                token2_out.to_string(),
                wallet_address.clone(),
                pool.derived_wallet.clone(),
            ) {
                panic!("derived send failed");
            }

            let new_circulating_lp_tokens =
                types::string_to_u256(pool.circulating_lp_tokens.clone()) - lp_tokens_u256;
            let new_token1_volume = types::string_to_u256(pool.token1_volume.clone()) - token1_out;
            let new_token2_volume = types::string_to_u256(pool.token2_volume.clone()) - token2_out;
            let new_redeemed_lp_tokens =
                types::string_to_u256(pool.redeemed_lp_tokens.clone()) + lp_tokens_u256;

            pool.circulating_lp_tokens = new_circulating_lp_tokens.to_string();
            pool.token1_volume = new_token1_volume.to_string();
            pool.token2_volume = new_token2_volume.to_string();
            pool.redeemed_lp_tokens = new_redeemed_lp_tokens.to_string();

            if (new_circulating_lp_tokens.is_zero()) {
                pool.active = false;

                if token1_calc == ZRA_CONTRACT.to_string() && fee_bps.clone() == "25".to_string()
                {
                    let ace_key = format!("{}{}", ACE_KEY, token2_calc.clone());
                    smart_contracts::delegate_clear_state(ace_key.to_string(), PROXY_CONTRACT.to_string());

                    let zra_ace_key = format!("{}{}{}", ACE_KEY, ZRA_CONTRACT.to_string(), token2_calc.clone());
                    smart_contracts::delegate_clear_state(zra_ace_key.to_string(), PROXY_CONTRACT.to_string());
                }
            }



            save_state(&lp_key, &pool);

            smart_contracts::emit("LIQUIDITY_REMOVED".to_string());
            smart_contracts::emit(format!("token1: {}", token1_calc.clone()));
            smart_contracts::emit(format!("token2: {}", token2_calc.clone()));
            smart_contracts::emit(format!("fee_bps: {}", fee_bps.clone()));
            smart_contracts::emit(format!("lp_token_id: {}", pool.lp_token_id.clone()));
            smart_contracts::emit(format!("lp_tokens_redeemed: {}", lp_tokens.clone()));
            smart_contracts::emit(format!("amount_token1: {}", token1_out.to_string()));
            smart_contracts::emit(format!("amount_token2: {}", token2_out.to_string()));
            smart_contracts::emit(format!(
                "reserve_token1: {}",
                pool.token1_volume.to_string()
            ));
            smart_contracts::emit(format!(
                "reserve_token2: {}",
                pool.token2_volume.to_string()
            ));
            smart_contracts::emit(format!(
                "circulating_lp_tokens: {}",
                pool.circulating_lp_tokens.to_string()
            ));
            smart_contracts::emit(format!(
                "lp_tokens_redeemed_total: {}",
                pool.redeemed_lp_tokens.to_string()
            ));
        }
    }

    #[wasmedge_bindgen]
    pub fn swap(token1: String, token2: String, token1_volume: String, fee_bps: String, platform_bps: String, platform_wallet: String) {
        unsafe {
            if !check_auth() {
                smart_contracts::emit("Failed: Must be called by proxy wallet".to_string());
                return;
            }

            if !types::is_valid_u256(token1_volume.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid token1 volume: {}",
                    token1_volume.clone()
                ));
                return;
            }

            if !types::is_valid_u256(platform_bps.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid platform bps: {}",
                    platform_bps.clone()
                ));
                return;
            }

            let platform_bps_u256 = types::string_to_u256(platform_bps.clone());

            if platform_bps_u256 > U256::from(500) {
                smart_contracts::emit(format!(
                    "Failed: Platform bps cannot be greater than 500 (5%): {}",
                    platform_bps.clone()
                ));
                return;
            }

            if platform_wallet.clone() != "".to_string() && !is_valid_wallet_address(&platform_wallet.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid platform wallet: {}",
                    platform_wallet.clone()
                ));
                return;
            }

            if token1_volume == "0" {
                smart_contracts::emit(format!("Failed: Token1 volume cannot be 0"));
                return;
            }

            if !valid_fee_bps(fee_bps.clone()) {
                smart_contracts::emit(format!(
                    "Failed: Invalid fee bps: {}. Must be 10, 25, 50, 100, 200, 400, or 800",
                    fee_bps.clone()
                ));
                return;
            }

            let mut lp_key = format!(
                "{}{}{}{}",
                LIQUIDITY_POOL_KEY,
                fee_bps.clone(),
                token1.clone(),
                token2.clone()
                );

            let mut swapped = false;
            // Load existing pool - if it doesn't exist, fail
            let mut pool: LiquidityPool = match load_state::<LiquidityPool>(&lp_key) {
                Ok(pool) => pool,
                Err(_) => {
                    lp_key = format!(
                        "{}{}{}{}",
                        LIQUIDITY_POOL_KEY,
                        fee_bps.clone(),
                        token2.clone(),
                        token1.clone()
                    );
                    match load_state::<LiquidityPool>(&lp_key) {
                        Ok(pool) => {
                            swapped = true;
                            pool
                        },
                        Err(_) => {
                            smart_contracts::emit(format!(
                                "Failed: Pool does not exist: {}{}",
                                token1.clone(),
                                token2.clone()
                            ));
                            return;
                        }
                    }
                } // Pool doesn't exist, fail
            };

            let wallet_address = smart_contracts::wallet_address();

            let token1_volume_u256 = types::string_to_u256(token1_volume.clone());

            let token1_balance =
                smart_contracts::wallet_balance(ZRA_CONTRACT.to_string(), wallet_address.clone());

            if token1_balance < token1_volume_u256 {
                smart_contracts::emit(format!(
                    "Failed: Insufficient token1 balance: {}",
                    token1_balance.to_string()
                ));
                return;
            }

            let fees: Fees = match load_state::<Fees>(FEE_KEY) {
                Ok(fees) => fees,
                Err(_) => {
                    smart_contracts::emit("Failed: Fees do not exist".to_string());
                    return;
                } // Fees don't exist, fail
            };


            let token1_denom = smart_contracts::contract_denomination(token1.clone());
            let token2_denom = smart_contracts::contract_denomination(token2.clone());

            let mut token1_scaled: U256 = U256::zero();
            let mut token2_scaled: U256 = U256::zero();
            if pool.token1 == token1.clone() {
                token1_scaled =
                    (types::string_to_u256(pool.token1_volume.clone()) * SCALE) / token1_denom;
                token2_scaled =
                    (types::string_to_u256(pool.token2_volume.clone()) * SCALE) / token2_denom;
            } else {
                token1_scaled =
                    (types::string_to_u256(pool.token2_volume.clone()) * SCALE) / token1_denom;
                token2_scaled =
                    (types::string_to_u256(pool.token1_volume.clone()) * SCALE) / token2_denom;
            }

            let token1_volume_scaled = (token1_volume_u256 * SCALE) / token1_denom;
            let (mut token_out, mut treasury_out, mut reward_fee) = calculate_swap(
                token1_volume_scaled,
                token1_scaled,
                token2_scaled,
                U256::from(fees.treasury_fee),
                U256::from(fee_bps.parse::<u64>().unwrap()),
            );

            token_out = (token_out * token2_denom) / SCALE;
            treasury_out = (treasury_out * token1_denom) / SCALE;
            reward_fee = (reward_fee * token1_denom) / SCALE;


            let pool_amount = token1_volume_u256 - treasury_out;
            let amounts = vec![pool_amount.to_string(), treasury_out.to_string()];
            let addresses = vec![pool.derived_wallet.clone(), fees.treasury_wallet.clone()];

            //make a multi transfer native function
            if !smart_contracts::transfer_multi(
                token1.clone(),
                token1_volume.clone(),
                amounts,
                addresses,
            ) {
                return;
            }
            
            let mut platform_out : U256 = U256::zero();
            if platform_bps_u256 == U256::zero() {
                if !smart_contracts::derived_send(
                    token2.clone(),
                    token_out.to_string(),
                    wallet_address.clone(),
                    pool.derived_wallet.clone(),
                )   {
                    panic!("derived send failed");
                }
            }
            else{
                platform_out = (token_out * platform_bps_u256) / U256::from(10000);
                let swap_out : U256 = token_out - platform_out;
                let swap_amounts = vec![swap_out.to_string(), platform_out.to_string()];
                let swap_addresses = vec![wallet_address.clone(), platform_wallet.clone()];

                if !smart_contracts::derived_send_multi(token2.clone(), token_out.to_string(), swap_amounts, swap_addresses, pool.derived_wallet.clone())
                {
                    panic!("derived send multi failed");
                }
            }
            let mut new_token1_volume = U256::zero();
            let mut new_token2_volume = U256::zero();

            if swapped {
                new_token1_volume = types::string_to_u256(pool.token1_volume.clone()) - token_out;
                new_token2_volume = types::string_to_u256(pool.token2_volume.clone()) + pool_amount;
            } else {
                new_token1_volume = types::string_to_u256(pool.token1_volume.clone()) + pool_amount;
                new_token2_volume = types::string_to_u256(pool.token2_volume.clone()) - token_out;
            }

            pool.token1_volume = new_token1_volume.to_string();
            pool.token2_volume = new_token2_volume.to_string();

            if !save_state(&lp_key, &pool) {
                panic!("save state failed");
            }

            if fee_bps.clone() == "25".to_string()
            {
                if(token1 == ZRA_CONTRACT.to_string())
                {
                    calc_ace_value(token2.clone(), new_token1_volume.clone(), new_token2_volume.clone(), token2_denom.clone());
                }
                else if (token2 == ZRA_CONTRACT.to_string())
                {
                    calc_ace_value(token1.clone(), new_token1_volume.clone(), new_token2_volume.clone(), token1_denom.clone());
                }
            }          
            smart_contracts::emit("SWAP_EXECUTED".to_string());
            smart_contracts::emit(format!("token_in: {}", token1.clone()));
            smart_contracts::emit(format!("token_out: {}", token2.clone()));
            smart_contracts::emit(format!("fee_bps: {}", fee_bps.clone()));
            smart_contracts::emit(format!("amount_in: {}", token1_volume.clone()));
            smart_contracts::emit(format!("amount_out: {}", token_out.to_string()));
            smart_contracts::emit(format!("reserve_in: {}", new_token1_volume.to_string()));
            smart_contracts::emit(format!("reserve_out: {}", new_token2_volume.to_string()));
            smart_contracts::emit(format!("treasury_fee: {}", treasury_out.to_string()));
            smart_contracts::emit(format!("reward_fee: {}", reward_fee.to_string()));
            smart_contracts::emit(format!("platform_fee: {}", platform_out.to_string()));
        }
    }

    fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
        let bytes = postcard::to_allocvec(data).unwrap();
        let b64 = base64::encode(bytes);

        unsafe { smart_contracts::delegate_store_state(key.to_string(), b64.to_string(), PROXY_CONTRACT.to_string()) }
    }
    fn load_state<T: DeserializeOwned>(key: &str) -> Result<T, bool> {
        let b64 = unsafe { smart_contracts::delegate_retrieve_state(key.to_string(), PROXY_CONTRACT.to_string()) };
        let bytes = base64::decode(b64).map_err(|_| false)?;
        postcard::from_bytes(&bytes).map_err(|_| false)
    }

    fn calc_ace_value(token: String, zra_volume: U256, token_volume: U256, token_denom: U256) {
        unsafe {
            let scaled_zra_volume: U256 = zra_volume * token_denom;
            let scaled_token_volume: U256 = token_volume * 1000000000;
            let ace_key = format!("{}{}", ACE_KEY, token.clone());
            let one_stable = types::string_to_u256(ONE_DOLLA.to_string());
            let ace_value : U256 = (scaled_zra_volume * one_stable) / scaled_token_volume;
            if!smart_contracts::delegate_store_state(ace_key.to_string(), ace_value.to_string(), PROXY_CONTRACT.to_string())
            {
                panic!("Failed to store ACE value");
            }

            let zra_ace_key = format!("{}{}{}", ACE_KEY, ZRA_CONTRACT.to_string(), token.clone());

            let zra_ace_value : U256 = (scaled_token_volume * one_stable) / scaled_zra_volume;

            if!smart_contracts::delegate_store_state(zra_ace_key.to_string(), zra_ace_value.to_string(), PROXY_CONTRACT.to_string())
            {
                panic!("Failed to store ZRA ACE value");
            }
        }
    }
    /// Calculates swap output for Token → Zera
    ///
    /// # Parameters:
    /// - `amount_token`: Amount of tokens being swapped in (normalized)
    /// - `reserve_zera`: Current ZERA reserve in the pool (raw, at 10^9)
    /// - `reserve_token`: Current token reserve in the pool (normalized)
    /// - `treasury_fee_bps`: Treasury fee in basis points (1 = 0.01%, 10 = 0.1%, 25 = 0.25%)
    ///
    /// # Returns:
    /// - (zera_out, treasury_fee_in_tokens): zera_out in raw ZERA, treasury_fee in normalized tokens
    /// Calculates swap output for any Token1 → Token2 swap
    ///
    /// # Parameters:
    /// - `amount_in`: Amount of input token being swapped in (normalized/scaled)
    /// - `reserve_in`: Current reserve of input token in the pool (normalized/scaled)
    /// - `reserve_out`: Current reserve of output token in the pool (normalized/scaled)
    /// - `treasury_fee_bps`: Treasury fee in basis points (1 = 0.01%, 10 = 0.1%, 25 = 0.25%)
    ///
    /// # Returns:
    /// - (amount_out, treasury_fee): amount_out of output token, treasury_fee in input token
    ///
    /// # Formula:
    /// 1. Treasury fee is taken from input: treasury_fee = amount_in * treasury_fee_bps / 10000
    /// 2. Remaining amount goes to pool: amount_to_pool = amount_in - treasury_fee
    /// 3. Constant product formula with reward fee:
    ///    amount_out = (reserve_out * amount_to_pool * (10000 - reward_fee_bps)) / ((reserve_in + amount_to_pool) * 10000)
    fn calculate_swap(
        amount_in: U256,
        reserve_in: U256,
        reserve_out: U256,
        treasury_fee_bps: U256,
        lp_fee: U256,
    ) -> (U256, U256, U256) {
        unsafe{

        let scale = U256::from(1000000);

        let lp_fee_calculated : U256 = lp_fee * 100;

        let treasury_fee_calculated = (lp_fee_calculated * treasury_fee_bps) / 1000;
        let reward_fee_calculated = lp_fee_calculated - treasury_fee_calculated;

        // Take treasury fee from input first
        let treasury_fee = (amount_in * treasury_fee_calculated) / scale;
        let reward_fee = (amount_in * reward_fee_calculated) / scale;
        let amount_to_pool = amount_in - treasury_fee;

        // Then do the swap calculation with reward fee on the amount going to pool
        // Using constant product formula: x * y = k
        let amount_out = (reserve_out * amount_to_pool * (scale - reward_fee_calculated))
            / ((reserve_in + amount_to_pool) * scale);
         
        (amount_out, treasury_fee, reward_fee)
        }
    }

    /// Calculates the amount of LP tokens to mint and actual amounts to transfer
    ///
    /// # Parameters:
    /// - `amount_zera`: Amount of ZERA user wants to add
    /// - `amount_token`: Amount of token user wants to add
    /// - `reserve_zera`: Current ZERA reserve in the pool
    /// - `reserve_token`: Current token reserve in the pool
    /// - `total_lp_supply`: Current total LP tokens in circulation
    ///
    /// # Returns:
    /// - (lp_tokens, zera_to_transfer, token_to_transfer)
    /// - If pool is empty: uses all amounts provided
    /// - If pool has liquidity: calculates amounts to maintain ratio
    fn calculate_lp_tokens_to_mint(
        amount_token1: U256,
        amount_token2: U256,
        reserve_token1: U256,
        reserve_token2: U256,
        total_lp_supply: U256,
    ) -> (U256, U256, U256) {
        // If pool is empty (drained back to 0), use sqrt formula and all amounts
        if total_lp_supply.is_zero() {
            let lp_tokens = U256::sqrt(amount_token1 * amount_token2);
            return (lp_tokens, amount_token1, amount_token2);
        }

        // Pool has liquidity, use the min of both ratios
        let lp_from_token1 = (amount_token1 * total_lp_supply) / reserve_token1;
        let lp_from_token2 = (amount_token2 * total_lp_supply) / reserve_token2;

        // Return the minimum to maintain ratio and calculate actual amounts
        if lp_from_token1 < lp_from_token2 {
            // Token1 is the limiting factor, calculate how much token2 is needed
            let token2_needed = (lp_from_token1 * reserve_token2) / total_lp_supply;
            (lp_from_token1, amount_token1, token2_needed)
        } else {
            // Token2 is the limiting factor, calculate how much token1 is needed
            let token1_needed = (lp_from_token2 * reserve_token1) / total_lp_supply;
            (lp_from_token2, token1_needed, amount_token2)
        }
    }

    /// Calculates the amounts of ZERA and token to return when removing liquidity
    ///
    /// # Parameters:
    /// - `lp_amount`: Amount of LP tokens being burned
    /// - `reserve_zera`: Current ZERA reserve in the pool
    /// - `reserve_token`: Current token reserve in the pool
    /// - `total_lp_supply`: Current total LP tokens in circulation
    ///
    /// # Returns:
    /// - (zera_out, token_out): Tuple of amounts to return to the user
    fn calculate_remove_liquidity(
        lp_amount: U256,
        reserve_token1: U256,
        reserve_token2: U256,
        total_lp_supply: U256,
    ) -> (U256, U256) {
        let token1_out = (lp_amount * reserve_token1) / total_lp_supply;
        let token2_out = (lp_amount * reserve_token2) / total_lp_supply;
        (token1_out, token2_out)
    }

    fn valid_fee_bps(fee_bps: String) -> bool {
        if fee_bps == "10"
            || fee_bps == "25"
            || fee_bps == "50"
            || fee_bps == "100"
            || fee_bps == "200"
            || fee_bps == "400"
            || fee_bps == "800"
        {
            return true;
        }
        return false;
    }

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



    #[derive(Serialize, Deserialize)]
    pub struct LiquidityPool {
        pub token1: String,
        pub token2: String,
        pub lp_token_id: String,
        pub token1_volume: String,
        pub token2_volume: String,
        pub circulating_lp_tokens: String,
        pub active: bool,
        pub derived_wallet: String,
        pub redeemed_lp_tokens: String,
        pub fee_bps: u64,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Fees {
        pub treasury_fee: u64,
        pub treasury_wallet: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct LockedLiquidityPool {
        pub token1: String,
        pub token2: String,
        pub lp_tokens: String,
        pub lock_timestamp: u64,
    }
}
