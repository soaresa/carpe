use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use crate::carpe_error::CarpeError;
use crate::stream::stream_key_manager;
use crate::stream::chains::osmosis::{osmosis_key_service, osmosis_account_service};
use crate::stream::chains::coin::{Coin, CoinAmount};
use crate::stream::chains::chain::ChainType;
use crate::stream::stream_configs::get_config_path;

// Define the Account struct with balances
#[derive(Serialize, Deserialize, Clone)]
pub struct Account {
  pub chain: ChainType,
  pub address: String,
  pub public_key: String,
  pub note: Option<String>,
  pub balances: Vec<CoinAmount>,
}

// Function to restore an account from a mnemonic
pub async fn restore_account(mnemonic: &str, chain_type: ChainType, note: Option<String>) -> Result<Account, CarpeError> {
  // Derive address, public key, and private key based on chain type
  let (address, public_key, private_key) = match chain_type {
    ChainType::Osmosis => match osmosis_key_service::derive_account(mnemonic) {
      Ok(key) => key,
      Err(_e) => {
        println!("Failed to generate account information");
        return Err(CarpeError::misc("Failed to generate account information"));
      }
    },
    // Add other chains here
  };

  // Load the existing accounts from the JSON file
  let accounts = load_accounts()?;
  
  // Check if the account already exists
  if is_account_registered(accounts, chain_type, &address) {
    return Err(CarpeError::misc("Account already registered"));
  }

  // Fetch balances for wLibra and USDC
  let balances = match chain_type {
    ChainType::Osmosis => {
      match osmosis_account_service::fetch_balances(&address, Some(vec![Coin::WLibra, Coin::USDC])).await {
        Ok(balances) => balances,
        Err(e) => {
          eprintln!("Error fetching balances: {}", e);
          return Err(CarpeError::misc(&format!("Failed to fetch balances: {}", e)));
        }
      }
    }
  };

  // Instantiate a new account with balances
  let account = Account {
    chain: chain_type,
    address: address.clone(),
    public_key,
    note,
    balances,
  };

  // Store the account information in a JSON file
  store_account(&account)?;

  // Store the private key associated with the address
  stream_key_manager::store_private_key(&chain_type.to_string(), &address, &private_key).map_err(|_e| CarpeError::misc(&format!("Failed to store private key")))?;

  // Return the instance of the account
  Ok(account)
}

pub async fn get_accounts() -> Result<Vec<Account>, CarpeError> {
  // Load all existing accounts
  let mut accounts = load_accounts()?;

  // Iterate over each account to fetch and update balances
  for account in &mut accounts {
    // Fetch the balances based on the chain type
    let balances = match account.chain {
      ChainType::Osmosis => osmosis_account_service::fetch_balances(&account.address, None).await.map_err(|_e| CarpeError::misc(&format!("Failed to fetch balances")))?,
      // Add additional chain types here as needed
      //_ => return Err(CarpeError::misc("Unsupported chain type")),
    };

    // Update the account's balances
    account.balances = balances;
  }

  // Store the updated accounts back into the JSON file
  store_accounts(&accounts)?;

  // Return the accounts with updated balances
  Ok(accounts)
}

pub fn remove_account(address: &str, chain: ChainType) -> Result<(), CarpeError> {
  // Load all existing accounts
  let mut accounts = load_accounts()?;

  // Find the index of the account to be removed based on the address and chain type
  if let Some(index) = accounts.iter().position(|account| account.chain == chain && account.address == address) {
      // Remove the account from the list
      accounts.remove(index);

      // Save the updated list of accounts back to the JSON file
      store_accounts(&accounts)?;

      // Remove the private key associated with the address
      stream_key_manager::erase_private_key(&chain.to_string(), address).map_err(|_e| CarpeError::misc(&format!("Failed to remove private key")))?; 

      Ok(())
  } else {
      // If the account is not found, return an error
      Err(CarpeError::misc("Account not found"))
  }
}

// Function to load all accounts registered in the application
fn load_accounts() -> Result<Vec<Account>, CarpeError> {
  let file_path = get_accounts_file_path()?;
  
  // Check if the file exists; if not, return an empty vector
  if !file_path.exists() {
    return Ok(Vec::new());
  }

  let file = File::open(file_path).map_err(|e| CarpeError::misc(&format!("Failed to open accounts file: {}", e)))?;
  let accounts: Vec<Account> = serde_json::from_reader(file).map_err(|e| CarpeError::misc(&format!("Failed to parse accounts JSON: {}", e)))?;
  
  Ok(accounts)
}

fn is_account_registered(accounts: Vec<Account>, chain: ChainType, address: &str) -> bool {
  accounts.iter().any(|account| account.chain == chain && account.address == address)
}

/// Function to store accounts in the JSON file
fn store_accounts(accounts: &[Account]) -> Result<(), CarpeError> {
  let file_path = get_accounts_file_path()?;
  let file = OpenOptions::new().write(true).create(true).truncate(true).open(file_path)
    .map_err(|e| CarpeError::misc(&format!("Failed to open accounts file for writing: {}", e)))?;
  
  serde_json::to_writer_pretty(file, accounts)
    .map_err(|e| CarpeError::misc(&format!("Failed to write accounts to JSON: {}", e)))?;
  
  Ok(())
}

// Private function to store account information in a JSON file
fn store_account(account: &Account) -> Result<(), CarpeError> {
    let file_path = get_accounts_file_path()?;
    let mut accounts: Vec<Account> = match File::open(&file_path) {
        Ok(mut file) => {
            let mut contents = String::new();
            file
              .read_to_string(&mut contents)
              .map_err(|e| CarpeError::misc(&format!("Failed to read file: {}", e)))?;
            serde_json::from_str(&contents).unwrap_or_else(|_| Vec::new())
        }
        Err(_) => Vec::new(),
    };

    accounts.push(account.clone());

    let serialized_data = serde_json::to_string_pretty(&accounts)
        .map_err(|e| CarpeError::misc(&format!("Failed to serialize accounts: {}", e)))?;

    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_path)
        .map_err(|e| CarpeError::misc(&format!("Failed to open wallets file for writing: {}", e)))?;

    file.write_all(serialized_data.as_bytes())
        .map_err(|e| CarpeError::misc(&format!("Failed to write wallets file: {}", e)))
}

// Function to get the path to the wallets file
fn get_accounts_file_path() -> Result<PathBuf, CarpeError> {
    let app_dir_path = get_config_path(); // Assuming this returns a PathBuf or Path
    if !app_dir_path.exists() {
        return Err(CarpeError::misc("App directory not found"));
    }
    Ok(app_dir_path.join("stream_accounts.json"))
}
