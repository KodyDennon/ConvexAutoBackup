use crate::db::AppDatabase;
use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit},
};
use anyhow::{Context, anyhow};
use base64::{Engine, engine::general_purpose::STANDARD};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StoredSecret {
    pub id: Uuid,
    pub label: String,
    pub kind: SecretKind,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    ConvexDeployKey,
    S3Credentials,
    WebhookToken,
    EncryptionKey,
}

#[derive(Debug, Clone)]
pub struct SecretVault {
    database: AppDatabase,
    key: [u8; 32],
}

impl SecretVault {
    pub fn from_env(database: AppDatabase) -> anyhow::Result<Self> {
        let key = std::env::var("CONVEX_AUTOBACKUP_MASTER_KEY")
            .context("CONVEX_AUTOBACKUP_MASTER_KEY is required for encrypted secrets")?;
        Ok(Self::with_master_key(database, &key))
    }

    pub fn with_master_key(database: AppDatabase, master_key: &str) -> Self {
        let digest = Sha256::digest(master_key.as_bytes());
        let mut key = [0_u8; 32];
        key.copy_from_slice(&digest);
        Self { database, key }
    }

    pub fn put_secret(
        &self,
        label: &str,
        kind: SecretKind,
        plaintext: &str,
    ) -> anyhow::Result<StoredSecret> {
        if label.trim().is_empty() {
            return Err(anyhow!("secret label is required"));
        }
        if plaintext.is_empty() {
            return Err(anyhow!("secret value is required"));
        }
        let mut nonce_bytes = [0_u8; 12];
        getrandom::getrandom(&mut nonce_bytes)
            .map_err(|error| anyhow!("failed to generate secret nonce: {error}"))?;
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|error| anyhow!("failed to initialize secret cipher: {error}"))?;
        let ciphertext = cipher
            .encrypt(Nonce::from_slice(&nonce_bytes), plaintext.as_bytes())
            .map_err(|error| anyhow!("failed to encrypt secret: {error}"))?;
        let now = Utc::now();
        let stored = StoredSecret {
            id: Uuid::now_v7(),
            label: label.to_string(),
            kind,
            created_at: now,
            updated_at: now,
        };
        let connection = self.database.connection()?;
        connection.execute(
            "INSERT INTO secrets (id, label, kind, nonce_b64, ciphertext_b64, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                stored.id.to_string(),
                stored.label,
                kind_to_str(&stored.kind),
                STANDARD.encode(nonce_bytes),
                STANDARD.encode(ciphertext),
                stored.created_at.to_rfc3339(),
                stored.updated_at.to_rfc3339()
            ],
        )?;
        Ok(stored)
    }

    pub fn get_secret(&self, id: Uuid) -> anyhow::Result<String> {
        let connection = self.database.connection()?;
        let row = connection
            .query_row(
                "SELECT nonce_b64, ciphertext_b64 FROM secrets WHERE id = ?1",
                params![id.to_string()],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("secret {id} does not exist"))?;
        let nonce = STANDARD
            .decode(row.0)
            .context("stored secret nonce is not base64")?;
        let ciphertext = STANDARD
            .decode(row.1)
            .context("stored secret ciphertext is not base64")?;
        let cipher = Aes256Gcm::new_from_slice(&self.key)
            .map_err(|error| anyhow!("failed to initialize secret cipher: {error}"))?;
        let plaintext = cipher
            .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
            .map_err(|error| anyhow!("failed to decrypt secret: {error}"))?;
        String::from_utf8(plaintext).context("stored secret is not valid UTF-8")
    }

    pub fn list_secrets(&self) -> anyhow::Result<Vec<StoredSecret>> {
        let connection = self.database.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, label, kind, created_at, updated_at FROM secrets ORDER BY created_at ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(StoredSecret {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        error.into(),
                    )
                })?,
                label: row.get(1)?,
                kind: kind_from_str(&row.get::<_, String>(2)?).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        2,
                        rusqlite::types::Type::Text,
                        error.into(),
                    )
                })?,
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(3)?)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            3,
                            rusqlite::types::Type::Text,
                            error.into(),
                        )
                    })?
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            4,
                            rusqlite::types::Type::Text,
                            error.into(),
                        )
                    })?
                    .with_timezone(&Utc),
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}

fn kind_to_str(kind: &SecretKind) -> &'static str {
    match kind {
        SecretKind::ConvexDeployKey => "convex_deploy_key",
        SecretKind::S3Credentials => "s3_credentials",
        SecretKind::WebhookToken => "webhook_token",
        SecretKind::EncryptionKey => "encryption_key",
    }
}

fn kind_from_str(value: &str) -> anyhow::Result<SecretKind> {
    match value {
        "convex_deploy_key" => Ok(SecretKind::ConvexDeployKey),
        "s3_credentials" => Ok(SecretKind::S3Credentials),
        "webhook_token" => Ok(SecretKind::WebhookToken),
        "encryption_key" => Ok(SecretKind::EncryptionKey),
        other => Err(anyhow!("unknown secret kind {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_decrypts_secret_without_exposing_plaintext_in_database() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let vault = SecretVault::with_master_key(db.clone(), "test-master-key");

        let stored = vault
            .put_secret(
                "prod deploy key",
                SecretKind::ConvexDeployKey,
                "super-secret",
            )
            .unwrap();

        assert_eq!(vault.get_secret(stored.id).unwrap(), "super-secret");
        let raw_db = std::fs::read_to_string(db.path()).unwrap_or_default();
        assert!(!raw_db.contains("super-secret"));
        assert_eq!(vault.list_secrets().unwrap().len(), 1);
    }
}
