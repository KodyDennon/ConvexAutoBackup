use crate::db::AppDatabase;
use anyhow::anyhow;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{DateTime, Utc};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateUser {
    pub email: String,
    pub password: String,
    pub role: Role,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub token: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
    Operator,
    Viewer,
}

impl Role {
    pub fn can_manage(self) -> bool {
        matches!(self, Self::Owner | Self::Admin)
    }

    pub fn can_run_backup(self) -> bool {
        matches!(self, Self::Owner | Self::Admin | Self::Operator)
    }
}

#[derive(Debug, Clone)]
pub struct AuthService {
    database: AppDatabase,
}

impl AuthService {
    pub fn new(database: AppDatabase) -> Self {
        Self { database }
    }

    pub fn create_user(&self, input: CreateUser) -> anyhow::Result<User> {
        if !input.email.contains('@') {
            return Err(anyhow!("email must contain @"));
        }
        if input.password.len() < 12 {
            return Err(anyhow!("password must be at least 12 characters"));
        }
        let password_hash = hash_password(&input.password)?;
        let user = User {
            id: Uuid::now_v7(),
            email: input.email.trim().to_lowercase(),
            role: input.role,
            created_at: Utc::now(),
        };
        let connection = self.database.connection()?;
        connection.execute(
            "INSERT INTO users (id, email, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                user.id.to_string(),
                user.email,
                password_hash,
                role_to_str(user.role),
                user.created_at.to_rfc3339()
            ],
        )?;
        Ok(user)
    }

    pub fn verify_password(&self, email: &str, password: &str) -> anyhow::Result<User> {
        let connection = self.database.connection()?;
        let row = connection
            .query_row(
                "SELECT id, email, password_hash, role, created_at FROM users WHERE email = ?1",
                params![email.trim().to_lowercase()],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("invalid email or password"))?;

        verify_password_hash(password, &row.2)?;
        Ok(User {
            id: Uuid::parse_str(&row.0)?,
            email: row.1,
            role: role_from_str(&row.3)?,
            created_at: DateTime::parse_from_rfc3339(&row.4)?.with_timezone(&Utc),
        })
    }

    pub fn create_api_token(&self, user_id: Uuid, name: &str) -> anyhow::Result<ApiToken> {
        if name.trim().is_empty() {
            return Err(anyhow!("token name is required"));
        }
        self.require_user(user_id)?;
        let raw = new_token();
        let token_hash = token_hash(&raw);
        let token = ApiToken {
            id: Uuid::now_v7(),
            user_id,
            name: name.to_string(),
            token: Some(raw),
            created_at: Utc::now(),
        };
        let connection = self.database.connection()?;
        connection.execute(
            "INSERT INTO api_tokens (id, user_id, name, token_hash, created_at, revoked_at) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
            params![
                token.id.to_string(),
                token.user_id.to_string(),
                token.name,
                token_hash,
                token.created_at.to_rfc3339()
            ],
        )?;
        Ok(token)
    }

    pub fn authenticate_token(&self, raw: &str) -> anyhow::Result<User> {
        let hash = token_hash(raw);
        let connection = self.database.connection()?;
        let row = connection
            .query_row(
                "SELECT users.id, users.email, users.role, users.created_at, api_tokens.token_hash
                 FROM api_tokens
                 JOIN users ON users.id = api_tokens.user_id
                 WHERE api_tokens.revoked_at IS NULL AND api_tokens.token_hash = ?1",
                params![hash],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                },
            )
            .optional()?;

        let Some(row) = row else {
            return Err(anyhow!("invalid API token"));
        };
        if hash.as_bytes().ct_eq(row.4.as_bytes()).unwrap_u8() != 1 {
            return Err(anyhow!("invalid API token"));
        }
        Ok(User {
            id: Uuid::parse_str(&row.0)?,
            email: row.1,
            role: role_from_str(&row.2)?,
            created_at: DateTime::parse_from_rfc3339(&row.3)?.with_timezone(&Utc),
        })
    }

    fn require_user(&self, user_id: Uuid) -> anyhow::Result<()> {
        let connection = self.database.connection()?;
        let count: i64 = connection.query_row(
            "SELECT COUNT(*) FROM users WHERE id = ?1",
            params![user_id.to_string()],
            |row| row.get(0),
        )?;
        if count == 0 {
            return Err(anyhow!("user {user_id} does not exist"));
        }
        Ok(())
    }
}

fn hash_password(password: &str) -> anyhow::Result<String> {
    let mut salt_bytes = [0_u8; 16];
    getrandom::getrandom(&mut salt_bytes)
        .map_err(|error| anyhow!("failed to generate password salt: {error}"))?;
    let salt = SaltString::encode_b64(&salt_bytes)
        .map_err(|error| anyhow!("invalid password salt: {error}"))?;
    Ok(Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| anyhow!("failed to hash password: {error}"))?
        .to_string())
}

fn verify_password_hash(password: &str, encoded: &str) -> anyhow::Result<()> {
    let parsed = PasswordHash::new(encoded)
        .map_err(|error| anyhow!("stored password hash is invalid: {error}"))?;
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| anyhow!("invalid email or password"))
}

fn new_token() -> String {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).expect("system random source is available");
    format!("cab_{}", URL_SAFE_NO_PAD.encode(bytes))
}

fn token_hash(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    format!("{digest:x}")
}

fn role_to_str(role: Role) -> &'static str {
    match role {
        Role::Owner => "owner",
        Role::Admin => "admin",
        Role::Operator => "operator",
        Role::Viewer => "viewer",
    }
}

fn role_from_str(value: &str) -> anyhow::Result<Role> {
    match value {
        "owner" => Ok(Role::Owner),
        "admin" => Ok(Role::Admin),
        "operator" => Ok(Role::Operator),
        "viewer" => Ok(Role::Viewer),
        other => Err(anyhow!("unknown role {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_user_and_verifies_password() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let auth = AuthService::new(db);

        let user = auth
            .create_user(CreateUser {
                email: "OWNER@example.com".to_string(),
                password: "very-secure-password".to_string(),
                role: Role::Owner,
            })
            .unwrap();

        assert_eq!(user.email, "owner@example.com");
        assert!(
            auth.verify_password("owner@example.com", "wrong-password")
                .is_err()
        );
        assert_eq!(
            auth.verify_password("owner@example.com", "very-secure-password")
                .unwrap()
                .role,
            Role::Owner
        );
    }

    #[test]
    fn creates_one_time_api_token_and_authenticates_it() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let auth = AuthService::new(db);
        let user = auth
            .create_user(CreateUser {
                email: "ops@example.com".to_string(),
                password: "very-secure-password".to_string(),
                role: Role::Operator,
            })
            .unwrap();

        let token = auth.create_api_token(user.id, "agent").unwrap();
        let raw = token.token.as_ref().unwrap();
        assert!(raw.starts_with("cab_"));
        assert_eq!(auth.authenticate_token(raw).unwrap().id, user.id);
        assert!(auth.authenticate_token("cab_wrong").is_err());
    }
}
