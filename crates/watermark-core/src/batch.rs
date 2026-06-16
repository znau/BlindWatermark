use crate::evidence::{EvidenceStore, NewEvidenceRecord, NewExtractionEvidence};
use crate::watermark::{embed_file, extract_file, EmbedOptions, ExtractResult};
use anyhow::{Context, Result};
use chrono::Utc;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchEmbedSummary {
    pub batch_id: Uuid,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub failures: Vec<FileFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchExtractSummary {
    pub batch_id: Uuid,
    pub total: usize,
    pub succeeded: usize,
    pub failed: usize,
    pub results: Vec<ExtractResult>,
    pub failures: Vec<FileFailure>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileFailure {
    pub path: PathBuf,
    pub error: String,
}

pub fn collect_image_paths(path: impl AsRef<Path>) -> Result<Vec<PathBuf>> {
    let path = path.as_ref();
    if path.is_file() {
        return Ok(if is_supported_image(path) {
            vec![path.to_path_buf()]
        } else {
            vec![]
        });
    }

    let mut files = Vec::new();
    for entry in WalkDir::new(path).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_file() && is_supported_image(entry.path()) {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

pub fn batch_embed(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    owner: &str,
    key: &[u8],
    options: &EmbedOptions,
    store: &EvidenceStore,
) -> Result<BatchEmbedSummary> {
    let input = input.as_ref();
    let output_dir = output_dir.as_ref();
    let files = collect_image_paths(input)?;
    let batch_id = store.create_batch("embed", Some(owner))?;

    let outcomes = files
        .par_iter()
        .map(|path| {
            let relative = output_relative_path(input, path);
            let mut output_path = output_dir.join(relative);
            let extension = options.output_format.extension(path);
            output_path.set_extension(extension);
            embed_file(path, &output_path, key, options)
                .with_context(|| format!("embed {}", path.display()))
                .map(|embedded| (path.clone(), embedded))
                .map_err(|err| (path.clone(), err))
        })
        .collect::<Vec<_>>();

    let mut failures = Vec::new();
    let mut succeeded = 0_usize;
    for outcome in outcomes {
        match outcome {
            Ok((_path, embedded)) => {
                let record =
                    NewEvidenceRecord::from_embedded(batch_id, owner.to_string(), embedded);
                store.insert_embed_record(&record)?;
                succeeded += 1;
            }
            Err((path, err)) => failures.push(FileFailure {
                path,
                error: format!("{err:#}"),
            }),
        }
    }

    let status = if failures.is_empty() {
        "completed"
    } else {
        "completed-with-errors"
    };
    store.finish_batch(batch_id, status)?;

    Ok(BatchEmbedSummary {
        batch_id,
        total: files.len(),
        succeeded,
        failed: failures.len(),
        failures,
    })
}

pub fn batch_extract(
    input: impl AsRef<Path>,
    key: &[u8],
    store: &EvidenceStore,
) -> Result<BatchExtractSummary> {
    let files = collect_image_paths(input)?;
    let batch_id = store.create_batch("extract", None)?;
    let outcomes = files
        .par_iter()
        .map(|path| {
            extract_file(path, key)
                .with_context(|| format!("extract {}", path.display()))
                .map(|result| (path.clone(), result))
                .map_err(|err| (path.clone(), err))
        })
        .collect::<Vec<_>>();

    let mut failures = Vec::new();
    let mut results = Vec::new();
    for outcome in outcomes {
        match outcome {
            Ok((path, result)) => {
                let record = NewExtractionEvidence {
                    batch_id: Some(batch_id),
                    input_path: path,
                    detected_watermark_id: result.watermark_id,
                    expected_watermark_id: None,
                    confidence: result.confidence,
                    ecc_status: result.ecc_status.clone(),
                    diagnostics: result.diagnostics.clone(),
                    created_at: Utc::now(),
                };
                store.insert_extraction_record(&record)?;
                results.push(result);
            }
            Err((path, err)) => failures.push(FileFailure {
                path,
                error: format!("{err:#}"),
            }),
        }
    }

    let status = if failures.is_empty() {
        "completed"
    } else {
        "completed-with-errors"
    };
    store.finish_batch(batch_id, status)?;

    Ok(BatchExtractSummary {
        batch_id,
        total: files.len(),
        succeeded: results.len(),
        failed: failures.len(),
        results,
        failures,
    })
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "jpg" | "jpeg" | "png" | "webp" | "tif" | "tiff"
            )
        })
        .unwrap_or(false)
}

fn output_relative_path(input_root: &Path, path: &Path) -> PathBuf {
    if input_root.is_file() {
        return path
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("watermarked"));
    }
    path.strip_prefix(input_root).unwrap_or(path).to_path_buf()
}
