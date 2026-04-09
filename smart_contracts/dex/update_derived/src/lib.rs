pub mod dex_update_v2 {
    use native_functions::zera::smart_contracts;
    use native_functions::zera::wasmedge_bindgen;
    use native_txns::*;
    use serde::{Deserialize, Serialize};

    const PROXY_CONTRACT: &str = "zera_dex_proxy_1";
    const SMART_CONTRACT_KEY: &str = "SMART_CONTRACT_";
    const PROXY_WALLET: &str = "3uct7y6rcxW3KA8o8b2gqtaygw7hA39P3SyjV466fXWP"; //sc_zera_dex_proxy_1

    const FINISHED_KEY: &str = "FINISHED";
    const OLD_DERIVED_WALLET: &str = "Gp4tWJANR2bb6UZyX4uixTo8T6KgmWppc92dFQK4L3nk";
    const NEW_WALLET: &str = "A4u1EV7MyAeWbhiLcgBs6S6L2s2uAwcSsqjUk7ksSvSh";
    const ZRA_CONTRACT: &str = "$ZRA+0000";
    const USDC_CONTRACT: &str = "$sol-USDC+000000";
    const PROXY_CONTRACT_NAME: &str = "zera_dex_proxy";
    const PROXY_CONTRACT_INSTANCE: &str = "1";

    #[wasmedge_bindgen]
    pub fn init() {
    }

    #[wasmedge_bindgen]
    pub fn transfer_derived_funds() {
        unsafe{
            if smart_contracts::retrieve_state(FINISHED_KEY.to_string()) == "true" {
                smart_contracts::emit("Failed: Already finished".to_string());
                return;
            }

            if !check_auth() {
                smart_contracts::emit("Failed: Must be called by proxy wallet".to_string());
                return;
            }

            let zra_balance = smart_contracts::wallet_balance(ZRA_CONTRACT.to_string(), OLD_DERIVED_WALLET.to_string());
            let usdc_balance = smart_contracts::wallet_balance(USDC_CONTRACT.to_string(), OLD_DERIVED_WALLET.to_string());

            if zra_balance > native_functions::zera::types::U256::zero() {
                if !smart_contracts::derived_delegate_send(
                    ZRA_CONTRACT.to_string(),
                    zra_balance.to_string(),
                    NEW_WALLET.to_string(),
                    OLD_DERIVED_WALLET.to_string(),
                    PROXY_CONTRACT_NAME.to_string(),
                    PROXY_CONTRACT_INSTANCE.to_string(),
                ) {
                    smart_contracts::emit("Failed: Could not transfer ZRA".to_string());
                    return;
                }
            }

            if usdc_balance > native_functions::zera::types::U256::zero() {
                if !smart_contracts::derived_delegate_send(
                    USDC_CONTRACT.to_string(),
                    usdc_balance.to_string(),
                    NEW_WALLET.to_string(),
                    OLD_DERIVED_WALLET.to_string(),
                    PROXY_CONTRACT_NAME.to_string(),
                    PROXY_CONTRACT_INSTANCE.to_string(),
                ) {
                    smart_contracts::emit("Failed: Could not transfer USDC".to_string());
                    return;
                }
            }

            smart_contracts::store_state(FINISHED_KEY.to_string(), "true".to_string());

            smart_contracts::emit("DERIVED_FUNDS_TRANSFERRED".to_string());
            smart_contracts::emit(format!("zra_transferred: {}", zra_balance.to_string()));
            smart_contracts::emit(format!("usdc_transferred: {}", usdc_balance.to_string()));
        }
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

    fn save_state<T: Serialize>(key: &str, data: &T) -> bool {
        let bytes = postcard::to_allocvec(data).unwrap();
        let b64 = base64::encode(bytes);

        unsafe { smart_contracts::delegate_store_state(key.to_string(), b64.to_string(), PROXY_CONTRACT.to_string()) }
    }


    #[derive(Serialize, Deserialize)]
    pub struct SmartContractState {
        pub smart_contract: String,
        pub instance: String,
    }


}
