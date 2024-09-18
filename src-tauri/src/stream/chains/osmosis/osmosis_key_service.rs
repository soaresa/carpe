// src/stream/account_manager.rs
use std::str::FromStr;
use bip32::{XPrv, DerivationPath};
use bip39::{Language, Mnemonic, Seed};
use bech32::{ToBase32, encode, Variant};
use sha2::{Sha256, Digest};
use anyhow::Result;
use ripemd160::Ripemd160;
use cosmrs::crypto::secp256k1::SigningKey;
use crate::carpe_error::CarpeError;

// Derive address, private key, and public key from mnemonic
pub fn derive_account(mnemonic: &str) -> Result<(String, String, String), CarpeError> {
  // Step 1: Convert mnemonic to seed
  let mnemonic = Mnemonic::from_phrase(mnemonic, Language::English)
      .map_err(|e| CarpeError::misc(&format!("Failed to create mnemonic from phrase: {}", e)))?;
  let seed = Seed::new(&mnemonic, "");

  // Derive the extended private key using BIP-32
  let derivation_path = DerivationPath::from_str("m/44'/118'/0'/0/0")
      .map_err(|e| CarpeError::misc(&format!("Failed to create DerivationPath: {}", e)))?;
  let child_xprv = XPrv::derive_from_path(&seed, &derivation_path)
      .map_err(|e| CarpeError::misc(&format!("Failed to derive child xprv: {}", e)))?;

  // Get the public key
  let public_key = child_xprv.public_key();
  let public_key_str = hex::encode(public_key.to_bytes());

  // Get the private key
  let private_key_str = hex::encode(child_xprv.private_key().to_bytes());

  // Hash the public key with SHA-256 followed by RIPEMD-160 to get the address
  let sha256_hash = Sha256::digest(&public_key.to_bytes());
  let ripemd160_hash = Ripemd160::digest(&sha256_hash);

  // Encode the result in Bech32 with the "osmo" prefix
  let address = encode("osmo", ripemd160_hash.to_base32(), Variant::Bech32)
      .map(|s| s.to_string())
      .map_err(|e| CarpeError::misc(&format!("Failed to encode address: {}", e)))?;

  Ok((address, public_key_str, private_key_str))
}


pub fn derive_signing_key(private_key_str: &str) -> Result<SigningKey, CarpeError> {
  // Convert the private key string to bytes
  let private_key_bytes = hex::decode(private_key_str)
    .map_err(|e| CarpeError::misc(&format!("Failed to decode private key string: {}", e)))?;

  // Convert the private key bytes to a SigningKey
  let signing_key = SigningKey::from_bytes(&private_key_bytes)
    .map_err(|e| CarpeError::misc(&format!("Failed to convert to SigningKey: {}", e)))?;
  
  Ok(signing_key)
}
