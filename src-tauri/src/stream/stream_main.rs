use std::path::PathBuf;
use std::fs::File;
use std::io::{self, Read};
use super::stream_configs;
use super::stream_account_manager;
use super::stream_key_manager;
use super::chains::osmosis::{osmosis_pool_service, osmosis_key_service, osmosis_transaction};
use super::chains::chain::ChainType;
use super::stream_configs::get_env;
use super::chains::coin::Coin;

use rand::Rng;
use chrono::{DateTime, Utc, Duration as ChronoDuration};
use tokio::time::{interval, Duration};

pub async fn initialize() {
  println!("### Initializing Stream module");

  stream_configs::initialize();

  // execute_trade_stream().await;

  // Tests
  println!(">>> Osmosis Chain ID: {}", ChainType::Osmosis.chain_id());

  let env = get_env();

  let (pool_id, token_in, token_out, min_price, amount_out) = match env.as_str() {
    "prod" => (1721, Coin::WLibra, Coin::USDC, 0.00060, 1), // 1942, BTC
    _ => (15, Coin::TOSMO, Coin::TUSDC, 0.9, 2_000) // Test
  };
  
  let price = match osmosis_pool_service::fetch_coin_price(pool_id).await {
    Ok(data) => data,
    Err(e) => {
        println!("Error fetching Osmosis pool data: {}", e);
        0.0 // Provide a default value or handle accordingly
    }
  };
  println!("$$$ Token {} price is: {}", token_in, price);

  // Restore account if it does not exists
  let accounts = stream_account_manager::get_accounts().await;
  match accounts {
    Ok(accounts) => {
      if accounts.is_empty() {
        let mnemonic_phrase = read_mnemonic_from_file().unwrap();
        let _ = stream_account_manager::restore_account(&mnemonic_phrase, ChainType::Osmosis, Some("Primeira Conta Stream".to_string())).await;
      }
    },
    Err(_) => {
      eprintln!("Error fetching accounts");
    }
  }

  // print accounts balances
  let accounts = stream_account_manager::get_accounts().await;
  for account in accounts.unwrap() {
    println!("### Account restored: {:?}", account.address);
    // print balances
    for coin_balance in account.balances {
      println!("$$$ Coin: {:?}, Balance: {:?}", coin_balance.coin, coin_balance.amount / 1_000_000);
    }
  }

  // print get_current_block_height
  let block_height = osmosis_pool_service::get_current_block_height().await;
  match block_height {
    Ok(height) => {
      println!("### Current block height: {:?}", height);
    },
    Err(e) => {
      eprintln!("Error fetching block height: {:?}", e);
    }
  }

  // test swap

  let accounts = stream_account_manager::get_accounts().await;
  match accounts {
    Ok(accounts) => {
      for account in accounts {
        if account.chain == ChainType::Osmosis {
          // get private key
          let private_key = match stream_key_manager::retrieve_private_key(&account.chain.to_string(), &account.address) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("Error retrieving private key: {:?}", e);
                return;  // or handle the error appropriately
            }
          };
        
          // derive signing key
          let signing_key = match osmosis_key_service::derive_signing_key(&private_key) {
            Ok(key) => key,
            Err(e) => {
                eprintln!("Error retrieving signing key: {:?}", e);
                return;  // or handle the error appropriately
            }
          };
        
          // perform swap
          let token_in_max_amount = 45_100_000;
          println!("### Performing swap: token_in_max_amount: {}", token_in_max_amount);
          let ret = osmosis_pool_service::perform_swap(
            &account.address,
            &signing_key,
            pool_id,
            token_in,
            token_out,
            amount_out,
            min_price,
          ).await;
          
          // print response
          match ret {
            Ok(response) => {
              println!("Swap successful! Response: {}", response);
            },
            Err(e) => {
              eprintln!("Error performing swap: {:?}", e);
            }
          }
        }
      }
    },
    Err(e) => {
      eprintln!("Error fetching accounts: {:?}", e);
    }
  }

}

async fn execute_trade_stream() {
  // Constants
  const STREAMS_PER_DAY: u32 = 4;
  const USDC_PER_DAY: f64 = 100.0;
  const INTERVAL_SECONDS: u64 = 60;

  // Initializations
  let mut end_block_time: DateTime<Utc> = Utc::now();
  let mut next_trade: DateTime<Utc> = Utc::now();
  let mut block_executed = false;

  // Create an interval that ticks every minute
  let mut interval = interval(Duration::from_secs(INTERVAL_SECONDS));

  loop {
      // Wait for the next tick
      interval.tick().await;
      println!(">>> tick");

      let now = Utc::now();

      if end_block_time < now {
        println!(">>> New block started at {}", now.format("%Y-%m-%d %H:%M:%S"));
          block_executed = false;

          // Calculate the end time of the current block
          let block_duration = ChronoDuration::hours(24) / STREAMS_PER_DAY as i32;
          end_block_time = now + block_duration;

          // Generate a random time between now and the end of the block
          let now_timestamp = now.timestamp();
          let end_timestamp = end_block_time.timestamp();
          let mut rng = rand::thread_rng();
          let random_timestamp = rng.gen_range(now_timestamp..end_timestamp);
          next_trade = DateTime::<Utc>::from_utc(
              chrono::NaiveDateTime::from_timestamp(random_timestamp, 0),
              Utc,
          );
          println!(
              ">>> Next trade scheduled for {}",
              next_trade.format("%Y-%m-%d %H:%M:%S")
          );
      }

      if block_executed {
          continue;
      }

      if next_trade < now {
          block_executed = true;
          let block_amount = USDC_PER_DAY / STREAMS_PER_DAY as f64;
          println!(
              "$$$ Trade executed at {} for amount {}",
              now.format("%Y-%m-%d %H:%M:%S"),
              block_amount
          );
      }
  }
}

fn read_mnemonic_from_file() -> io::Result<String> {
  // Construct the full path to the file
  let mut path: PathBuf = stream_configs::get_config_path();
  path.push("temp_mnemonic.txt");

  // Open the file
  let mut file = File::open(path)?;

  // Read the mnemonic from the file into a String
  let mut mnemonic = String::new();
  file.read_to_string(&mut mnemonic)?;

  // Trim any extra whitespace or newlines
  Ok(mnemonic.trim().to_string())
}