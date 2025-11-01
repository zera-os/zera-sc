pub mod zera {
  #[allow(unused_imports)]
  use wasmedge_bindgen::*;
  //use serde::{Serialize, Deserialize};
  pub use wasmedge_bindgen_macro::wasmedge_bindgen; // 'use' is used to import an item into the current module, 'pub use' allows us to (not only import but,) re-export the item.
  
  pub mod types{
    use uint::construct_uint;

    construct_uint! {
        pub struct U256(4);
      }

      pub fn string_to_u256(input: String) -> U256 {
        let input_str = input.as_str();
        match U256::from_dec_str(input_str) {
            Ok(value) => value,
            Err(_) => {
                println!("Error parsing string to U256");
                U256::zero() // Return a default value, such as zero, in case of an error
            }
        }
      }

      pub fn is_valid_u256(input: String) -> bool {
        let input_str = input.as_str();
        U256::from_dec_str(input_str).is_ok()
      }   
  }

  #[derive(PartialEq)]
  pub enum SHAKEHashLength {
    Bits_1024,
    Bits_2048,
    Bits_4096,
  }

  #[derive(PartialEq)]
  pub enum Blake3HashLength {
    Bits_256,
    Bits_512,
    Bits_1024,
    Bits_2048,
    Bits_4096,
    Bits_9001,
  }

  pub mod smart_contracts {
    use crate::zera; // https://doc.rust-lang.org/beta/reference/items/use-declarations.html
    use crate::zera::types::U256;
    use crate::zera::types;

    pub unsafe fn store_state(key_string: String, value_string: String) -> bool {
      let key = key_string.as_str();
      let value = value_string.as_str();

      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;

      let value_pointer = value.as_bytes().as_ptr();
      let value_length = value.len() as i32;
  
      let result = zera::store_state(key_pointer, key_length, value_pointer, value_length);
      if result == 0 {
        return false;
      }

      return true;
    }

    pub unsafe fn delegate_store_state(key_string: String, value_string: String, contract_string: String) -> bool {
      let key = key_string.as_str();
      let value = value_string.as_str();
      let contract = contract_string.as_str();

      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;
      let value_pointer = value.as_bytes().as_ptr();
      let value_length = value.len() as i32;
      let contract_pointer = contract.as_bytes().as_ptr();
      let contract_length = contract.len() as i32;

      let result = zera::delegate_store_state(key_pointer, key_length, value_pointer, value_length, contract_pointer, contract_length);
      if result == 0 {
        return false;
      }

      return true;
    }

    pub unsafe fn clear_state(key_string: String) {
      let key = key_string.as_str();

      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;
  
      zera::clear_state(key_pointer, key_length);
    }

    pub unsafe fn delegate_clear_state(key_string: String, contract_string: String) {
      let key = key_string.as_str();
      let contract = contract_string.as_str();
      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;
      let contract_pointer = contract.as_bytes().as_ptr();
      let contract_length = contract.len() as i32;
      zera::delegate_clear_state(key_pointer, key_length, contract_pointer, contract_length);
    }



    pub unsafe fn retrieve_state(key_string: String) -> String {
      let key = key_string.as_str();

      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::retrieve_state(key_pointer, key_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();


      return res;
    }

    pub unsafe fn delegate_retrieve_state(key_string: String, contract_string: String) -> String {
      let key = key_string.as_str();
      let contract = contract_string.as_str();

      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;
      let contract_pointer = contract.as_bytes().as_ptr();
      let contract_length = contract.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::delegate_retrieve_state(key_pointer, key_length, contract_pointer, contract_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }

    pub unsafe fn get_ace_data(contract: String) -> (bool, U256)  {
  
      let contract_pointer = contract.as_bytes().as_ptr();
      let contract_length = contract.len() as i32;
  
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
  
      let result_len = zera::get_ace_data(contract_pointer, contract_length, target_pointer) as usize;
  
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      // Split the result string into an array of strings using comma as delimiter
      let string_array: Vec<String> = res.split(',').map(|s| s.to_string()).collect();
      let auth_string = string_array[0].clone();
      let mut auth = false;

      if auth_string == "true" {
        auth = true;
      }

      let amount = types::string_to_u256(string_array[1].clone());
  
      return (auth, amount);
   }


    pub unsafe fn db_get_data(key_string: String) -> String {
      let key = key_string.as_str();

      let key_pointer = key.as_bytes().as_ptr();
      let key_length = key.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::db_get_data(key_pointer, key_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }

    pub unsafe fn db_get_any_data(key_string: String, db_key_string: String) -> String
    {
      let key_pointer = key_string.as_bytes().as_ptr();
      let key_length = key_string.len() as i32;

      let db_key_pointer = db_key_string.as_bytes().as_ptr();
      let db_key_length = db_key_string.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::db_get_any_data(key_pointer, key_length, db_key_pointer, db_key_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn call(contract_name: String, nonce: String, function_name: String, parameters: Vec<String>) -> Vec<String> {
      let contract_name_pointer = contract_name.as_bytes().as_ptr();
      let contract_name_length = contract_name.len() as i32;

      let nonce_pointer = nonce.as_bytes().as_ptr();
      let nonce_length = nonce.len() as i32;

      let function_name_pointer = function_name.as_bytes().as_ptr();
      let function_name_length = function_name.len() as i32;
      let binding = parameters.join("##");
      let parameters_combined_in_string = binding.as_str();
      let parameters_pointer = parameters_combined_in_string.as_ptr();
      let parameters_length = parameters_combined_in_string.len() as i32;

      // a struct for storing result
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::call(contract_name_pointer, contract_name_length, nonce_pointer, nonce_length, function_name_pointer, function_name_length, parameters_pointer, parameters_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let results: Vec<String> = res
        .split("[res]")
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches("[end]").to_string())
        .collect();

      return results;
    }

    pub unsafe fn delegatecall(contract_name: String, nonce: String, function_name: String, parameters: Vec<String>) -> Vec<String> {
      let contract_name_pointer = contract_name.as_bytes().as_ptr();
      let contract_name_length = contract_name.len() as i32;

      let nonce_pointer = nonce.as_bytes().as_ptr();
      let nonce_length = nonce.len() as i32;

      let function_name_pointer = function_name.as_bytes().as_ptr();
      let function_name_length = function_name.len() as i32;
      let binding = parameters.join("##");
      let parameters_combined_in_string = binding.as_str();
      let parameters_pointer = parameters_combined_in_string.as_ptr();
      let parameters_length = parameters_combined_in_string.len() as i32;

      // a struct for storing result
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::delegatecall(contract_name_pointer, contract_name_length, nonce_pointer, nonce_length, function_name_pointer, function_name_length, parameters_pointer, parameters_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let results: Vec<String> = res
        .split("[res]")
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches("[end]").to_string())
        .collect();

      return results;
    }

    // pub unsafe fn randomish() -> String {
    //   let mut buffer = Vec::with_capacity(0);
    //   let target_pointer = buffer.as_mut_ptr();

    //   let result_len = zera::randomish(target_pointer) as usize;

    //   buffer.set_len(result_len);
    //   let res = String::from_utf8(buffer).unwrap();
  
    //   return res;
    // }


    // pub unsafe fn balance() -> f32 {
    //   return zera::balance();
    // }

    pub unsafe fn version() -> i32 {
      return zera::version();
    }

    pub unsafe fn emit(value: String) -> bool{
      //let value = value_string.as_str();

      let value_pointer = value.as_bytes().as_ptr();
      let value_length = value.len() as i32;
  
      let result = zera::emit(value_pointer, value_length);

      if result == 0 {
        return false;
      }

      return true;
    }

    pub unsafe fn transfer(contract_id: String, amount: String, address: String) -> bool {
      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;
      
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::transfer(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }
    pub unsafe fn current_hold(contract_id: String, amount: String) -> bool{

      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::current_hold(contract_id_pointer, contract_id_length, amount_pointer, amount_length, target_pointer) as usize;

      buffer.set_len(result_len);
      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;

    }

    pub unsafe fn delegate_send(contract_id: String, amount: String, address: String, sc_wallet: String) -> bool{
      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let sc_wallet_pointer = sc_wallet.as_bytes().as_ptr();
      let sc_wallet_length = sc_wallet.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::delegate_send(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, sc_wallet_pointer, sc_wallet_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn current_send(contract_id: String, amount: String, address: String) -> bool{
      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::current_send(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn current_send_all(wallet_address: String) -> String{
      let address_pointer = wallet_address.as_bytes().as_ptr();
      let address_length = wallet_address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::current_send_all(address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn current_mint(contract_id: String, amount: String, address: String) -> bool{
      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::current_mint(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn delegate_send_all(wallet_address: String, sc_wallet: String) -> String{
      let address_pointer = wallet_address.as_bytes().as_ptr();
      let address_length = wallet_address.len() as i32;

      let sc_wallet_pointer = sc_wallet.as_bytes().as_ptr();
      let sc_wallet_length = sc_wallet.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::delegate_send_all(address_pointer, address_length, sc_wallet_pointer, sc_wallet_length, target_pointer) as usize;

      buffer.set_len(result_len);

      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn delegate_mint(contract_id: String, amount: String, address: String, sc_wallet: String) -> bool{
      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let sc_wallet_pointer = sc_wallet.as_bytes().as_ptr();
      let sc_wallet_length = sc_wallet.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::delegate_mint(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, sc_wallet_pointer, sc_wallet_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn hold(contract_id: String, amount: String) -> bool{

      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::hold(contract_id_pointer, contract_id_length, amount_pointer, amount_length, target_pointer) as usize;

      buffer.set_len(result_len);
      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;

    }

    pub unsafe fn send(contract_id: String, amount: String, address: String) -> bool{
      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::send(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn send_all(wallet_address: String) -> String{
      let address_pointer = wallet_address.as_bytes().as_ptr();
      let address_length = wallet_address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::send_all(address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn send_multi(contract_id: String, input_amounts: String, amounts: Vec<String>, addresses: Vec<String>) -> bool{

      let amount_string = amounts.join(",");
      let address_string = addresses.join(",");
      let input_amounts_string = input_amounts.clone();

      let input_amounts_pointer = input_amounts_string.as_bytes().as_ptr();
      let input_amounts_length = input_amounts_string.len() as i32;
      let amounts_pointer = amount_string.as_bytes().as_ptr();
      let amounts_length = amount_string.len() as i32;
      let addresses_pointer = address_string.as_bytes().as_ptr();
      let addresses_length = address_string.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::send_multi(contract_id_pointer, contract_id_length, input_amounts_pointer, input_amounts_length, amounts_pointer, amounts_length, addresses_pointer, addresses_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn mint(contract_id: String, amount: String, address: String) -> bool{
      let amount_pointer = amount.as_bytes().as_ptr();
      let amount_length = amount.len() as i32;

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let address_pointer = address.as_bytes().as_ptr();
      let address_length = address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::mint(contract_id_pointer, contract_id_length, amount_pointer, amount_length, address_pointer, address_length, target_pointer) as usize;

      buffer.set_len(result_len);

      match String::from_utf8(buffer) {
        Ok(res) => {
            if res == "OK" {
                return true;
            }
        }
        Err(e) => {
            println!("Error converting buffer to String: {}", e);
            return false;
        }
      }
      return false;
    }

    pub unsafe fn last_block_time() -> u64{
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::last_block_time(target_pointer) as usize;
      buffer.set_len(result_len);

      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let time = res.parse::<u64>().unwrap(); 

      return time;
    }

    pub unsafe fn wallet_address() -> String {
      let mut buffer = Vec::with_capacity(1024);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::wallet_address(target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn public_key() -> String {
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::public_key(target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn txn_hash() -> String {
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::txn_hash(target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }

    pub unsafe fn contract_exists(contract_id: String) -> bool {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::contract_exists(contract_name_pointer, contract_name_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let mut exists = false;

      if res == "true" {
        exists = true;
      }

      return exists;
    }

    pub unsafe fn contract_denomination(contract_id: String) -> U256 {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::contract_denomination(contract_name_pointer, contract_name_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let denomination = types::string_to_u256(res);

      return denomination;
    }

    pub unsafe fn circulating_supply(contract_id: String) -> U256 {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::circulating_supply(contract_name_pointer, contract_name_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let circ = types::string_to_u256(res);

      return circ;
    }

    pub unsafe fn wallet_tokens(wallet_address: String) -> Vec<String>  {
      let wallet_address_pointer = wallet_address.as_bytes().as_ptr();
      let wallet_address_length = wallet_address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::wallet_tokens(wallet_address_pointer, wallet_address_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let tokens: Vec<String> = res.split(',')
                                .map(|s| s.to_string())
                                .collect();

      return tokens;
    }
    pub unsafe fn smart_contract_balance(contract_id: String) -> U256 {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::smart_contract_balance(contract_name_pointer, contract_name_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let balance = types::string_to_u256(res);

      return balance;
    }

    pub unsafe fn wallet_balance(contract_id: String, wallet_address: String) -> U256 {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let wallet_pointer = wallet_address.as_bytes().as_ptr();
      let wallet_length = wallet_address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::wallet_balance(contract_name_pointer, contract_name_length, wallet_pointer, wallet_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      let balance = types::string_to_u256(res);

      return balance;
    }

    pub unsafe fn compliance(contract_id: String, wallet_address: String) -> bool {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let wallet_pointer = wallet_address.as_bytes().as_ptr();
      let wallet_length = wallet_address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::compliance(contract_name_pointer, contract_name_length, wallet_pointer, wallet_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      if(res == "true"){
        return true;
      }
      else{
        return false;
      }
    }

    pub unsafe fn compliance_levels(contract_id: String, wallet_address: String) -> Vec<u32> {
      let contract_name_pointer = contract_id.as_bytes().as_ptr();
      let contract_name_length = contract_id.len() as i32;

      let wallet_pointer = wallet_address.as_bytes().as_ptr();
      let wallet_length = wallet_address.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::compliance_levels(contract_name_pointer, contract_name_length, wallet_pointer, wallet_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      if res == "0" {
        return vec![];
      }

      let levels: Vec<u32> = res
        .split(',')
        .filter_map(|s| s.parse::<u32>().ok()) // Convert to u32 and filter out invalid entries
        .collect();

      return levels;
    }
    
    pub unsafe fn smart_contract_wallet() -> String {

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::smart_contract_wallet(target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }

    pub unsafe fn current_smart_contract_wallet() -> String {

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::current_smart_contract_wallet(target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }

    pub unsafe fn called_smart_contract_wallet() -> String {

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::called_smart_contract_wallet(target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }

    pub unsafe fn vote_options(proposal_id: String, support: u32) -> String {
      let proposal_id_pointer = proposal_id.as_bytes().as_ptr();
      let proposal_id_length = proposal_id.len() as i32;
  
      // Convert `support` (a digit) into a string
      let support_string = support.to_string();
      let support_pointer = support_string.as_bytes().as_ptr();
      let support_length = support_string.len() as i32;
  
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
  
      let result_len = zera::vote(
          proposal_id_pointer,
          proposal_id_length,
          support_pointer,
          support_length,
          target_pointer,
      ) as usize;
  
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }
    pub unsafe fn vote(proposal_id: String, support: bool) -> String {
      let proposal_id_pointer = proposal_id.as_bytes().as_ptr();
      let proposal_id_length = proposal_id.len() as i32;
  
      // Convert the bool `support` to a String ("true" or "false")
      let support_string = if support { "true" } else { "false" };
      let support_pointer = support_string.as_bytes().as_ptr();
      let support_length = support_string.len() as i32;
  
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
  
      let result_len = zera::vote(
          proposal_id_pointer,
          proposal_id_length,
          support_pointer,
          support_length,
          target_pointer,
      ) as usize;
  
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
  
      return res;
    }
    pub unsafe fn expense_ratio(contract_id: String, output_address: String, addresses: Vec<String>) -> String{ 
      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let output_address_pointer = output_address.as_bytes().as_ptr();
      let output_address_length = output_address.len() as i32;

      let binding = addresses.join("##");
      let addresses_combined_in_string = binding.as_str();
      let addresses_pointer = addresses_combined_in_string.as_ptr();
      let addresses_length = addresses_combined_in_string.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::expense_ratio(contract_id_pointer, contract_id_length, addresses_pointer, addresses_length, output_address_pointer, output_address_length, target_pointer) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }


    // leave allowed equiv/amount | period months/seconds if not needed
    pub unsafe fn allowance(
      contract_id: String, 
      wallet_address: String, 
      mut allowed_equiv: String, 
      mut allowed_amount: String, 
      mut period_months: String, 
      mut period_seconds: String, 
      start_time: String) -> String{ 

      let contract_id_pointer = contract_id.as_bytes().as_ptr();
      let contract_id_length = contract_id.len() as i32;

      let wallet_address_pointer = wallet_address.as_bytes().as_ptr();
      let wallet_address_length = wallet_address.len() as i32;
      // Check if allowed_equiv is empty and change its value
      if allowed_equiv.is_empty() {
        allowed_equiv = "N/A".to_string(); // Replace "default_value" with your desired value
      }
      let allowed_equiv_pointer = allowed_equiv.as_bytes().as_ptr();
      let allowed_equiv_length = allowed_equiv.len() as i32;

      if allowed_amount.is_empty() {
        allowed_amount = "N/A".to_string(); // Replace "default_value" with your desired value
      }
      let allowed_amount_pointer = allowed_amount.as_bytes().as_ptr();
      let allowed_amount_length = allowed_amount.len() as i32;

      if period_months.is_empty() {
        period_months = "N/A".to_string(); // Replace "default_value" with your desired value
      }
      let period_months_pointer = period_months.as_bytes().as_ptr();
      let period_months_length = period_months.len() as i32;

      if period_seconds.is_empty() {
        period_seconds = "N/A".to_string(); // Replace "default_value" with your desired value
      }
      let period_seconds_pointer = period_seconds.as_bytes().as_ptr();
      let period_seconds_length = period_seconds.len() as i32;

      let start_time_pointer = start_time.as_bytes().as_ptr();
      let start_time_length = start_time.len() as i32;

      let authorize = "true".to_string(); // Assuming authorization is always true
      let authorize_pointer = authorize.as_bytes().as_ptr();
      let authorize_length = authorize.len() as i32;

      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();

      let result_len = zera::allowance(
          contract_id_pointer,
          contract_id_length,
          wallet_address_pointer,
          wallet_address_length,
          allowed_equiv_pointer,
          allowed_equiv_length,
          allowed_amount_pointer,
          allowed_amount_length,
          period_months_pointer,
          period_months_length,
          period_seconds_pointer,
          period_seconds_length,
          start_time_pointer,
          start_time_length,
          authorize_pointer,
          authorize_length,
          target_pointer
      ) as usize;

      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();

      return res;
    }
  
  
  pub unsafe fn allowance_sender(
    contract_id: String, 
    wallet_address: String, 
    mut allowed_equiv: String, 
    mut allowed_amount: String, 
    mut period_months: String, 
    mut period_seconds: String, 
    start_time: String) -> String{ 

    let contract_id_pointer = contract_id.as_bytes().as_ptr();
    let contract_id_length = contract_id.len() as i32;

    let wallet_address_pointer = wallet_address.as_bytes().as_ptr();
    let wallet_address_length = wallet_address.len() as i32;
    // Check if allowed_equiv is empty and change its value
    if allowed_equiv.is_empty() {
      allowed_equiv = "N/A".to_string(); // Replace "default_value" with your desired value
    }
    let allowed_equiv_pointer = allowed_equiv.as_bytes().as_ptr();
    let allowed_equiv_length = allowed_equiv.len() as i32;

    if allowed_amount.is_empty() {
      allowed_amount = "N/A".to_string(); // Replace "default_value" with your desired value
    }
    let allowed_amount_pointer = allowed_amount.as_bytes().as_ptr();
    let allowed_amount_length = allowed_amount.len() as i32;

    if period_months.is_empty() {
      period_months = "N/A".to_string(); // Replace "default_value" with your desired value
    }
    let period_months_pointer = period_months.as_bytes().as_ptr();
    let period_months_length = period_months.len() as i32;

    if period_seconds.is_empty() {
      period_seconds = "N/A".to_string(); // Replace "default_value" with your desired value
    }
    let period_seconds_pointer = period_seconds.as_bytes().as_ptr();
    let period_seconds_length = period_seconds.len() as i32;

    let start_time_pointer = start_time.as_bytes().as_ptr();
    let start_time_length = start_time.len() as i32;

    let authorize = "true".to_string(); // Assuming authorization is always true
    let authorize_pointer = authorize.as_bytes().as_ptr();
    let authorize_length = authorize.len() as i32;

    let mut buffer = Vec::with_capacity(0);
    let target_pointer = buffer.as_mut_ptr();

    let result_len = zera::allowance_sender(
        contract_id_pointer,
        contract_id_length,
        wallet_address_pointer,
        wallet_address_length,
        allowed_equiv_pointer,
        allowed_equiv_length,
        allowed_amount_pointer,
        allowed_amount_length,
        period_months_pointer,
        period_months_length,
        period_seconds_pointer,
        period_seconds_length,
        start_time_pointer,
        start_time_length,
        authorize_pointer,
        authorize_length,
        target_pointer
    ) as usize;

    buffer.set_len(result_len);
    let res_ = String::from_utf8(buffer).unwrap();
    let res = res_.clone();

    return res;
  }

  pub unsafe fn allowance_sender_deauthorize(contract_id: String, wallet_address: String) -> String{ 

    let contract_id_pointer = contract_id.as_bytes().as_ptr();
    let contract_id_length = contract_id.len() as i32;

    let wallet_address_pointer = wallet_address.as_bytes().as_ptr();
    let wallet_address_length = wallet_address.len() as i32;

    let mut allowed_equiv = "N/A".to_string(); // Assuming allowed_equiv is not needed for deauthorization
    let allowed_equiv_pointer = allowed_equiv.as_bytes().as_ptr();
    let allowed_equiv_length = allowed_equiv.len() as i32;

    let mut allowed_amount = "N/A".to_string(); // Assuming allowed_amount is not needed for deauthorization
    let allowed_amount_pointer = allowed_amount.as_bytes().as_ptr();
    let allowed_amount_length = allowed_amount.len() as i32;

    let mut period_months = "N/A".to_string(); // Assuming period_months is not needed for deauthorization
    let period_months_pointer = period_months.as_bytes().as_ptr();
    let period_months_length = period_months.len() as i32;

    let mut period_seconds = "N/A".to_string(); // Assuming period_seconds is not needed for deauthorization
    let period_seconds_pointer = period_seconds.as_bytes().as_ptr();
    let period_seconds_length = period_seconds.len() as i32;

    let start_time = "N/A".to_string(); // Assuming start_time is not needed for deauthorization
    let start_time_pointer = start_time.as_bytes().as_ptr();
    let start_time_length = start_time.len() as i32;

    let authorize = "false".to_string(); // Assuming authorization is always true
    let authorize_pointer = authorize.as_bytes().as_ptr();
    let authorize_length = authorize.len() as i32;

    let mut buffer = Vec::with_capacity(0);
    let target_pointer = buffer.as_mut_ptr();

    let result_len = zera::allowance_sender(
        contract_id_pointer,
        contract_id_length,
        wallet_address_pointer,
        wallet_address_length,
        allowed_equiv_pointer,
        allowed_equiv_length,
        allowed_amount_pointer,
        allowed_amount_length,
        period_months_pointer,
        period_months_length,
        period_seconds_pointer,
        period_seconds_length,
        start_time_pointer,
        start_time_length,
        authorize_pointer,
        authorize_length,
        target_pointer
    ) as usize;

    buffer.set_len(result_len);
    let res_ = String::from_utf8(buffer).unwrap();
    let res = res_.clone();

    return res;
  }
  pub unsafe fn authorized_currency_equiv(contract_ids: String, rates: String, authorized: String, max_stakes: String) -> String{
    let contract_ids_pointer = contract_ids.as_bytes().as_ptr();
    let contract_ids_length = contract_ids.len() as i32;

    let rates_pointer = rates.as_bytes().as_ptr();
    let rates_length = rates.len() as i32;

    let authorized_pointer = authorized.as_bytes().as_ptr();
    let authorized_length = authorized.len() as i32;

    let max_stakes_pointer = max_stakes.as_bytes().as_ptr();
    let max_stakes_length = max_stakes.len() as i32;

    let mut buffer = Vec::with_capacity(0);
    let target_pointer = buffer.as_mut_ptr();
    
    let result_len = zera::authorized_currency_equiv(contract_ids_pointer, contract_ids_length, rates_pointer, rates_length, authorized_pointer, authorized_length, max_stakes_pointer, max_stakes_length, target_pointer) as usize;

    buffer.set_len(result_len);
    let res_ = String::from_utf8(buffer).unwrap();
    let res = res_.clone();

    return res;
  }

  pub unsafe fn instrument_contract_bridge(symbol: String, name: String, denomination: String, contract_id: String, mint_id: String, uri: String, authorized_key: String, wallet: String, amount: String) -> String{
    let symbol_pointer = symbol.as_bytes().as_ptr();
    let symbol_length = symbol.len() as i32;
    let name_pointer = name.as_bytes().as_ptr();
    let name_length = name.len() as i32;
    let denomination_pointer = denomination.as_bytes().as_ptr();
    let denomination_length = denomination.len() as i32;

    let contract_id_pointer = contract_id.as_bytes().as_ptr();
    let contract_id_length = contract_id.len() as i32;

    let mint_id_pointer = mint_id.as_bytes().as_ptr();
    let mint_id_length = mint_id.len() as i32;

    let mut mut_uri = uri.clone();
    if mut_uri.is_empty() {
      mut_uri = "N/A".to_string();
    }
    let uri_pointer = mut_uri.as_bytes().as_ptr();
    let uri_length = mut_uri.len() as i32;

    let mut mut_authorized_key = authorized_key.clone();
    if mut_authorized_key.is_empty() {
      mut_authorized_key = "N/A".to_string();
    }
    let authorized_key_pointer = mut_authorized_key.as_bytes().as_ptr();
    let authorized_key_length = mut_authorized_key.len() as i32;

    let wallet_pointer = wallet.as_bytes().as_ptr();
    let wallet_length = wallet.len() as i32;

    let amount_pointer = amount.as_bytes().as_ptr();
    let amount_length = amount.len() as i32;

    let mut buffer = Vec::with_capacity(0);
    let target_pointer = buffer.as_mut_ptr();

    let result_len = zera::instrument_contract_bridge(symbol_pointer, symbol_length, 
      name_pointer, name_length, 
      denomination_pointer, denomination_length, 
      contract_id_pointer, contract_id_length, 
      mint_id_pointer, mint_id_length, 
      uri_pointer, uri_length, 
      authorized_key_pointer, authorized_key_length, 
      wallet_pointer, wallet_length, 
      amount_pointer, amount_length, target_pointer) as usize;

    buffer.set_len(result_len);
    let res_ = String::from_utf8(buffer).unwrap();
    let res = res_.clone();

    return res;
  }
  pub unsafe fn verify_signature(message: String, signatures: String, public_key: String) -> bool {
    let message_pointer = message.as_bytes().as_ptr();
    let message_length = message.len() as i32;
    let signatures_pointer = signatures.as_bytes().as_ptr();
    let signatures_length = signatures.len() as i32;
    let public_key_pointer = public_key.as_bytes().as_ptr();
    let public_key_length = public_key.len() as i32;
    let mut buffer = Vec::with_capacity(0);
    let target_pointer = buffer.as_mut_ptr();

    let result_len = zera::verify_signature(message_pointer, message_length, signatures_pointer, signatures_length, public_key_pointer, public_key_length, target_pointer) as usize;

    buffer.set_len(result_len);
    let res_ = String::from_utf8(buffer).unwrap();
    let res = res_.clone();

    if res == "true" {
      return true;
    }
    else {
      return false;
    }
  }

    pub unsafe fn sha256(data: String) -> String {
      let data_pointer = data.as_bytes().as_ptr();
      let data_length = data.len() as i32;
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
      let hash_type = "sha256".as_bytes().as_ptr();
      let hash_type_length = "sha256".len() as i32;

      let result_len = zera::hash(data_pointer, data_length, hash_type, hash_type_length, target_pointer) as usize;
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
      return res;
    }

    pub unsafe fn sha512(data: String) -> String {
      let data_pointer = data.as_bytes().as_ptr();
      let data_length = data.len() as i32;
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
      let hash_type = "sha512".as_bytes().as_ptr();
      let hash_type_length = "sha512".len() as i32;

      let result_len = zera::hash(data_pointer, data_length, hash_type, hash_type_length, target_pointer) as usize;
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
      return res;
    }

    pub unsafe fn blake3(data: String, length: crate::zera::Blake3HashLength) -> String {

      let mut hash_type = "".to_string();

      if length == crate::zera::Blake3HashLength::Bits_256 {
        hash_type = "blake3_256".to_string();
      }
      else if length == crate::zera::Blake3HashLength::Bits_512 {
        hash_type = "blake3_512".to_string();
      }
      else if length == crate::zera::Blake3HashLength::Bits_1024 {
        hash_type = "blake3_1024".to_string();
      }
      else if length == crate::zera::Blake3HashLength::Bits_2048 {
        hash_type = "blake3_2048".to_string();
      }
      else if length == crate::zera::Blake3HashLength::Bits_4096 {
        hash_type = "blake3_4096".to_string();
      }
      else if length == crate::zera::Blake3HashLength::Bits_9001 {
        hash_type = "blake3_9001".to_string();
      }
      else {
        return "error".to_string();
      }

      let data_pointer = data.as_bytes().as_ptr();
      let data_length = data.len() as i32;
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
      let hash_type_pointer = hash_type.as_bytes().as_ptr();
      let hash_type_length = hash_type.len() as i32;

      let result_len = zera::hash(data_pointer, data_length, hash_type_pointer, hash_type_length, target_pointer) as usize;
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
      return res;
    }


    pub unsafe fn shake(data: String, length: crate::zera::SHAKEHashLength) -> String {

      let mut hash_type = "".to_string();

      if length == crate::zera::SHAKEHashLength::Bits_1024 {
        hash_type = "shake_1024".to_string();
      }
      else if length == crate::zera::SHAKEHashLength::Bits_2048 {
        hash_type = "shake_2048".to_string();
      }
      else if length == crate::zera::SHAKEHashLength::Bits_4096 {
        hash_type = "shake_4096".to_string();
      }
      else {
        return "error".to_string();
      }

      let data_pointer = data.as_bytes().as_ptr();
      let data_length = data.len() as i32;
      let mut buffer = Vec::with_capacity(0);
      let target_pointer = buffer.as_mut_ptr();
      let hash_type_pointer = hash_type.as_bytes().as_ptr();
      let hash_type_length = hash_type.len() as i32;

      let result_len = zera::hash(data_pointer, data_length, hash_type_pointer, hash_type_length, target_pointer) as usize;
      buffer.set_len(result_len);
      let res_ = String::from_utf8(buffer).unwrap();
      let res = res_.clone();
      return res;
    }


}

  // #[repr(C)]
  // #[derive(Serialize, Deserialize, Debug)]
  // pub struct ZeraStatus {
  //   pub code: i32,
  //   pub txn_status: i32,
  // }

  #[link(wasm_import_module = "native_functions")]
  extern "C" {
      //*****************************
      // Zera txn functions
      //*****************************
      //original sc functions (these will take the value of the original smart contract)
      pub fn authorized_currency_equiv(contract_ids: *const u8, contract_ids_length: i32, rates: *const u8, rates_length: i32, authorized: *const u8, authorized_length: i32, max_stakes: *const u8, max_stakes_length: i32, target_pointer: *const u8) -> i32;
      pub fn transfer(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn hold(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, target_pointer: *const u8) -> i32;
      pub fn send(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn send_all(address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn mint(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn send_multi(contract_id: *const u8, contract_id_length: i32, input_amounts_pointer: *const u8, input_amounts_length: i32, amounts_pointer: *const u8, amounts_length: i32, addresses_pointer: *const u8, addresses_length: i32, target_pointer: *const u8) -> i32;

      //any sc functions (user can specify any smart contract value, will be verified by function if it is in stack)
      pub fn delegate_send(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, sc_wallet: *const u8, sc_wallet_length: i32,target_pointer: *const u8) -> i32;
      pub fn delegate_send_all(address_pointer: *const u8, address_length: i32, sc_wallet: *const u8, sc_wallet_length: i32,target_pointer: *const u8) -> i32;
      pub fn delegate_mint(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, sc_wallet: *const u8, sc_wallet_length: i32,target_pointer: *const u8) -> i32;
      pub fn delegate_hold(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, sc_wallet: *const u8, sc_wallet_length: i32,target_pointer: *const u8) -> i32;

      //current sc functions (this will take the values of the latest sc on the stack)
      pub fn current_hold(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, target_pointer: *const u8) -> i32;
      pub fn current_send(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn current_send_all(address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn current_mint(contract_id: *const u8, contract_id_length: i32, amount_pointer: *const u8, amount_length: i32, address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      

      pub fn instrument_contract_bridge(symbol: *const u8, symbol_length: i32, name: *const u8, name_length: i32, denomination: *const u8, denomination_length: i32, contract_id: *const u8, contract_id_length: i32, mint_id: *const u8, mint_id_length: i32, uri: *const u8, uri_length: i32, authorized_key: *const u8, authorized_key_length: i32, wallet: *const u8, wallet_length: i32, amount: *const u8, amount_length: i32, target_pointer: *const u8) -> i32;
      //***************************** 
      //utils
      //*****************************
      pub fn hash(data_pointer: *const u8, data_length: i32, hash_type_pointer: *const u8, hash_type_length: i32, target_pointer: *const u8) -> i32;
      pub fn public_key(target_pointer: *const u8) -> i32;
      pub fn txn_hash(target_pointer: *const u8) -> i32;
      pub fn wallet_address(target_pointer: *const u8) -> i32;
      pub fn last_block_time(target_pointer: *const u8) -> i32;
      pub fn wallet_tokens(address_pointer: *const u8, address_length: i32, target_pointer: *const u8) -> i32;
      pub fn contract_exists(contract_name_pointer: *const u8, contract_name_length: i32, target_pointer: *const u8) -> i32;
      pub fn contract_denomination(contract_name_pointer: *const u8, contract_name_length: i32, target_pointer: *const u8) -> i32;
      pub fn wallet_balance(contract_name_pointer: *const u8, contract_name_length: i32, wallet_pointer: *const u8, wallet_length: i32, target_pointer: *const u8) ->i32;
      pub fn circulating_supply(contract_name_pointer: *const u8, contract_name_length: i32, target_pointer: *const u8) ->i32;
      pub fn supply_data(contract_name_pointer: *const u8, contract_name_length: i32, target_pointer: *const u8) -> i32;
      pub fn compliance(contract_name_pointer: *const u8, contract_name_length: i32, wallet_pointer: *const u8, wallet_length: i32, target_pointer: *const u8) -> i32;
      pub fn compliance_levels(contract_name_pointer: *const u8, contract_name_length: i32, wallet_pointer: *const u8, wallet_length: i32, target_pointer: *const u8) -> i32;
      pub fn allowance(contract_id_pointer: *const u8, contract_id_length: i32, wallet_pointer: *const u8, wallet_length: i32, 
        allowed_equiv_pointer: *const u8, allowed_equiv_length: i32, allowed_amount_pointer: *const u8, allowed_amount_length: i32,
      months_pointer: *const u8, months_length: i32, seconds_pointer: *const u8, seconds_length: i32,
    start_pointer: *const u8, start_length: i32, authorize_pointer: *const u8, authorize_length: i32, target_pointer: *const u8) -> i32;

    pub fn allowance_sender(contract_id_pointer: *const u8, contract_id_length: i32, wallet_pointer: *const u8, wallet_length: i32, 
      allowed_equiv_pointer: *const u8, allowed_equiv_length: i32, allowed_amount_pointer: *const u8, allowed_amount_length: i32,
    months_pointer: *const u8, months_length: i32, seconds_pointer: *const u8, seconds_length: i32,
  start_pointer: *const u8, start_length: i32, authorize_pointer: *const u8, authorize_length: i32, target_pointer: *const u8) -> i32;

  pub fn allowance_current(contract_id_pointer: *const u8, contract_id_length: i32, wallet_pointer: *const u8, wallet_length: i32, 
    allowed_equiv_pointer: *const u8, allowed_equiv_length: i32, allowed_amount_pointer: *const u8, allowed_amount_length: i32,
  months_pointer: *const u8, months_length: i32, seconds_pointer: *const u8, seconds_length: i32,
start_pointer: *const u8, start_length: i32, authorize_pointer: *const u8, authorize_length: i32, target_pointer: *const u8) -> i32; 

      //original sc functions (these will take the value of the original smart contract)
      pub fn smart_contract_balance(contract_name_pointer: *const u8, contract_name_length: i32, target_pointer: *const u8) -> i32;
      pub fn smart_contract_wallet(target_pointer: *const u8) -> i32;
      pub fn verify_signature(message_pointer: *const u8, message_length: i32, signatures_pointer: *const u8, signatures_length: i32, public_key_pointer: *const u8, public_key_length: i32, target_pointer: *const u8) -> i32;

      //current sc functions (this will take the values of the latest sc on the stack)
      pub fn current_smart_contract_balance(contract_name_pointer: *const u8, contract_name_length: i32, target_pointer: *const u8) -> i32;
      pub fn current_smart_contract_wallet(target_pointer: *const u8) -> i32;
      pub fn called_smart_contract_wallet(target_pointer: *const u8) -> i32;

      pub fn vote(proposal_pointer: *const u8, proposal_length: i32, support_pointer: *const u8, support_length: i32, target_pointer: *const u8) -> i32;
      pub fn expense_ratio(contract_name_pointer: *const u8, contract_name_length: i32, addresses_pointer: *const u8, addresses_length: i32, 
        output_address_pointer: *const u8, output_address_length: i32, target_pointer: *const u8) -> i32;
      //pub fn randomish(target_pointer: *const u8) -> i32;
      pub fn version() -> i32;

      //needed for state management
      pub fn call(contract_name_pointer: *const u8, contract_name_length: i32, nonce_pointer: *const u8, nonce_length: i32, function_name_pointer: *const u8, function_name_length: i32, parameters_pointer: *const u8, parameters_length: i32, target_pointer: *const u8) -> i32;
      pub fn delegatecall(contract_name_pointer: *const u8, contract_name_length: i32, nonce_pointer: *const u8, nonce_length: i32, function_name_pointer: *const u8, function_name_length: i32, parameters_pointer: *const u8, parameters_length: i32, target_pointer: *const u8) -> i32;
      pub fn store_state(key_pointer: *const u8, key_length: i32, value_pointer: *const u8, value_length: i32) -> i32;
      pub fn retrieve_state(key_pointer: *const u8, key_length: i32, target_pointer: *const u8) -> i32;
      pub fn clear_state(key_pointer: *const u8, key_length: i32) -> i32;
      pub fn delegate_store_state(key_pointer: *const u8, key_length: i32, value_pointer: *const u8, value_length: i32, contract_pointer: *const u8, contract_length: i32) -> i32;
      pub fn delegate_retrieve_state(key_pointer: *const u8, key_length: i32, contract_pointer: *const u8, contract_length: i32, target_pointer: *const u8) -> i32;
      pub fn delegate_clear_state(key_pointer: *const u8, key_length: i32, contract_pointer: *const u8, contract_length: i32) -> i32;
      pub fn db_get_any_data(key_pointer: *const u8, key_length: i32, db_key_pointer: *const u8, db_key_length: i32, target_pointer: *const u8) -> i32;
      pub fn db_get_data(key_pointer: *const u8, key_length: i32, target_pointer: *const u8) -> i32;
      pub fn get_ace_data(contract_pointer: *const u8, contract_length: i32, target_pointer: *const u8) -> i32;
      pub fn emit(value_pointer: *const u8, value_length: i32) -> i32;
  }
}