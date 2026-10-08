use crate::crypto::ENCRYPTED_EXTENSION;
use crate::{
    AppDatabase, BackupManifest, SecretVault, StorageDestination, StorageKind,
    paths::safe_backup_relative_path,
};
use crate::{Result, ResultContext, error};
use chrono::Utc;
use hmac::{Hmac, Mac};
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

impl RetentionPruneResult {
    fn none() -> Self {
        Self {
            deleted_archives: 0,
            deleted_manifests: 0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct S3CredentialSecret {
    pub access_key_id: String,
    pub secret_access_key: String,
}

const MANIFEST_SUFFIX: &str = ".manifest.json";

/// Archive file name for a run: `<timestamp>-<run id>.zip`, plus `.age` when encrypted.
fn archive_file_name(manifest: &BackupManifest) -> String {
    let base = format!(
        "{}-{}.zip",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        manifest.run_id
    );
    if manifest.encryption.is_some() {
        format!("{base}.{ENCRYPTED_EXTENSION}")
    } else {
        base
    }
}

/// Writes `stored_bytes` (already encrypted if the manifest says so) and its manifest.
pub async fn store_backup(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    stored_bytes: &[u8],
    manifest: &BackupManifest,
) -> Result<StoredBackup> {
    match &destination.kind {
        StorageKind::LocalFilesystem { .. } => store_local_backup(
            destination,
            project_name,
            deployment,
            stored_bytes,
            manifest,
        ),
        StorageKind::S3Compatible { .. } => {
            store_s3_backup(
                database,
                destination,
                project_name,
                deployment,
                stored_bytes,
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
    stored_bytes: &[u8],
    manifest: &BackupManifest,
) -> Result<StoredBackup> {
    let StorageKind::LocalFilesystem { root } = &destination.kind else {
        return Err(error!(
            "destination {} is not local filesystem",
            destination.id
        ));
    };

    let archive_name = archive_file_name(manifest);
    let manifest_name = format!("{archive_name}{MANIFEST_SUFFIX}");
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

    let tmp_archive = parent.join(format!("{archive_name}.tmp"));
    std::fs::write(&tmp_archive, stored_bytes)
        .with_context(|| format!("failed to write {}", tmp_archive.display()))?;
    std::fs::rename(&tmp_archive, &archive_path)
        .with_context(|| format!("failed to commit {}", archive_path.display()))?;

    let storage_uri = format!("file://{}", archive_path.display());
    let stored_manifest = manifest_for_storage(manifest, &storage_uri);
    let manifest_json = serde_json::to_vec_pretty(&stored_manifest)?;
    let tmp_manifest = parent.join(format!("{manifest_name}.tmp"));
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

/// The manifest written next to an archive. For encrypted archives the table
/// inventory (table names and document counts) is omitted so the unencrypted
/// manifest does not leak schema details; the app database keeps the full copy.
fn manifest_for_storage(manifest: &BackupManifest, storage_uri: &str) -> BackupManifest {
    let mut stored = manifest.clone();
    stored.storage_uri = storage_uri.to_string();
    if stored.encryption.is_some() {
        stored.inventory = None;
    }
    stored
}

/// Applies `keep_last` retention to archive/manifest pairs for one deployment.
pub async fn prune_retention(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
) -> Result<RetentionPruneResult> {
    match &destination.kind {
        StorageKind::LocalFilesystem { .. } => {
            prune_local_retention(destination, project_name, deployment)
        }
        StorageKind::S3Compatible { .. } => {
            prune_s3_retention(database, destination, project_name, deployment).await
        }
    }
}

pub fn prune_local_retention(
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
) -> Result<RetentionPruneResult> {
    let StorageKind::LocalFilesystem { root } = &destination.kind else {
        return Ok(RetentionPruneResult::none());
    };
    let Some(keep_last) = destination.retention.keep_last else {
        return Ok(RetentionPruneResult::none());
    };
    let backup_dir = Path::new(root)
        .join(safe_segment(project_name))
        .join(safe_segment(deployment));
    if !backup_dir.exists() {
        return Ok(RetentionPruneResult::none());
    }

    let mut manifests = std::fs::read_dir(&backup_dir)?
        .filter_map(std::result::Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_manifest_name)
        })
        .collect::<Vec<_>>();
    manifests.sort();
    let keep_last = keep_last as usize;
    if manifests.len() <= keep_last {
        return Ok(RetentionPruneResult::none());
    }

    let delete_count = manifests.len() - keep_last;
    let mut result = RetentionPruneResult::none();
    for manifest_path in manifests.into_iter().take(delete_count) {
        let archive_name = manifest_path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(MANIFEST_SUFFIX))
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

async fn prune_s3_retention(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
) -> Result<RetentionPruneResult> {
    let StorageKind::S3Compatible { prefix, .. } = &destination.kind else {
        return Ok(RetentionPruneResult::none());
    };
    let Some(keep_last) = destination.retention.keep_last else {
        return Ok(RetentionPruneResult::none());
    };
    let client = s3_client_from_destination(database, destination)?;
    let base_key = object_key(prefix.as_deref(), project_name, deployment);
    let keys = client.list_keys(&format!("{base_key}/")).await?;
    let manifests = select_manifests_to_prune(&keys, keep_last as usize);
    let mut result = RetentionPruneResult::none();
    for manifest_key in manifests {
        let archive_key = manifest_key
            .strip_suffix(MANIFEST_SUFFIX)
            .ok_or_else(|| error!("invalid manifest key {manifest_key}"))?;
        if keys.iter().any(|key| key == archive_key) {
            client.delete_object(archive_key).await?;
            result.deleted_archives += 1;
        }
        client.delete_object(&manifest_key).await?;
        result.deleted_manifests += 1;
    }
    Ok(result)
}

/// Oldest manifests beyond `keep_last`; names sort chronologically by timestamp prefix.
fn select_manifests_to_prune(keys: &[String], keep_last: usize) -> Vec<String> {
    let mut manifests = keys
        .iter()
        .filter(|key| key.rsplit('/').next().is_some_and(is_manifest_name))
        .cloned()
        .collect::<Vec<_>>();
    manifests.sort();
    let delete_count = manifests.len().saturating_sub(keep_last);
    manifests.into_iter().take(delete_count).collect()
}

fn is_manifest_name(name: &str) -> bool {
    name.ends_with(MANIFEST_SUFFIX) && name.contains(".zip")
}

pub async fn store_s3_backup(
    database: &AppDatabase,
    destination: &StorageDestination,
    project_name: &str,
    deployment: &str,
    stored_bytes: &[u8],
    manifest: &BackupManifest,
) -> Result<StoredBackup> {
    let StorageKind::S3Compatible { bucket, prefix, .. } = &destination.kind else {
        return Err(error!(
            "destination {} is not S3-compatible",
            destination.id
        ));
    };

    let client = s3_client_from_destination(database, destination)?;

    let archive_name = archive_file_name(manifest);
    let manifest_name = format!("{archive_name}{MANIFEST_SUFFIX}");
    let base_key = object_key(prefix.as_deref(), project_name, deployment);
    let archive_key = format!("{base_key}/{archive_name}");
    let manifest_key = format!("{base_key}/{manifest_name}");
    let storage_uri = format!("s3://{bucket}/{archive_key}");
    let stored_manifest = manifest_for_storage(manifest, &storage_uri);

    client
        .put_object(&archive_key, stored_bytes.to_vec())
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

/// Reads the stored (possibly encrypted) archive bytes for `storage_uri` from `destination`.
pub async fn read_stored_archive(
    database: &AppDatabase,
    destination: Option<&StorageDestination>,
    storage_uri: &str,
) -> Result<Vec<u8>> {
    if let Some(path) = storage_uri.strip_prefix("file://") {
        return std::fs::read(path).with_context(|| format!("failed to read archive {path}"));
    }
    if storage_uri.starts_with("s3://") {
        let destination = destination
            .filter(|destination| matches!(destination.kind, StorageKind::S3Compatible { .. }))
            .ok_or_else(|| error!("S3 archive {storage_uri} has no S3 destination configured"))?;
        let client = s3_client_from_destination(database, destination)?;
        let key = s3_object_key_from_uri(storage_uri)?;
        return client
            .get_object(&key)
            .await
            .context("failed to read S3 archive");
    }
    Err(error!("unsupported archive URI {storage_uri}"))
}

/// Writes, reads back and deletes a small probe object to prove the destination works.
pub async fn test_destination(
    database: &AppDatabase,
    destination: &StorageDestination,
) -> Result<String> {
    let probe_name = format!(".convex-autobackup-probe-{}", uuid::Uuid::now_v7());
    let payload = format!("convex-autobackup probe {}", Utc::now().to_rfc3339()).into_bytes();
    match &destination.kind {
        StorageKind::LocalFilesystem { root } => {
            std::fs::create_dir_all(root)
                .with_context(|| format!("failed to create backup directory {root}"))?;
            let path = Path::new(root).join(&probe_name);
            std::fs::write(&path, &payload).with_context(|| format!("cannot write to {root}"))?;
            let read_back = std::fs::read(&path);
            let _ = std::fs::remove_file(&path);
            if read_back.context("cannot read back probe file")? != payload {
                return Err(error!("probe file contents did not match"));
            }
            Ok(format!("wrote, read and deleted a probe file in {root}"))
        }
        StorageKind::S3Compatible { bucket, prefix, .. } => {
            let client = s3_client_from_destination(database, destination)?;
            let key = [
                prefix.as_deref().unwrap_or("").trim_matches('/'),
                &probe_name,
            ]
            .into_iter()
            .filter(|segment| !segment.is_empty())
            .collect::<Vec<_>>()
            .join("/");
            client
                .put_object(&key, payload.clone())
                .await
                .context("upload probe failed")?;
            let read_back = client.get_object(&key).await;
            let deleted = client.delete_object(&key).await;
            if read_back.context("download probe failed")? != payload {
                return Err(error!("probe object contents did not match"));
            }
            deleted.context("delete probe failed")?;
            Ok(format!(
                "wrote, read and deleted a probe object in bucket {bucket}"
            ))
        }
    }
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
        let request = self.signed_request(Method::PUT, Some(key), &[], body)?;
        self.send(request, "PUT").await
    }

    pub async fn get_object(&self, key: &str) -> Result<Vec<u8>> {
        let request = self.signed_request(Method::GET, Some(key), &[], Vec::new())?;
        Ok(self.send(request, "GET").await?.body)
    }

    pub async fn delete_object(&self, key: &str) -> Result<()> {
        let request = self.signed_request(Method::DELETE, Some(key), &[], Vec::new())?;
        self.send(request, "DELETE").await?;
        Ok(())
    }

    /// Lists every object key under `prefix` (ListObjectsV2, following continuation tokens).
    pub async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let mut continuation: Option<String> = None;
        loop {
            let mut query = vec![
                ("list-type".to_string(), "2".to_string()),
                ("prefix".to_string(), prefix.to_string()),
            ];
            if let Some(token) = &continuation {
                query.push(("continuation-token".to_string(), token.clone()));
            }
            let request = self.signed_request(Method::GET, None, &query, Vec::new())?;
            let response = self.send(request, "LIST").await?;
            let page = parse_list_objects_v2(&String::from_utf8_lossy(&response.body));
            keys.extend(page.keys);
            match page.next_continuation_token {
                Some(token) if page.is_truncated => continuation = Some(token),
                _ => break,
            }
        }
        Ok(keys)
    }

    async fn send(&self, request: SignedS3Request, operation: &str) -> Result<S3Response> {
        let response = self
            .http
            .request(request.method, request.url)
            .headers(request.headers)
            .body(request.body)
            .send()
            .await
            .with_context(|| format!("failed to send S3 {operation} request"))?;
        ensure_success(response).await
    }

    fn signed_request(
        &self,
        method: Method,
        key: Option<&str>,
        query: &[(String, String)],
        body: Vec<u8>,
    ) -> Result<SignedS3Request> {
        let now = Utc::now();
        let canonical_query = canonical_query_string(query);
        let mut url = match key {
            Some(key) => self.object_url(key)?,
            None => self.bucket_url()?,
        };
        if !canonical_query.is_empty() {
            url.set_query(Some(&canonical_query));
        }
        let host = match url.port() {
            Some(port) => format!(
                "{}:{port}",
                url.host_str()
                    .ok_or_else(|| error!("S3 endpoint URL has no host"))?
            ),
            None => url
                .host_str()
                .ok_or_else(|| error!("S3 endpoint URL has no host"))?
                .to_string(),
        };
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
            "{}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}",
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
        Url::parse(&format!("{}/{encoded_key}", self.aws_bucket_host()))
            .context("failed to build AWS S3 URL")
    }

    fn bucket_url(&self) -> Result<Url> {
        if let Some(endpoint) = &self.endpoint {
            let endpoint = endpoint.trim_end_matches('/');
            return Url::parse(&format!("{endpoint}/{}", self.bucket))
                .context("failed to build S3-compatible endpoint URL");
        }
        Url::parse(&format!("{}/", self.aws_bucket_host())).context("failed to build AWS S3 URL")
    }

    fn aws_bucket_host(&self) -> String {
        let region = if self.region == "auto" {
            "us-east-1"
        } else {
            &self.region
        };
        format!("https://{}.s3.{region}.amazonaws.com", self.bucket)
    }
}

/// SigV4 canonical query string: RFC 3986-encoded pairs sorted by key then value.
fn canonical_query_string(query: &[(String, String)]) -> String {
    let mut pairs = query
        .iter()
        .map(|(key, value)| (uri_encode(key), uri_encode(value)))
        .collect::<Vec<_>>();
    pairs.sort();
    pairs
        .into_iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

/// Percent-encodes everything except RFC 3986 unreserved characters.
fn uri_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ListObjectsPage {
    keys: Vec<String>,
    is_truncated: bool,
    next_continuation_token: Option<String>,
}

fn parse_list_objects_v2(xml: &str) -> ListObjectsPage {
    ListObjectsPage {
        keys: xml_values(xml, "Key")
            .into_iter()
            .map(|key| xml_unescape(&key))
            .collect(),
        is_truncated: xml_values(xml, "IsTruncated")
            .first()
            .is_some_and(|value| value == "true"),
        next_continuation_token: xml_values(xml, "NextContinuationToken")
            .into_iter()
            .next()
            .map(|token| xml_unescape(&token)),
    }
}

fn xml_values(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut values = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let Some(end) = after.find(&close) else {
            break;
        };
        values.push(after[..end].to_string());
        rest = &after[end + close.len()..];
    }
    values
}

fn xml_unescape(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
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
    key.split('/').map(uri_encode).collect::<Vec<_>>().join("/")
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
            inventory: None,
            destination_id: None,
            encryption: None,
            stored_sha256: None,
            stored_size_bytes: None,
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

    #[test]
    fn canonical_query_sorts_and_encodes() {
        let query = vec![
            ("prefix".to_string(), "Client A/prod:x/".to_string()),
            ("list-type".to_string(), "2".to_string()),
            ("continuation-token".to_string(), "a+b/c=".to_string()),
        ];
        assert_eq!(
            canonical_query_string(&query),
            "continuation-token=a%2Bb%2Fc%3D&list-type=2&prefix=Client%20A%2Fprod%3Ax%2F"
        );
    }

    #[test]
    fn parses_list_objects_v2_page() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<ListBucketResult><Name>b</Name><IsTruncated>true</IsTruncated>
<Contents><Key>p/a.zip</Key><Size>1</Size></Contents>
<Contents><Key>p/a.zip.manifest.json</Key></Contents>
<Contents><Key>p/R&amp;D.zip</Key></Contents>
<NextContinuationToken>tok&amp;1</NextContinuationToken></ListBucketResult>"#;
        let page = parse_list_objects_v2(xml);
        assert_eq!(
            page.keys,
            vec!["p/a.zip", "p/a.zip.manifest.json", "p/R&D.zip"]
        );
        assert!(page.is_truncated);
        assert_eq!(page.next_continuation_token.as_deref(), Some("tok&1"));
    }

    #[test]
    fn prune_selection_keeps_newest_manifests() {
        let keys = [
            "p/20260701T000000Z-a.zip",
            "p/20260701T000000Z-a.zip.manifest.json",
            "p/20260702T000000Z-b.zip.age",
            "p/20260702T000000Z-b.zip.age.manifest.json",
            "p/20260703T000000Z-c.zip.age",
            "p/20260703T000000Z-c.zip.age.manifest.json",
            "p/.convex-autobackup-probe-x",
        ]
        .map(String::from);
        assert_eq!(
            select_manifests_to_prune(&keys, 2),
            vec!["p/20260701T000000Z-a.zip.manifest.json".to_string()]
        );
        assert!(select_manifests_to_prune(&keys, 5).is_empty());
    }

    #[test]
    fn r2_urls_are_path_style() {
        let client = S3CompatibleClient::new(
            "bucket".to_string(),
            "auto".to_string(),
            Some("https://acct.r2.cloudflarestorage.com/".to_string()),
            S3CredentialSecret {
                access_key_id: "id".to_string(),
                secret_access_key: "secret".to_string(),
            },
        )
        .unwrap();
        assert_eq!(
            client.object_url("Client A/x.zip").unwrap().as_str(),
            "https://acct.r2.cloudflarestorage.com/bucket/Client%20A/x.zip"
        );
        let request = client
            .signed_request(
                Method::GET,
                None,
                &[("list-type".to_string(), "2".to_string())],
                Vec::new(),
            )
            .unwrap();
        assert_eq!(
            request.url.as_str(),
            "https://acct.r2.cloudflarestorage.com/bucket?list-type=2"
        );
    }

    #[tokio::test]
    async fn local_destination_probe_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let db = AppDatabase::open(dir.path().join("app.db")).unwrap();
        let destination = StorageDestination {
            id: Uuid::now_v7(),
            team_id: Uuid::now_v7(),
            name: "Local".to_string(),
            kind: StorageKind::LocalFilesystem {
                root: dir.path().join("backups").to_string_lossy().to_string(),
            },
            encryption: EncryptionMode::Disabled,
            retention: RetentionPolicy::default(),
        };
        test_destination(&db, &destination).await.unwrap();
        assert_eq!(
            std::fs::read_dir(dir.path().join("backups"))
                .unwrap()
                .count(),
            0
        );
    }

    #[test]
    fn encrypted_manifest_files_omit_inventory() {
        let started_at = Utc::now();
        let mut manifest = crate::BackupManifest::from_input(crate::ManifestInput {
            project_id: Uuid::now_v7(),
            target_id: Uuid::now_v7(),
            run_id: Uuid::now_v7(),
            deployment: "prod:x".to_string(),
            convex_cli_version: "test".to_string(),
            include_file_storage: true,
            archive_bytes: b"bytes".to_vec(),
            started_at,
            finished_at: started_at,
            storage_uri: "preupload://x".to_string(),
        });
        manifest.inventory = Some(crate::manifest::BackupInventory {
            total_tables: 1,
            total_documents: 3,
            total_storage_files: 0,
            storage_files_bytes: 0,
            tables: vec![crate::manifest::TableInventory {
                table_name: "secret_customers".to_string(),
                document_count: 3,
                uncompressed_bytes: 10,
            }],
        });
        assert!(
            manifest_for_storage(&manifest, "s3://b/k")
                .inventory
                .is_some()
        );
        manifest.encryption = Some(crate::crypto::ArchiveEncryption::age_scrypt(Uuid::now_v7()));
        let stored = manifest_for_storage(&manifest, "s3://b/k");
        assert!(stored.inventory.is_none());
        assert!(
            !serde_json::to_string(&stored)
                .unwrap()
                .contains("secret_customers")
        );
    }
}
