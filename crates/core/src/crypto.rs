//! Backup archive encryption.
//!
//! Archives are encrypted with [age](https://age-encryption.org) using a
//! passphrase (scrypt recipient). The output is a standard `.age` file, so a
//! backup can be decrypted without this application: `age -d backup.zip.age`.

use crate::models::EncryptionMode;
use crate::secrets::SecretVault;
use crate::{AppDatabase, Result, ResultContext, error};
use age::secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::iter;
use uuid::Uuid;

/// Minimum passphrase length accepted when enabling encryption.
pub const MIN_PASSPHRASE_LEN: usize = 12;

/// File extension appended to encrypted archives.
pub const ENCRYPTED_EXTENSION: &str = "age";

const AGE_MAGIC: &[u8] = b"age-encryption.org/v1";

/// Encryption metadata recorded in a backup manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveEncryption {
    /// Always `age_scrypt` for passphrase encryption.
    pub mode: String,
    /// Secret vault id of the passphrase used for this archive.
    pub key_ref: Uuid,
}

impl ArchiveEncryption {
    pub fn age_scrypt(key_ref: Uuid) -> Self {
        Self {
            mode: "age_scrypt".to_string(),
            key_ref,
        }
    }
}

pub fn validate_passphrase(passphrase: &str) -> Result<()> {
    if passphrase.chars().count() < MIN_PASSPHRASE_LEN {
        return Err(error!(
            "encryption passphrase must be at least {MIN_PASSPHRASE_LEN} characters"
        ));
    }
    Ok(())
}

pub fn encrypt_with_passphrase(plaintext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let encryptor =
        age::Encryptor::with_user_passphrase(SecretString::from(passphrase.to_string()));
    let mut ciphertext = Vec::with_capacity(plaintext.len() + 1024);
    let mut writer = encryptor
        .wrap_output(&mut ciphertext)
        .context("failed to start archive encryption")?;
    writer
        .write_all(plaintext)
        .context("failed to encrypt archive")?;
    writer
        .finish()
        .context("failed to finish archive encryption")?;
    Ok(ciphertext)
}

pub fn decrypt_with_passphrase(ciphertext: &[u8], passphrase: &str) -> Result<Vec<u8>> {
    let decryptor = age::Decryptor::new_buffered(ciphertext)
        .map_err(|err| error!("archive is not a valid age file: {err}"))?;
    if !decryptor.is_scrypt() {
        return Err(error!("archive is not passphrase-encrypted"));
    }
    let identity = age::scrypt::Identity::new(SecretString::from(passphrase.to_string()));
    let mut reader = decryptor
        .decrypt(iter::once(&identity as &dyn age::Identity))
        .map_err(|err| error!("failed to decrypt archive (wrong passphrase?): {err}"))?;
    let mut plaintext = Vec::with_capacity(ciphertext.len());
    reader
        .read_to_end(&mut plaintext)
        .context("failed to read decrypted archive")?;
    Ok(plaintext)
}

pub fn is_age_ciphertext(bytes: &[u8]) -> bool {
    bytes.starts_with(AGE_MAGIC)
}

/// Looks up the vault passphrase referenced by `key_ref`.
pub fn passphrase_for_key(database: &AppDatabase, key_ref: Uuid) -> Result<String> {
    SecretVault::from_env(database.clone())?
        .get_secret(key_ref)
        .with_context(|| format!("encryption passphrase secret {key_ref} is unavailable"))
}

/// Encrypts `plaintext` according to a destination's encryption mode.
///
/// Returns the bytes to store and, when encrypted, the manifest metadata.
pub fn encrypt_for_destination(
    database: &AppDatabase,
    mode: &EncryptionMode,
    plaintext: &[u8],
) -> Result<(Vec<u8>, Option<ArchiveEncryption>)> {
    match mode {
        EncryptionMode::Disabled => Ok((plaintext.to_vec(), None)),
        EncryptionMode::Passphrase { key_ref } => {
            let passphrase = passphrase_for_key(database, key_ref.id)?;
            let ciphertext = encrypt_with_passphrase(plaintext, &passphrase)?;
            Ok((ciphertext, Some(ArchiveEncryption::age_scrypt(key_ref.id))))
        }
        EncryptionMode::AgeX25519 { .. } | EncryptionMode::ManagedKey { .. } => Err(error!(
            "this encryption mode is not supported yet; use passphrase encryption"
        )),
    }
}

/// Reverses [`encrypt_for_destination`] using the manifest's encryption metadata.
pub fn decrypt_archive(
    database: &AppDatabase,
    encryption: Option<&ArchiveEncryption>,
    stored: Vec<u8>,
) -> Result<Vec<u8>> {
    match encryption {
        None => Ok(stored),
        Some(meta) if meta.mode == "age_scrypt" => {
            let passphrase = passphrase_for_key(database, meta.key_ref)?;
            decrypt_with_passphrase(&stored, &passphrase)
        }
        Some(meta) => Err(error!("unsupported archive encryption mode {}", meta.mode)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passphrase_round_trip() {
        let ciphertext = encrypt_with_passphrase(b"zip bytes", "correct horse battery").unwrap();
        assert!(is_age_ciphertext(&ciphertext));
        assert_ne!(ciphertext, b"zip bytes");
        let plaintext = decrypt_with_passphrase(&ciphertext, "correct horse battery").unwrap();
        assert_eq!(plaintext, b"zip bytes");
    }

    #[test]
    fn wrong_passphrase_fails() {
        let ciphertext = encrypt_with_passphrase(b"zip bytes", "correct horse battery").unwrap();
        let error = decrypt_with_passphrase(&ciphertext, "wrong horse battery").unwrap_err();
        assert!(error.to_string().contains("wrong passphrase"));
    }

    #[test]
    fn short_passphrase_rejected() {
        assert!(validate_passphrase("short").is_err());
        assert!(validate_passphrase("long enough passphrase").is_ok());
    }
}
