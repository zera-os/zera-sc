pub mod network_fees_v2 {
    use native_functions::zera::smart_contracts;
    use native_functions::zera::types;
    use native_functions::zera::types::U256;
    use native_functions::zera::wasmedge_bindgen;

    const PROXY_WALLET: &str = "5o5AkKgjtcqsTVbxNHCRHvUCfL9nJ7F48CqfayJyuRu"; //sc_network_fee_proxy_1
    const ZRA_CONTRACT: &str = "$ZRA+0000";

    #[wasmedge_bindgen]
    pub fn init() {
        unsafe {
            let (authorized, rate) = smart_contracts::get_ace_data(ZRA_CONTRACT.to_string());
            let denomination = smart_contracts::contract_denomination(ZRA_CONTRACT.to_string());
            let one_dolla = types::string_to_u256("10000000000000000000".to_string()); //change to 10 $
            let one_dolla_zera = (one_dolla * denomination) / rate;

            smart_contracts::hold(ZRA_CONTRACT.to_string(), one_dolla_zera.to_string());

            //Key/hash fees
            smart_contracts::store_state("A_KEY_FEE".to_string(), "1".to_string());
            smart_contracts::store_state("B_KEY_FEE".to_string(), "50000000000000000".to_string());
            smart_contracts::store_state("a_HASH_FEE".to_string(), "20000000000000000".to_string());
            smart_contracts::store_state("b_HASH_FEE".to_string(), "50000000000000000".to_string());
            smart_contracts::store_state("c_HASH_FEE".to_string(), "10000000000000000".to_string());

            smart_contracts::store_state(
                "DELEGATED_VOTING_TXN_FEE".to_string(),
                "50000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "VALIDATOR_HOLDING_MINIMUM".to_string(),
                "25000000000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "VALIDATOR_MINIMUM_ZERA".to_string(),
                "10000000000000".to_string(),
            );

            smart_contracts::store_state("RESTRICTED_KEY_FEE".to_string(), "3".to_string());
            smart_contracts::store_state("GAS_FEE".to_string(), "13750000000".to_string());
            smart_contracts::store_state("COIN_TYPE".to_string(), "15000000000000".to_string());
            smart_contracts::store_state("STORAGE_FEE".to_string(), "13500000000000".to_string());
            smart_contracts::store_state(
                "CONTRACT_TXN_FEE".to_string(),
                "860000000000000".to_string(),
            );
            smart_contracts::store_state(
                "EXPENSE_RATIO_TYPE".to_string(),
                "4000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "ITEM_MINT_TYPE".to_string(),
                "1000000000000000".to_string(),
            );
            smart_contracts::store_state("MINT_TYPE".to_string(), "1000000000000000".to_string());
            smart_contracts::store_state("NFT_TYPE".to_string(), "300000000000000".to_string());
            smart_contracts::store_state(
                "PROPOSAL_RESULT_TYPE".to_string(),
                "10000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "PROPOSAL_TYPE".to_string(),
                "5000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "SMART_CONTRACT_EXECUTE_TYPE".to_string(),
                "1500000000000".to_string(),
            );
            smart_contracts::store_state(
                "SMART_CONTRACT_TYPE".to_string(),
                "400000000000000".to_string(),
            );
            smart_contracts::store_state(
                "SMART_CONTRACT_INSTANTIATE_TYPE".to_string(),
                "10000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "UPDATE_CONTRACT_TYPE".to_string(),
                "75000000000000000".to_string(),
            );
            smart_contracts::store_state("VOTE_TYPE".to_string(), "100000000000000".to_string());
            smart_contracts::store_state(
                "VALIDATOR_REGISTRATION_TYPE".to_string(),
                "10000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "VALIDATOR_HEARTBEAT_TYPE".to_string(),
                "50000000000000".to_string(),
            );
            smart_contracts::store_state(
                "FAST_QUORUM_TYPE".to_string(),
                "40000000000000000".to_string(),
            );
            smart_contracts::store_state("QUASH_TYPE".to_string(), "1000000000000000".to_string());
            smart_contracts::store_state("REVOKE_TYPE".to_string(), "1000000000000000".to_string());
            smart_contracts::store_state(
                "SBT_BURN_TYPE".to_string(),
                "1000000000000000".to_string(),
            );
            smart_contracts::store_state("SAFE_SEND".to_string(), "100000000000000".to_string());
            smart_contracts::store_state(
                "COMPLIANCE_TYPE".to_string(),
                "1000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "DELEGATED_VOTING_TYPE".to_string(),
                "1000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "DELEGATED_FEE".to_string(),
                "1000000000000000".to_string(),
            );
            smart_contracts::store_state(
                "ALLOWANCE_TYPE".to_string(),
                "1000000000000000".to_string(),
            );

            smart_contracts::store_state("VALIDATOR_FEE_PERCENTAGE".to_string(), "50".to_string());
            smart_contracts::store_state("BURN_FEE_PERCENTAGE".to_string(), "25".to_string());
            smart_contracts::store_state("TREASURY_FEE_PERCENTAGE".to_string(), "25".to_string());
            smart_contracts::store_state("VALIDATOR_REGISTRATION_TXN_FEE".to_string(), "10000000000000000".to_string());
            smart_contracts::store_state("COMPLIANCE_TXN_FEE".to_string(), "1000000000000000".to_string());
            smart_contracts::store_state("ATTESTATION_QUORUM".to_string(), "51".to_string());
            smart_contracts::store_state("DELEGATED_VOTE_TXN_FEE".to_string(), "1000000000000000".to_string());
        }
    }

    #[wasmedge_bindgen]
    pub fn update_network_fees(target: String, amount: String) {
        unsafe {
            let sc_wallet_ = smart_contracts::called_smart_contract_wallet();
            let sc_wallet = sc_wallet_.clone();

            if sc_wallet != PROXY_WALLET.to_string() {
                let emit1 = format!("Failed: Unauthorized sender key: {}", sc_wallet.clone());
                return;
            }

            let target_vec: Vec<String> =
                target.clone().split("**").map(|s| s.to_string()).collect();
            let amount_vec: Vec<String> =
                amount.clone().split("**").map(|s| s.to_string()).collect();

            // Check if both vectors are the same size
            if target_vec.len() != amount_vec.len() {
                let emit1 = format!("Failed: Target and amount vectors must be the same size. Targets: {}, Amounts: {}", target_vec.len(), amount_vec.len());
                smart_contracts::emit(emit1.clone());
                return;
            }

            // Loop through both vectors and update each target with its corresponding amount
            for i in 0..target_vec.len() {
                if (!types::is_valid_u256(amount_vec[i].clone())) {
                    let emit1 = format!("Failed: Invalid amount: {}", amount_vec[i].clone());
                    smart_contracts::emit(emit1.clone());
                    return;
                }
                let amount_u256 = types::string_to_u256(amount_vec[i].clone());

                if (amount_u256 <= U256::from(0)) {
                    let emit1 = format!(
                        "Failed: Amount cannot be less than or equal to 0: {}",
                        amount_vec[i].clone()
                    );
                    smart_contracts::emit(emit1.clone());
                    return;
                }
                if target_vec[i] == "VALIDATOR_MINIMUM_ZERA" {
                    // Handle VALIDATOR_MINIMUM_ZERA specifically
                    if amount_u256 > types::string_to_u256("100000000000000".to_string()){
                        let emit1 = format!("Failed: VALIDATOR_MINIMUM_ZERA cannot be greater than 100 000 000 000 000");
                        smart_contracts::emit(emit1.clone());
                        return;
                    }
                
                } else if (target_vec[i] == "COIN_TYPE" || target_vec[i] == "VOTE_TYPE") {
                    // Handle other targets normally
                    if amount_u256 > types::string_to_u256("10000000000000000".to_string()){
                        let emit1 = format!("Failed: COIN_TYPE and VOTE_TYPE cannot be greater than 10_000_000_000_000_000");
                        smart_contracts::emit(emit1.clone());
                        return;
                    }
                } else if target_vec[i] == "VALIDATOR_FEE_PERCENTAGE" || target_vec[i] == "BURN_FEE_PERCENTAGE" || target_vec[i] == "TREASURY_FEE_PERCENTAGE" {
                    if amount_u256 > U256::from(100) {
                        let emit1 = format!("Failed: VALIDATOR_FEE_PERCENTAGE, BURN_FEE_PERCENTAGE, and TREASURY_FEE_PERCENTAGE cannot be greater than 100");
                        smart_contracts::emit(emit1.clone());
                        return;
                    }
                }
                else if target_vec[i] == "ATTESTATION_QUORUM" {
                    if amount_u256 > U256::from(100) || amount_u256 < U256::from(51) {
                        let emit1 = format!("Failed: ATTESTATION_QUORUM cannot be greater than 100 or less than 51");
                        smart_contracts::emit(emit1.clone());
                        return;
                    }
                }
                else {
                      if amount_u256 > types::string_to_u256("500000000000000000".to_string()) {
                        let emit1 = format!("Failed: SMART_CONTRACT_EXECUTE_TYPE cannot be greater than 500 000 000 000 000 000");
                        smart_contracts::emit(emit1.clone());
                        return;
                      }
                }
            }

            for i in 0..target_vec.len() {
                let emit1 = format!("{}: {}", target_vec[i].clone(), amount_vec[i].clone());
                smart_contracts::emit(emit1.clone());
                smart_contracts::store_state(target_vec[i].clone(), amount_vec[i].clone());
            }
            let emit1 = format!("Success: Network fees updated");
            smart_contracts::emit(emit1.clone());
        }
    }
}
