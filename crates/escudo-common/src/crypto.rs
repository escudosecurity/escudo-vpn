use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use rand::RngCore;
use x25519_dalek::PublicKey;

pub struct KeyPair {
    pub private_key: String,
    pub public_key: String,
}

pub fn generate_keypair() -> KeyPair {
    let mut private_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut private_bytes);

    // Clamp for Curve25519
    private_bytes[0] &= 248;
    private_bytes[31] &= 127;
    private_bytes[31] |= 64;

    let secret = x25519_dalek::StaticSecret::from(private_bytes);
    let public = PublicKey::from(&secret);

    KeyPair {
        private_key: BASE64.encode(private_bytes),
        public_key: BASE64.encode(public.as_bytes()),
    }
}

pub fn generate_preshared_key() -> String {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    BASE64.encode(key)
}

/// Encrypt plaintext with AES-256-GCM using the provided key.
/// Returns base64(nonce || ciphertext).
pub fn encrypt_private_key(plaintext: &str, master_key: &[u8; 32]) -> Result<String, String> {
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|e| format!("Encryption failed: {e}"))?;

    // Prepend nonce to ciphertext
    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    Ok(BASE64.encode(&combined))
}

/// Decrypt a value produced by encrypt_private_key.
pub fn decrypt_private_key(encrypted: &str, master_key: &[u8; 32]) -> Result<String, String> {
    let combined = BASE64
        .decode(encrypted)
        .map_err(|e| format!("Base64 decode failed: {e}"))?;

    if combined.len() < 13 {
        return Err("Ciphertext too short".to_string());
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let key = Key::<Aes256Gcm>::from_slice(master_key);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| format!("Decryption failed: {e}"))?;

    String::from_utf8(plaintext).map_err(|e| format!("UTF-8 decode failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let kp = generate_keypair();
        let priv_bytes = BASE64.decode(&kp.private_key).unwrap();
        let pub_bytes = BASE64.decode(&kp.public_key).unwrap();
        assert_eq!(priv_bytes.len(), 32);
        assert_eq!(pub_bytes.len(), 32);
        // Verify clamping
        assert_eq!(priv_bytes[0] & 7, 0);
        assert_eq!(priv_bytes[31] & 128, 0);
        assert_eq!(priv_bytes[31] & 64, 64);
    }

    #[test]
    fn test_two_keypairs_differ() {
        let kp1 = generate_keypair();
        let kp2 = generate_keypair();
        assert_ne!(kp1.private_key, kp2.private_key);
        assert_ne!(kp1.public_key, kp2.public_key);
    }

    #[test]
    fn test_preshared_key_length() {
        let psk = generate_preshared_key();
        let bytes = BASE64.decode(&psk).unwrap();
        assert_eq!(bytes.len(), 32);
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let master_key = [0x42u8; 32];
        let plaintext = "aBcDeFgHiJkLmNoPqRsTuVwXyZ012345678/+=";

        let encrypted = encrypt_private_key(plaintext, &master_key).unwrap();
        assert_ne!(encrypted, plaintext);

        let decrypted = decrypt_private_key(&encrypted, &master_key).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertexts() {
        let master_key = [0x42u8; 32];
        let plaintext = "same-key-data";
        let e1 = encrypt_private_key(plaintext, &master_key).unwrap();
        let e2 = encrypt_private_key(plaintext, &master_key).unwrap();
        // Different nonces → different ciphertexts
        assert_ne!(e1, e2);
        // But both decrypt to the same value
        assert_eq!(decrypt_private_key(&e1, &master_key).unwrap(), plaintext);
        assert_eq!(decrypt_private_key(&e2, &master_key).unwrap(), plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = [0x42u8; 32];
        let key2 = [0x43u8; 32];
        let encrypted = encrypt_private_key("secret", &key1).unwrap();
        assert!(decrypt_private_key(&encrypted, &key2).is_err());
    }

    #[test]
    fn test_decrypt_garbage_fails() {
        let key = [0x42u8; 32];
        assert!(decrypt_private_key("not-valid-base64!!!", &key).is_err());
        assert!(decrypt_private_key(&BASE64.encode([0u8; 5]), &key).is_err());
    }
}
