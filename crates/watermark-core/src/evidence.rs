use crate::watermark::{AlgorithmProfile, EccStatus, ALGORITHM_VERSION};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRecord {
    pub id: i64,
    pub batch_id: Uuid,
    pub watermark_id: Uuid,
    pub owner: String,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub input_sha256: String,
    pub output_sha256: String,
    pub perceptual_hash: String,
    pub algorithm_version: String,
    pub profile: AlgorithmProfile,
    pub psnr: f32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionEvidence {
    pub id: i64,
    pub batch_id: Option<Uuid>,
    pub input_path: PathBuf,
    pub detected_watermark_id: Option<Uuid>,
    pub expected_watermark_id: Option<Uuid>,
    pub confidence: f32,
    pub ecc_status: EccStatus,
    pub diagnostics: Vec<String>,
    pub created_at: DateTime<Utc>,
}

pub struct EvidenceStore {
    conn: Connection,
}

impl EvidenceStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .with_context(|| format!("open evidence database {}", path.as_ref().display()))?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let store = Self { conn };
        store.migrate()?;
        Ok(store)
    }

    pub fn create_batch(&self, kind: &str, owner: Option<&str>) -> Result<Uuid> {
        let batch_id = Uuid::new_v4();
        let now = Utc::now();
        self.conn.execute(
            "insert into batches (batch_id, kind, owner, status, created_at, updated_at)
             values (?1, ?2, ?3, 'running', ?4, ?4)",
            params![
                batch_id.to_string(),
                kind,
                owner.unwrap_or_default(),
                now.to_rfc3339()
            ],
        )?;
        Ok(batch_id)
    }

    pub fn finish_batch(&self, batch_id: Uuid, status: &str) -> Result<()> {
        self.conn.execute(
            "update batches set status = ?2, updated_at = ?3 where batch_id = ?1",
            params![batch_id.to_string(), status, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn insert_embed_record(&self, record: &NewEvidenceRecord) -> Result<i64> {
        self.conn.execute(
            "insert into evidence_records (
                batch_id, watermark_id, owner, input_path, output_path, input_sha256,
                output_sha256, perceptual_hash, algorithm_version, profile, psnr, created_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.batch_id.to_string(),
                record.watermark_id.to_string(),
                &record.owner,
                record.input_path.display().to_string(),
                record.output_path.display().to_string(),
                &record.input_sha256,
                &record.output_sha256,
                &record.perceptual_hash,
                &record.algorithm_version,
                record.profile.to_string(),
                record.psnr as f64,
                record.created_at.to_rfc3339(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn insert_extraction_record(&self, record: &NewExtractionEvidence) -> Result<i64> {
        self.conn.execute(
            "insert into extraction_records (
                batch_id, input_path, detected_watermark_id, expected_watermark_id,
                confidence, ecc_status, diagnostics, created_at
             ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                record.batch_id.map(|id| id.to_string()),
                record.input_path.display().to_string(),
                record.detected_watermark_id.map(|id| id.to_string()),
                record.expected_watermark_id.map(|id| id.to_string()),
                record.confidence as f64,
                format!("{:?}", record.ecc_status),
                serde_json::to_string(&record.diagnostics)?,
                record.created_at.to_rfc3339(),
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn records_for_batch(&self, batch_id: Uuid) -> Result<Vec<EvidenceRecord>> {
        let mut stmt = self.conn.prepare(
            "select id, batch_id, watermark_id, owner, input_path, output_path, input_sha256,
                    output_sha256, perceptual_hash, algorithm_version, profile, psnr, created_at
             from evidence_records
             where batch_id = ?1
             order by id asc",
        )?;
        let records = stmt
            .query_map(params![batch_id.to_string()], map_evidence_record)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(records)
    }

    pub fn lookup_watermark(&self, watermark_id: Uuid) -> Result<Option<EvidenceRecord>> {
        self.conn
            .query_row(
                "select id, batch_id, watermark_id, owner, input_path, output_path, input_sha256,
                        output_sha256, perceptual_hash, algorithm_version, profile, psnr, created_at
                 from evidence_records
                 where watermark_id = ?1
                 limit 1",
                params![watermark_id.to_string()],
                map_evidence_record,
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn recent_records(&self, limit: usize) -> Result<Vec<EvidenceRecord>> {
        let mut stmt = self.conn.prepare(
            "select id, batch_id, watermark_id, owner, input_path, output_path, input_sha256,
                    output_sha256, perceptual_hash, algorithm_version, profile, psnr, created_at
             from evidence_records
             order by id desc
             limit ?1",
        )?;
        let records = stmt
            .query_map(params![limit as i64], map_evidence_record)?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(records)
    }

    fn migrate(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            pragma journal_mode = wal;
            create table if not exists batches (
                batch_id text primary key,
                kind text not null,
                owner text not null default '',
                status text not null,
                created_at text not null,
                updated_at text not null
            );
            create table if not exists evidence_records (
                id integer primary key autoincrement,
                batch_id text not null,
                watermark_id text not null unique,
                owner text not null,
                input_path text not null,
                output_path text not null,
                input_sha256 text not null,
                output_sha256 text not null,
                perceptual_hash text not null,
                algorithm_version text not null,
                profile text not null,
                psnr real not null,
                created_at text not null
            );
            create index if not exists idx_evidence_batch on evidence_records(batch_id);
            create index if not exists idx_evidence_watermark on evidence_records(watermark_id);
            create table if not exists extraction_records (
                id integer primary key autoincrement,
                batch_id text,
                input_path text not null,
                detected_watermark_id text,
                expected_watermark_id text,
                confidence real not null,
                ecc_status text not null,
                diagnostics text not null,
                created_at text not null
            );
            create table if not exists task_queue (
                task_id text primary key,
                kind text not null,
                status text not null,
                payload_json text not null,
                progress_total integer not null default 0,
                progress_done integer not null default 0,
                error text,
                created_at text not null,
                updated_at text not null
            );
            ",
        )?;
        Ok(())
    }
}

pub struct NewEvidenceRecord {
    pub batch_id: Uuid,
    pub watermark_id: Uuid,
    pub owner: String,
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub input_sha256: String,
    pub output_sha256: String,
    pub perceptual_hash: String,
    pub algorithm_version: String,
    pub profile: AlgorithmProfile,
    pub psnr: f32,
    pub created_at: DateTime<Utc>,
}

pub struct NewExtractionEvidence {
    pub batch_id: Option<Uuid>,
    pub input_path: PathBuf,
    pub detected_watermark_id: Option<Uuid>,
    pub expected_watermark_id: Option<Uuid>,
    pub confidence: f32,
    pub ecc_status: EccStatus,
    pub diagnostics: Vec<String>,
    pub created_at: DateTime<Utc>,
}

impl NewEvidenceRecord {
    pub fn from_embedded(
        batch_id: Uuid,
        owner: String,
        embedded: crate::watermark::EmbeddedFile,
    ) -> Self {
        Self {
            batch_id,
            watermark_id: embedded.watermark_id,
            owner,
            input_path: embedded.input_path,
            output_path: embedded.output_path,
            input_sha256: embedded.input_sha256,
            output_sha256: embedded.output_sha256,
            perceptual_hash: embedded.perceptual_hash,
            algorithm_version: embedded.algorithm_version,
            profile: embedded.profile,
            psnr: embedded.psnr,
            created_at: Utc::now(),
        }
    }
}

impl Default for NewEvidenceRecord {
    fn default() -> Self {
        Self {
            batch_id: Uuid::new_v4(),
            watermark_id: Uuid::new_v4(),
            owner: String::new(),
            input_path: PathBuf::new(),
            output_path: PathBuf::new(),
            input_sha256: String::new(),
            output_sha256: String::new(),
            perceptual_hash: String::new(),
            algorithm_version: ALGORITHM_VERSION.to_string(),
            profile: AlgorithmProfile::Balanced,
            psnr: 0.0,
            created_at: Utc::now(),
        }
    }
}

fn map_evidence_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<EvidenceRecord> {
    let profile: String = row.get(10)?;
    Ok(EvidenceRecord {
        id: row.get(0)?,
        batch_id: parse_uuid_col(row, 1)?,
        watermark_id: parse_uuid_col(row, 2)?,
        owner: row.get(3)?,
        input_path: PathBuf::from(row.get::<_, String>(4)?),
        output_path: PathBuf::from(row.get::<_, String>(5)?),
        input_sha256: row.get(6)?,
        output_sha256: row.get(7)?,
        perceptual_hash: row.get(8)?,
        algorithm_version: row.get(9)?,
        profile: parse_profile(&profile),
        psnr: row.get::<_, f64>(11)? as f32,
        created_at: parse_time_col(row, 12)?,
    })
}

fn parse_uuid_col(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<Uuid> {
    let value: String = row.get(idx)?;
    Uuid::parse_str(&value).map_err(|err| {
        rusqlite::Error::FromSqlConversionFailure(idx, rusqlite::types::Type::Text, Box::new(err))
    })
}

fn parse_time_col(row: &rusqlite::Row<'_>, idx: usize) -> rusqlite::Result<DateTime<Utc>> {
    let value: String = row.get(idx)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| {
            rusqlite::Error::FromSqlConversionFailure(
                idx,
                rusqlite::types::Type::Text,
                Box::new(err),
            )
        })
}

fn parse_profile(profile: &str) -> AlgorithmProfile {
    match profile {
        "strong" => AlgorithmProfile::Strong,
        "fidelity" => AlgorithmProfile::Fidelity,
        _ => AlgorithmProfile::Balanced,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_store_round_trip() {
        let store = EvidenceStore::memory().unwrap();
        let batch_id = store.create_batch("embed", Some("owner")).unwrap();
        let record = NewEvidenceRecord {
            batch_id,
            owner: "owner".to_string(),
            input_path: PathBuf::from("in.png"),
            output_path: PathBuf::from("out.png"),
            input_sha256: "a".repeat(64),
            output_sha256: "b".repeat(64),
            perceptual_hash: "00".to_string(),
            ..Default::default()
        };
        store.insert_embed_record(&record).unwrap();
        let records = store.records_for_batch(batch_id).unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].owner, "owner");
    }
}
