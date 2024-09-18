use serde::Deserialize;
use std::error::Error as StdError;
use std::env;
use reqwest;
use crate::stream::chains::coin::{Coin, CoinAmount};

#[derive(Deserialize)]
struct Balance {
    denom: String,
    amount: String,
}

#[derive(Deserialize)]
struct BalancesResponse {
    balances: Vec<Balance>,
}

pub async fn fetch_balances(address: &str, coins: Option<Vec<Coin>>) -> Result<Vec<CoinAmount>, Box<dyn StdError>> {
    let url = get_osmosis_pool_price_url();
    let url_formated = url.replace("{}", address);
    let response = reqwest::get(&url_formated).await?;
    let response = response.error_for_status()?;
    
    let balances: BalancesResponse = response.json().await?;
    let mut result = Vec::new();

    match coins {
        Some(coins_list) => {
            for coin in coins_list {
                if let Some(balance) = balances.balances.iter().find(|b| b.denom == coin.denom()) {
                    let amount = balance.amount.parse()?;
                    result.push(CoinAmount { coin, amount });
                }
            }
        },
        None => {
            for balance in balances.balances {
                let amount = balance.amount.parse()?;
                let coin = match balance.denom.as_str() {
                    "factory/osmo19hdqma2mj0vnmgcxag6ytswjnr8a3y07q7e70p/wLIBRA" => Coin::WLibra,
                    "ibc/498A0751C798A0D9A389AA3691123DADA57DAA4FE165D5C75894505B876BA6E4" => Coin::USDC,
                    "uosmo" => Coin::OSMO,
                    // TEST coins
                    "factory/osmo109ns4u04l44kqdkvp876hukd3hxz8zzm7809el/uusdc" => Coin::TUSDC,
                    // Handle other coins as needed
                    _ => continue, // Skip unknown denominations
                };
                result.push(CoinAmount { coin, amount });
            }
        }
    }

    Ok(result)
}


fn get_osmosis_pool_price_url() -> String {
    env::var("OSMOSIS_ACCOUNT_BALANCES_URL").unwrap_or_else(|_| "https://lcd-osmosis.imperator.co/cosmos/bank/v1beta1/balances/{}".to_string())
}