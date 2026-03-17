// Crypto Service
// Secure token storage using AES-256-GCM encryption with machine-derived key
// This approach doesn't rely on OS Keychain, making it stable across dev builds

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Digest;

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;
// Application-specific salt for key derivation
const APP_SALT: &[u8] = b"SpecForge-Token-Encryption-v1";

/// Encrypted data structure stored in JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedData {
    /// Base64 encoded nonce
    pub nonce: String,
    /// Base64 encoded ciphertext
    pub ciphertext: String,
}

/// Result of encryption operations
#[derive(Debug)]
pub enum CryptoError {
    KeychainError(String),
    EncryptionError(String),
    DecryptionError(String),
    InvalidData(String),
}

impl std::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CryptoError::KeychainError(msg) => write!(f, "Key derivation error: {}", msg),
            CryptoError::EncryptionError(msg) => write!(f, "Encryption error: {}", msg),
            CryptoError::DecryptionError(msg) => write!(f, "Decryption error: {}", msg),
            CryptoError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
        }
    }
}

impl std::error::Error for CryptoError {}

/// Get the machine-derived encryption key
/// This key is deterministically derived from machine-specific identifiers,
/// so it will always be the same on the same machine without needing external storage.
pub fn get_or_create_master_key() -> Result<[u8; KEY_SIZE], CryptoError> {
    let machine_id = get_machine_id()?;
    let key = derive_key_from_machine_id(&machine_id);
    Ok(key)
}

/// Get a unique machine identifier
/// On macOS, this uses the IOPlatformUUID from IOKit
fn get_machine_id() -> Result<String, CryptoError> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        // Get hardware UUID from ioreg (IOPlatformUUID)
        let output = Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output()
            .map_err(|e| CryptoError::KeychainError(format!("Failed to get machine ID: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse IOPlatformUUID from output
        for line in stdout.lines() {
            if line.contains("IOPlatformUUID") {
                if let Some(uuid) = line.split('"').nth(3) {
                    return Ok(uuid.to_string());
                }
            }
        }

        // Fallback: use hostname + username as identifier
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown-host".to_string());
        let username = std::env::var("USER").unwrap_or_else(|_| "unknown-user".to_string());

        Ok(format!("{}-{}", hostname, username))
    }

    #[cfg(not(target_os = "macos"))]
    {
        // For other platforms, use hostname + username
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().to_string())
            .unwrap_or_else(|_| "unknown-host".to_string());
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "unknown-user".to_string());

        Ok(format!("{}-{}", hostname, username))
    }
}

/// Derive encryption key from machine identifier using SHA-256
fn derive_key_from_machine_id(machine_id: &str) -> [u8; KEY_SIZE] {
    let mut hasher = sha2::Sha256::new();
    hasher.update(APP_SALT);
    hasher.update(machine_id.as_bytes());
    hasher.update(APP_SALT); // Double salt for extra mixing

    let result = hasher.finalize();

    let mut key = [0u8; KEY_SIZE];
    key.copy_from_slice(&result);
    key
}

/// Encrypt a plaintext string using AES-256-GCM
pub fn encrypt(plaintext: &str) -> Result<EncryptedData, CryptoError> {
    let key = get_or_create_master_key()?;

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::EncryptionError(format!("Failed to create cipher: {}", e)))?;

    // Generate random nonce
    let mut nonce_bytes = [0u8; NONCE_SIZE];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    // Encrypt
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| CryptoError::EncryptionError(format!("Encryption failed: {}", e)))?;

    Ok(EncryptedData {
        nonce: BASE64.encode(nonce_bytes),
        ciphertext: BASE64.encode(ciphertext),
    })
}

/// Decrypt an encrypted data structure
pub fn decrypt(encrypted: &EncryptedData) -> Result<String, CryptoError> {
    let key = get_or_create_master_key()?;

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| CryptoError::DecryptionError(format!("Failed to create cipher: {}", e)))?;

    // Decode nonce
    let nonce_bytes = BASE64
        .decode(&encrypted.nonce)
        .map_err(|e| CryptoError::InvalidData(format!("Invalid nonce: {}", e)))?;

    if nonce_bytes.len() != NONCE_SIZE {
        return Err(CryptoError::InvalidData(format!(
            "Invalid nonce size: expected {}, got {}",
            NONCE_SIZE,
            nonce_bytes.len()
        )));
    }

    let nonce = Nonce::from_slice(&nonce_bytes);

    // Decode ciphertext
    let ciphertext = BASE64
        .decode(&encrypted.ciphertext)
        .map_err(|e| CryptoError::InvalidData(format!("Invalid ciphertext: {}", e)))?;

    // Decrypt
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|e| CryptoError::DecryptionError(format!("Decryption failed: {}", e)))?;

    String::from_utf8(plaintext)
        .map_err(|e| CryptoError::DecryptionError(format!("Invalid UTF-8: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let plaintext = "my-secret-api-token-12345";

        let encrypted = encrypt(plaintext).expect("Encryption should succeed");
        let decrypted = decrypt(&encrypted).expect("Decryption should succeed");

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_machine_id_consistency() {
        // Machine ID should be consistent across calls
        let id1 = get_machine_id().expect("Should get machine ID");
        let id2 = get_machine_id().expect("Should get machine ID");
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_key_derivation_consistency() {
        // Key should be consistent for same machine ID
        let key1 = get_or_create_master_key().expect("Should get key");
        let key2 = get_or_create_master_key().expect("Should get key");
        assert_eq!(key1, key2);
    }
}
