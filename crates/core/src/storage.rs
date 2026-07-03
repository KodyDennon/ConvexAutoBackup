use crate::{
    AppDatabase, BackupManifest, SecretVault, StorageDestination, StorageKind,
    paths::safe_backup_relative_path,
};
use crate::{Result, ResultContext, error};
use chrono::Utc;
use hmac::{Hmac, Mac};
use percent_encoding::{NON_ALPHANUMERIC, percent_encode};
use reqwest::{Client as HttpClient, Method, Url};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Clone)]
pub struct StoredBackup {
    pub archive_path: PathBuf,
    pub manifest_path: PathBuf,
    pub storage_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetentionPruneResult {
    pub deleted_archives: usize,
    pub deleted_manifests: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3CredentialSecret {
    pub access_key_id: String,
    pub secret_access_key: String,
}

pub async fn store_backup(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    archive_bytes: &[u8],
    manifest: &BackupManifest,
) -> Result<StoredBackup> {
    match &destination.kind {
        StorageKind::LocalFilesystem { .. } => store_local_backup(
            destination,
            project_name,
            deployment,
            archive_bytes,
            manifest,
        ),
        StorageKind::S3Compatible { .. } => {
            store_s3_backup(
                database,
                destination,
                project_name,
                deployment,
                archive_bytes,
                manifest,
            )
            .await
        }
    }
}

pub fn store_local_backup(
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    archive_bytes: &[u8],
    manifest: &BackupManifest,
) -> Result<StoredBackup> {
    let StorageKind::LocalFilesystem { root } = &destination.kind else {
        return Err(error!(
            "destination {} is not local filesystem",
            destination.id
        ));
    };

    let archive_name = format!(
        "{}-{}.zip",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        manifest.run_id
    );
    let manifest_name = format!("{archive_name}.manifest.json");
    let project_segment = safe_segment(project_name);
    let deployment_segment = safe_segment(deployment);
    let relative_archive =
        safe_backup_relative_path(&project_segment, &deployment_segment, &archive_name)?;
    let relative_manifest =
        safe_backup_relative_path(&project_segment, &deployment_segment, &manifest_name)?;
    let archive_path = Path::new(root).join(&relative_archive);
    let manifest_path = Path::new(root).join(&relative_manifest);
    let parent = archive_path
        .parent()
        .ok_or_else(|| error!("archive path has no parent"))?;
    std::fs::create_dir_all(parent)
        .with_context(|| format!("failed to create backup directory {}", parent.display()))?;

    let tmp_archive = archive_path.with_extension("zip.tmp");
    std::fs::write(&tmp_archive, archive_bytes)
        .with_context(|| format!("failed to write {}", tmp_archive.display()))?;
    std::fs::rename(&tmp_archive, &archive_path)
        .with_context(|| format!("failed to commit {}", archive_path.display()))?;

    let storage_uri = format!("file://{}", archive_path.display());
    let mut stored_manifest = manifest.clone();
    stored_manifest.storage_uri.clone_from(&storage_uri);
    let manifest_json = serde_json::to_vec_pretty(&stored_manifest)?;
    let tmp_manifest = manifest_path.with_extension("json.tmp");
    std::fs::write(&tmp_manifest, manifest_json)
        .with_context(|| format!("failed to write {}", tmp_manifest.display()))?;
    std::fs::rename(&tmp_manifest, &manifest_path)
        .with_context(|| format!("failed to commit {}", manifest_path.display()))?;

    Ok(StoredBackup {
        storage_uri,
        archive_path,
        manifest_path,
    })
}

pub fn prune_local_retention(
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
) -> Result<RetentionPruneResult> {
    let StorageKind::LocalFilesystem { root } = &destination.kind else {
        return Ok(RetentionPruneResult {
            deleted_archives: 0,
            deleted_manifests: 0,
        });
    };
    let Some(keep_last) = destination.retention.keep_last else {
        return Ok(RetentionPruneResult {
            deleted_archives: 0,
            deleted_manifests: 0,
        });
    };
    let backup_dir = Path::new(root)
        .join(safe_segment(project_name))
        .join(safe_segment(deployment));
    if !backup_dir.exists() {
        return Ok(RetentionPruneResult {
            deleted_archives: 0,
            deleted_manifests: 0,
        });
    }

    let mut manifests = std::fs::read_dir(&backup_dir)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".zip.manifest.json"))
        })
        .collect::<Vec<_>>();
    manifests.sort();
    let keep_last = keep_last as usize;
    if manifests.len() <= keep_last {
        return Ok(RetentionPruneResult {
            deleted_archives: 0,
            deleted_manifests: 0,
        });
    }

    let delete_count = manifests.len() - keep_last;
    let mut result = RetentionPruneResult {
        deleted_archives: 0,
        deleted_manifests: 0,
    };
    for manifest_path in manifests.into_iter().take(delete_count) {
        let archive_name = manifest_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".manifest.json"))
            .ok_or_else(|| error!("invalid manifest file name {}", manifest_path.display()))?;
        let archive_path = manifest_path.with_file_name(archive_name);
        if archive_path.exists() {
            std::fs::remove_file(&archive_path)
                .with_context(|| format!("failed to delete {}", archive_path.display()))?;
            result.deleted_archives += 1;
        }
        std::fs::remove_file(&manifest_path)
            .with_context(|| format!("failed to delete {}", manifest_path.display()))?;
        result.deleted_manifests += 1;
    }
    Ok(result)
}

pub async fn store_s3_backup(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    archive_bytes: &[u8],
    manifest: &BackupManifest,
) -> Result<StoredBackup> {
    let StorageKind::S3Compatible {
        bucket,
        region: _,
        endpoint: _,
        prefix,
        credentials: _,
    } = &destination.kind
    else {
        return Err(error!(
            "destination {} is not S3-compatible",
            destination.id
        ));
    };

    let client = s3_client_from_destination(database, destination)?;

    let archive_name = format!(
        "{}-{}.zip",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        manifest.run_id
    );
    let manifest_name = format!("{archive_name}.manifest.json");
    let base_key = object_key(prefix.as_deref(), project_name, deployment);
    let archive_key = format!("{base_key}/{archive_name}");
    let manifest_key = format!("{base_key}/{manifest_name}");
    let storage_uri = format!("s3://{bucket}/{archive_key}");
    let mut stored_manifest = manifest.clone();
    stored_manifest.storage_uri.clone_from(&storage_uri);

    client
        .put_object(&archive_key, archive_bytes.to_vec())
        .await
        .context("failed to upload S3 archive")?;
    client
        .put_object(&manifest_key, serde_json::to_vec_pretty(&stored_manifest)?)
        .await
        .context("failed to upload S3 manifest")?;

    Ok(StoredBackup {
        archive_path: PathBuf::from(&archive_key),
        manifest_path: PathBuf::from(&manifest_key),
        storage_uri,
    })
}

pub fn s3_client_from_destination(
    database: &AppDatabase,
    destination: &StorageDestination,
) -> Result<S3CompatibleClient> {
    let StorageKind::S3Compatible {
        bucket,
        region,
        endpoint,
        credentials,
        ..
    } = &destination.kind
    else {
        return Err(error!(
            "destination {} is not S3-compatible",
            destination.id
        ));
    };
    let secret_json = SecretVault::from_env(database.clone())?.get_secret(credentials.id)?;
    let secret: S3CredentialSecret =
        serde_json::from_str(&secret_json).context("S3 credential secret must be JSON")?;
    S3CompatibleClient::new(
        bucket.clone(),
        region.clone().unwrap_or_else(|| "auto".to_string()),
        endpoint.clone(),
        secret,
    )
}

pub fn s3_object_key_from_uri(uri: &str) -> Result<String> {
    let without_scheme = uri
        .strip_prefix("s3://")
        .ok_or_else(|| error!("S3 URI must start with s3://"))?;
    let (_, key) = without_scheme
        .split_once('/')
        .ok_or_else(|| error!("S3 URI must include bucket and key"))?;
    Ok(key.to_string())
}

fn object_key(prefix: Option<&str>, project_name: &str, deployment: &str) -> String {
    [
        prefix.unwrap_or("").trim_matches('/'),
        &safe_segment(project_name),
        &safe_segment(deployment),
    ]
    .into_iter()
    .filter(|segment| !segment.is_empty())
    .collect::<Vec<_>>()
    .join("/")
}

#[derive(Clone)]
pub struct S3CompatibleClient {
    http: HttpClient,
    bucket: String,
    region: String,
    endpoint: Option<String>,
    credentials: S3CredentialSecret,
}

impl S3CompatibleClient {
    fn new(
        bucket: String,
        region: String,
        endpoint: Option<String>,
        credentials: S3CredentialSecret,
    ) -> Result<Self> {
        Ok(Self {
            http: HttpClient::builder()
                .build()
                .context("failed to build S3 HTTP client")?,
            bucket,
            region,
            endpoint,
            credentials,
        })
    }

    pub async fn put_object(&self, key: &str, body: Vec<u8>) -> Result<S3Response> {
        let request = self.signed_request(Method::PUT, key, body)?;
        let response = self
            .http
            .request(request.method, request.url)
            .headers(request.headers)
            .body(request.body)
            .send()
            .await
            .context("failed to send S3 PUT request")?;
        ensure_success(response).await
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let request = self.signed_request(Method::GET, key, Vec::new())?;
        let response = self
            .http
            .request(request.method, request.url)
            .headers(request.headers)
            .send()
            .await
            .context("failed to send S3 GET request")?;
        let response = ensure_success(response).await?;
        Ok(response.body)
    }

    fn signed_request(&self, method: Method, key: &str, body: Vec<u8>) -> Result<SignedS3Request> {
        let now = Utc::now();
        let url = self.object_url(key)?;
        let host = url
            .host_str()
            .ok_or_else(|| error!("S3 endpoint URL has no host"))?
            .to_string();
        let payload_hash = hex_sha256(&body);
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
        let date = now.format("%Y%m%d").to_string();
        let scope = format!("{date}/{}/s3/aws4_request", self.region);
        let canonical_uri = if url.path().is_empty() {
            "/"
        } else {
            url.path()
        };
        let canonical_headers =
            format!("host:{host}\nx-amz-content-sha256:{payload_hash}\nx-amz-date:{amz_date}\n");
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "{}\n{canonical_uri}\n\n{canonical_headers}\n{signed_headers}\n{payload_hash}",
            method.as_str()
        );
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
            hex_sha256(canonical_request.as_bytes())
        );
        let signature = sign_v4(
            &self.credentials.secret_access_key,
            &date,
            &self.region,
            &string_to_sign,
        )?;
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            self.credentials.access_key_id
        );

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("host", host.parse()?);
        headers.insert("x-amz-content-sha256", payload_hash.parse()?);
        headers.insert("x-amz-date", amz_date.parse()?);
        headers.insert("authorization", authorization.parse()?);

        Ok(SignedS3Request {
            method,
            url,
            headers,
            body,
        })
    }

    fn object_url(&self, key: &str) -> Result<Url> {
        let encoded_key = encode_s3_key(key);
        if let Some(endpoint) = &self.endpoint {
            let endpoint = endpoint.trim_end_matches('/');
            return Url::parse(&format!("{endpoint}/{}/{encoded_key}", self.bucket))
                .context("failed to build S3-compatible endpoint URL");
        }
        let region = if self.region == "auto" {
            "us-east-1"
        } else {
            &self.region
        };
        Url::parse(&format!(
            "https://{}.s3.{region}.amazonaws.com/{encoded_key}",
            self.bucket
        ))
        .context("failed to build AWS S3 URL")
    }
}

struct SignedS3Request {
    method: Method,
    url: Url,
    headers: reqwest::header::HeaderMap,
    body: Vec<u8>,
}

pub struct S3Response {
    pub body: Vec<u8>,
}

async fn ensure_success(response: reqwest::Response) -> Result<S3Response> {
    let status = response.status();
    let body = response
        .bytes()
        .await
        .context("failed to read S3 response body")?
        .to_vec();
    if !status.is_success() {
        return Err(error!(
            "S3 request failed with status {status}: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    Ok(S3Response { body })
}

fn encode_s3_key(key: &str) -> String {
    key.split('/')
        .map(|segment| percent_encode(segment.as_bytes(), NON_ALPHANUMERIC).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

fn hex_sha256(bytes: impl AsRef<[u8]>) -> String {
    format!("{:x}", Sha256::digest(bytes.as_ref()))
}

fn sign_v4(secret: &str, date: &str, region: &str, string_to_sign: &str) -> Result<String> {
    let k_date = hmac_sha256(format!("AWS4{secret}").as_bytes(), date.as_bytes())?;
    let k_region = hmac_sha256(&k_date, region.as_bytes())?;
    let k_service = hmac_sha256(&k_region, b"s3")?;
    let k_signing = hmac_sha256(&k_service, b"aws4_request")?;
    Ok(hex::encode(hmac_sha256(
        &k_signing,
        string_to_sign.as_bytes(),
    )?))
}

fn hmac_sha256(key: &[u8], value: &[u8]) -> Result<Vec<u8>> {
    let mut mac = HmacSha256::new_from_slice(key).context("invalid HMAC key")?;
    mac.update(value);
    Ok(mac.finalize().into_bytes().to_vec())
}

fn safe_segment(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{EncryptionMode, RetentionPolicy};
    use chrono::{NaiveDate, TimeZone};
    use uuid::Uuid;

    #[test]
    fn local_storage_writes_archive_and_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let destination = StorageDestination {
            id: Uuid::now_v7(),
            team_id: Uuid::now_v7(),
            name: "Local".to_string(),
            kind: StorageKind::LocalFilesystem {
                root: dir.path().to_string_lossy().to_string(),
            },
            encryption: EncryptionMode::Disabled,
            retention: RetentionPolicy::default(),
        };
        let started_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
        );
        let manifest = BackupManifest {
            schema_version: 1,
            project_id: Uuid::now_v7(),
            target_id: Uuid::now_v7(),
            run_id: Uuid::now_v7(),
            deployment: "prod:careful-otter-123".to_string(),
            convex_cli_version: "test".to_string(),
            include_file_storage: true,
            archive_size_bytes: 5,
            sha256: "abc".to_string(),
            started_at,
            finished_at: started_at,
            duration_seconds: 0,
            storage_uri: "preupload://test-run".to_string(),
        };

        let stored = store_local_backup(
            &destination,
            "Client A",
            "prod:careful-otter-123",
            b"bytes",
            &manifest,
        )
        .unwrap();

        assert_eq!(std::fs::read(&stored.archive_path).unwrap(), b"bytes");
        let manifest_json = std::fs::read_to_string(&stored.manifest_path).unwrap();
        assert!(manifest_json.contains("careful-otter"));
        assert!(manifest_json.contains(&stored.storage_uri));
    }

    #[test]
    fn local_retention_prunes_old_archive_manifest_pairs() {
        let dir = tempfile::tempdir().unwrap();
        let destination = StorageDestination {
            id: Uuid::now_v7(),
            team_id: Uuid::now_v7(),
            name: "Local".to_string(),
            kind: StorageKind::LocalFilesystem {
                root: dir.path().to_string_lossy().to_string(),
            },
            encryption: EncryptionMode::Disabled,
            retention: RetentionPolicy {
                keep_last: Some(2),
                keep_days: None,
                keep_weeklies: None,
                keep_monthlies: None,
            },
        };
        let backup_dir = dir.path().join("Project").join("prod");
        std::fs::create_dir_all(&backup_dir).unwrap();
        for index in 0..4 {
            let archive = backup_dir.join(format!("2026070{index}T000000Z-run.zip"));
            let manifest = backup_dir.join(format!("2026070{index}T000000Z-run.zip.manifest.json"));
            std::fs::write(archive, b"zip").unwrap();
            std::fs::write(manifest, b"{}").unwrap();
        }

        let result = prune_local_retention(&destination, "Project", "prod").unwrap();

        assert_eq!(result.deleted_archives, 2);
        assert_eq!(result.deleted_manifests, 2);
        assert_eq!(
            std::fs::read_dir(backup_dir).unwrap().count(),
            4,
            "two archive/manifest pairs should remain"
        );
    }
}
