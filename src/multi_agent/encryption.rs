//! Encryption utilities for secure agent-to-agent communication.
//!
//! This module provides AES-GCM encryption for private
//! agent communication on Solana.

use crate::{Result, ToolkitError};
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce as AesNonce,
};
use rand::RngCore;

/// Encryption key type (256-bit).
pub type EncryptionKey = [u8; 32];

/// Nonce for encryption (96 bits / 12 bytes).
pub type MessageNonce = [u8; 12];

/// Message encryptor using AES-256-GCM.
#[derive(Debug, Clone)]
pub struct MessageEncryptor {
    cipher: Aes256Gcm,
    key: EncryptionKey,
}

impl MessageEncryptor {
    /// Create a new encryptor with a random key.
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        Self {
            cipher: Aes256Gcm::new(&key.into()),
            key,
        }
    }

    /// Create an encryptor with a specific key.
    pub fn with_key(key: EncryptionKey) -> Self {
        Self {
            cipher: Aes256Gcm::new(&key.into()),
            key,
        }
    }

    /// Get encryption key.
    pub fn key(&self) -> EncryptionKey {
        self.key
    }

    /// Encrypt a message.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The message to encrypt
    /// * `nonce` - The nonce to use for encryption (12 bytes)
    ///
    /// # Returns
    ///
    /// The encrypted ciphertext with authentication tag appended.
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 12 {
            return Err(ToolkitError::InvalidInput("Nonce must be exactly 12 bytes".to_string()));
        }

        let nonce_array: [u8; 12] = nonce.try_into()
            .map_err(|_| ToolkitError::InvalidInput("Invalid nonce length".to_string()))?;

        let nonce = AesNonce::from_slice(&nonce_array);

        self.cipher
            .encrypt(&nonce, plaintext)
            .map_err(|e| ToolkitError::Custom(format!("Encryption error: {}", e)))
    }

    /// Decrypt a message.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted message
    /// * `nonce` - The nonce used for encryption (12 bytes)
    ///
    /// # Returns
    ///
    /// The decrypted plaintext.
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8]) -> Result<Vec<u8>> {
        if nonce.len() != 12 {
            return Err(ToolkitError::InvalidInput("Nonce must be exactly 12 bytes".to_string()));
        }

        let nonce_array: [u8; 12] = nonce.try_into()
            .map_err(|_| ToolkitError::InvalidInput("Invalid nonce length".to_string()))?;

        let nonce = AesNonce::from_slice(&nonce_array);

        self.cipher
            .decrypt(&nonce, ciphertext)
            .map_err(|e| ToolkitError::Custom(format!("Decryption error: {}", e)))
    }

    /// Generate a random nonce.
    pub fn generate_nonce(&self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }

    /// Encrypt with auto-generated nonce.
    ///
    /// Returns (ciphertext, nonce).
    pub fn encrypt_auto(&self, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12])> {
        let nonce = self.generate_nonce();
        let ciphertext = self.encrypt(plaintext, &nonce)?;
        Ok((ciphertext, nonce))
    }

    /// Derive shared key from two agent's keys using XOR.
    ///
    /// In a real implementation, this would use ECDH (X25519).
    /// For simplicity, this uses XOR of two keys.
    pub fn derive_shared_key(agent1_key: &EncryptionKey, agent2_key: &EncryptionKey) -> EncryptionKey {
        let mut shared = [0u8; 32];
        for (i, (a, b)) in agent1_key.iter().zip(agent2_key.iter()).enumerate() {
            shared[i] = a ^ b;
        }
        shared
    }
}

impl MessageEncryptor {
    /// Create a new encryptor with a random key.
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);

        Self {
            cipher: ChaCha20Poly1305::new(&key.into()),
            key,
        }
    }

    /// Create an encryptor with a specific key.
    pub fn with_key(key: EncryptionKey) -> Self {
        Self {
            cipher: ChaCha20Poly1305::new(&key.into()),
            key,
        }
    }

    /// Get the encryption key.
    pub fn key(&self) -> EncryptionKey {
        self.key
    }

    /// Encrypt a message.
    ///
    /// # Arguments
    ///
    /// * `plaintext` - The message to encrypt
    /// * `nonce` - The nonce to use for encryption (12 bytes)
    ///
    /// # Returns
    ///
    /// The encrypted ciphertext with authentication tag appended.
    pub fn encrypt(&self, plaintext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce);

        self.cipher
            .encrypt(nonce, plaintext)
            .map_err(|e| ToolkitError::Custom(format!("Encryption error: {}", e)))
    }

    /// Decrypt a message.
    ///
    /// # Arguments
    ///
    /// * `ciphertext` - The encrypted message
    /// * `nonce` - The nonce used for encryption (12 bytes)
    ///
    /// # Returns
    ///
    /// The decrypted plaintext.
    pub fn decrypt(&self, ciphertext: &[u8], nonce: &[u8; 12]) -> Result<Vec<u8>> {
        let nonce = Nonce::from_slice(nonce);

        self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| ToolkitError::Custom(format!("Decryption error: {}", e)))
    }

    /// Generate a random nonce.
    pub fn generate_nonce(&self) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        OsRng.fill_bytes(&mut nonce);
        nonce
    }

    /// Encrypt with auto-generated nonce.
    ///
    /// Returns (ciphertext, nonce).
    pub fn encrypt_auto(&self, plaintext: &[u8]) -> Result<(Vec<u8>, [u8; 12])> {
        let nonce = self.generate_nonce();
        let ciphertext = self.encrypt(plaintext, &nonce)?;
        Ok((ciphertext, nonce))
    }

    /// Derive shared key from two agent's keys using ECDH.
    ///
    /// In a real implementation, this would use X25519 key exchange.
    /// For now, it uses XOR of the two keys (simplified).
    pub fn derive_shared_key(agent1_key: &EncryptionKey, agent2_key: &EncryptionKey) -> EncryptionKey {
        let mut shared = [0u8; 32];
        for (i, (a, b)) in agent1_key.iter().zip(agent2_key.iter()).enumerate() {
            shared[i] = a ^ b;
        }
        shared
    }
}

impl Default for MessageEncryptor {
    fn default() -> Self {
        Self::new()
    }
}

/// Key pair for asymmetric encryption (simplified).
#[derive(Debug, Clone)]
pub struct AgentKeyPair {
    public_key: EncryptionKey,
    private_key: EncryptionKey,
}

impl AgentKeyPair {
    /// Generate a new key pair.
    pub fn new() -> Self {
        let mut public = [0u8; 32];
        let mut private = [0u8; 32];
        OsRng.fill_bytes(&mut public);
        OsRng.fill_bytes(&mut private);

        Self {
            public_key: public,
            private_key,
        }
    }

    /// Get the public key.
    pub fn public_key(&self) -> EncryptionKey {
        self.public_key
    }

    /// Get the private key.
    pub fn private_key(&self) -> EncryptionKey {
        self.private_key
    }

    /// Derive shared key with another agent's public key.
    pub fn derive_shared_key(&self, their_public: &EncryptionKey) -> EncryptionKey {
        MessageEncryptor::derive_shared_key(&self.private_key, their_public)
    }
}

impl Default for AgentKeyPair {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared secret manager for multiple agents.
#[derive(Debug)]
pub struct SharedSecretManager {
    secrets: HashMap<String, EncryptionKey>,
}

impl SharedSecretManager {
    /// Create a new manager.
    pub fn new() -> Self {
        Self {
            secrets: HashMap::new(),
        }
    }

    /// Store a shared secret for two agents.
    pub fn store_secret(&mut self, agent1: &str, agent2: &str, secret: EncryptionKey) {
        let key = self.make_key(agent1, agent2);
        self.secrets.insert(key, secret);
    }

    /// Get a shared secret.
    pub fn get_secret(&self, agent1: &str, agent2: &str) -> Option<EncryptionKey> {
        self.secrets.get(&self.make_key(agent1, agent2)).copied()
    }

    /// Remove a shared secret.
    pub fn remove_secret(&mut self, agent1: &str, agent2: &str) -> bool {
        self.secrets.remove(&self.make_key(agent1, agent2)).is_some()
    }

    fn make_key(&self, agent1: &str, agent2: &str) -> String {
        let mut keys = vec![agent1, agent2];
        keys.sort();
        format!("{}:{}", keys[0], keys[1])
    }
}

impl Default for SharedSecretManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Message authentication code for integrity verification.
#[derive(Debug, Clone)]
pub struct MessageAuth {
    tag: Vec<u8>,
}

impl MessageAuth {
    /// Generate authentication tag for a message using simple XOR hashing.
    pub fn generate(message: &[u8], key: &EncryptionKey) -> Self {
        let mut tag = vec![0u8; 16];

        for (i, &byte) in message.iter().enumerate() {
            tag[i % 16] ^= byte;
        }

        for (i, &k) in key.iter().take(16).enumerate() {
            tag[i] ^= k;
        }

        Self { tag }
    }

    /// Verify authentication tag.
    pub fn verify(&self, message: &[u8], key: &EncryptionKey) -> bool {
        let expected = Self::generate(message, key);
        self.tag == expected.tag
    }

    /// Get tag bytes.
    pub fn tag(&self) -> &[u8] {
        &self.tag
    }
}

impl MessageAuth {
    /// Generate authentication tag for a message.
    pub fn generate(message: &[u8], key: &EncryptionKey) -> Self {
        use blake3::Hash;
        let hash = Hash::hash(message);
        let mut tag = hash.as_bytes().to_vec();

        for (i, k) in key.iter().take(16).enumerate() {
            tag[i] ^= k;
        }

        Self { tag }
    }

    /// Verify authentication tag.
    pub fn verify(&self, message: &[u8], key: &EncryptionKey) -> bool {
        let expected = Self::generate(message, key);
        self.tag == expected.tag
    }

    /// Get the tag bytes.
    pub fn tag(&self) -> &[u8] {
        &self.tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryptor_creation() {
        let encryptor = MessageEncryptor::new();
        assert!(!encryptor.key().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_encrypt_decrypt() {
        let encryptor = MessageEncryptor::new();
        let plaintext = b"Secret message for agent coordination";
        let nonce = encryptor.generate_nonce();

        let ciphertext = encryptor.encrypt(plaintext, &nonce).unwrap();
        let decrypted = encryptor.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_encrypt_auto() {
        let encryptor = MessageEncryptor::new();
        let plaintext = b"Auto-generated nonce message";

        let (ciphertext, nonce) = encryptor.encrypt_auto(plaintext).unwrap();
        let decrypted = encryptor.decrypt(&ciphertext, &nonce).unwrap();

        assert_eq!(plaintext, decrypted.as_slice());
    }

    #[test]
    fn test_derive_shared_key() {
        let key1: EncryptionKey = [1u8; 32];
        let key2: EncryptionKey = [2u8; 32];

        let shared1 = MessageEncryptor::derive_shared_key(&key1, &key2);
        let shared2 = MessageEncryptor::derive_shared_key(&key2, &key1);

        assert_eq!(shared1, shared2);
    }

    #[test]
    fn test_agent_key_pair() {
        let keypair = AgentKeyPair::new();
        assert_ne!(keypair.public_key(), keypair.private_key());
        assert!(!keypair.public_key().iter().all(|&b| b == 0));
        assert!(!keypair.private_key().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_shared_secret_manager() {
        let mut manager = SharedSecretManager::new();
        let secret = [42u8; 32];

        manager.store_secret("agent1", "agent2", secret);

        assert_eq!(manager.get_secret("agent1", "agent2"), Some(secret));
        assert_eq!(manager.get_secret("agent2", "agent1"), Some(secret));
    }

    #[test]
    fn test_message_auth() {
        let message = b"Authenticated message";
        let key = [1u8; 32];

        let auth = MessageAuth::generate(message, &key);
        assert!(auth.verify(message, &key));
        assert!(!auth.verify(b"Different message", &key));
    }
}
