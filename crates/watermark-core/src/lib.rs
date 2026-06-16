pub mod batch;
pub mod evidence;
pub mod export;
pub mod hash;
pub mod payload;
pub mod transform;
pub mod watermark;

pub use batch::{
    batch_embed, batch_extract, collect_image_paths, BatchEmbedSummary, BatchExtractSummary,
};
pub use evidence::{EvidenceRecord, EvidenceStore, ExtractionEvidence};
pub use export::{export_batch_records, EvidenceExportFormat};
pub use payload::{PayloadError, WatermarkPayload};
pub use watermark::{
    embed_file, extract_file, verify_file, AlgorithmProfile, EccStatus, EmbedOptions,
    ExtractResult, OutputFormat,
};
