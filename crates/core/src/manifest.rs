use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestInput {
    pub project_id: Uuid,
    pub target_id: Uuid,
    pub run_id: Uuid,
    pub deployment: String,
    pub convex_cli_version: String,
    pub include_file_storage: bool,
    pub archive_bytes: Vec<u8>,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub storage_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TableInventory {
    pub table_name: String,
    pub document_count: usize,
    pub uncompressed_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupInventory {
    pub total_tables: usize,
    pub total_documents: usize,
    pub total_storage_files: usize,
    pub storage_files_bytes: u64,
    pub tables: Vec<TableInventory>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackupManifest {
    pub schema_version: u32,
    pub project_id: Uuid,
    pub target_id: Uuid,
    pub run_id: Uuid,
    pub deployment: String,
    pub convex_cli_version: String,
    pub include_file_storage: bool,
    pub archive_size_bytes: u64,
    pub sha256: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub duration_seconds: i64,
    pub storage_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventory: Option<BackupInventory>,
}

impl BackupManifest {
    pub fn from_input(input: ManifestInput) -> Self {
        let sha256 = Sha256::digest(&input.archive_bytes);
        let inventory = parse_archive_inventory(&input.archive_bytes);
        Self {
            schema_version: 1,
            project_id: input.project_id,
            target_id: input.target_id,
            run_id: input.run_id,
            deployment: input.deployment,
            convex_cli_version: input.convex_cli_version,
            include_file_storage: input.include_file_storage,
            archive_size_bytes: input.archive_bytes.len() as u64,
            sha256: format!("{sha256:x}"),
            duration_seconds: (input.finished_at - input.started_at).num_seconds(),
            started_at: input.started_at,
            finished_at: input.finished_at,
            storage_uri: input.storage_uri,
            inventory,
        }
    }
}

pub fn parse_archive_inventory(archive_bytes: &[u8]) -> Option<BackupInventory> {
    use std::io::Read;
    let cursor = std::io::Cursor::new(archive_bytes);
    let mut archive = zip::ZipArchive::new(cursor).ok()?;

    let mut tables_map: std::collections::BTreeMap<String, (usize, u64)> = std::collections::BTreeMap::new();
    let mut total_storage_files = 0;
    let mut storage_files_bytes = 0;

    for i in 0..archive.len() {
        let mut file = match archive.by_index(i) {
            Ok(file) => file,
            Err(_) => continue,
        };
        let name = file.name().to_string();
        let uncompressed_size = file.size();

        if name.starts_with("_storage/") && !name.ends_with('/') {
            if !name.ends_with("documents.jsonl") && !name.ends_with("generated_schema.jsonl") {
                total_storage_files += 1;
                storage_files_bytes += uncompressed_size;
            }
        }

        if name.ends_with("/documents.jsonl") || name.ends_with(".jsonl") {
            let raw_table_name = if let Some(stripped) = name.strip_suffix("/documents.jsonl") {
                stripped
            } else if let Some(stripped) = name.strip_suffix(".jsonl") {
                stripped
            } else {
                continue;
            };

            if raw_table_name == "_storage" || raw_table_name.ends_with("generated_schema") {
                continue;
            }

            let mut contents = String::new();
            if file.read_to_string(&mut contents).is_ok() {
                let doc_count = contents.lines().filter(|l| !l.trim().is_empty()).count();
                let entry = tables_map.entry(raw_table_name.to_string()).or_insert((0, 0));
                entry.0 += doc_count;
                entry.1 += uncompressed_size;
            }
        }
    }

    let mut total_documents = 0;
    let mut tables = Vec::with_capacity(tables_map.len());
    for (table_name, (doc_count, bytes)) in tables_map {
        total_documents += doc_count;
        tables.push(TableInventory {
            table_name,
            document_count: doc_count,
            uncompressed_bytes: bytes,
        });
    }

    tables.sort_by(|a, b| b.document_count.cmp(&a.document_count));

    Some(BackupInventory {
        total_tables: tables.len(),
        total_documents,
        total_storage_files,
        storage_files_bytes,
        tables,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{NaiveDate, TimeZone};

    #[test]
    fn manifest_records_checksum_size_and_duration() {
        let started_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, 1)
                .unwrap()
                .and_hms_opt(10, 0, 0)
                .unwrap(),
        );
        let finished_at = Utc.from_utc_datetime(
            &NaiveDate::from_ymd_opt(2026, 7, 1)
                .unwrap()
                .and_hms_opt(10, 2, 0)
                .unwrap(),
        );
        let manifest = BackupManifest::from_input(ManifestInput {
            project_id: Uuid::now_v7(),
            target_id: Uuid::now_v7(),
            run_id: Uuid::now_v7(),
            deployment: "prod:careful-otter-123".to_string(),
            convex_cli_version: "1.28.0".to_string(),
            include_file_storage: true,
            archive_bytes: b"convex backup bytes".to_vec(),
            started_at,
            finished_at,
            storage_uri: "file:///backups/prod.zip".to_string(),
        });

        assert_eq!(manifest.schema_version, 1);
        assert_eq!(manifest.archive_size_bytes, 19);
        assert_eq!(manifest.duration_seconds, 120);
        assert_eq!(
            manifest.sha256,
            "f953a3e53e0ec7e939ccacc7fb7c0bf3b60874d2a954fe8aa07f1d418fee6f85"
        );
    }
}
