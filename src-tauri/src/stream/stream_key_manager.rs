extern crate keyring;
use anyhow::anyhow;

const KEYRING_APP_NAME: &str = "trade_stream";

fn keyring_identifier_string_format(chain_type: &str, address: &str) -> String {
  format!("{}_{}", chain_type, address.to_uppercase())
}

/// Store the encoded private key in the OS keyring
pub fn store_private_key(
  chain_type: &str,
  address: &str,
  key: &str,
) -> Result<(), keyring::Error> {
  let identifier: String = keyring_identifier_string_format(chain_type, address);
  let kr = keyring::Entry::new(KEYRING_APP_NAME, &identifier);
  kr.set_password(key)
}

/// Retrieve a private key from the OS keyring
pub fn retrieve_private_key(chain_type: &str, address: &str) -> Result<String, anyhow::Error> {
  let identifier: String = keyring_identifier_string_format(chain_type, address);
  let kr = keyring::Entry::new(KEYRING_APP_NAME, &identifier);
  match kr.get_password() {
    Ok(s) => {
      Ok(s)
    }
    Err(e) => {
      println!("{:?}", e);
      Err(anyhow!("{:?}", e))
    }
  }
}

/// Delete a private key from the OS keyring
pub fn erase_private_key(chain_type: &str, address: &str) -> anyhow::Result<()> {
  let identifier: String = keyring_identifier_string_format(chain_type, address);
  let kr = keyring::Entry::new(KEYRING_APP_NAME, &identifier);

  kr.delete_password()?;
  Ok(())
}
