use crate::evidence::{EvidenceRecord, EvidenceStore};
use anyhow::{bail, Context, Result};
use printpdf::{BuiltinFont, Mm, PdfDocument};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EvidenceExportFormat {
    Json,
    Csv,
    Pdf,
}

impl std::str::FromStr for EvidenceExportFormat {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.to_ascii_lowercase().as_str() {
            "json" => Ok(Self::Json),
            "csv" => Ok(Self::Csv),
            "pdf" => Ok(Self::Pdf),
            _ => bail!("unsupported evidence export format: {value}"),
        }
    }
}

pub fn export_batch_records(
    store: &EvidenceStore,
    batch_id: Uuid,
    format: EvidenceExportFormat,
    output: impl AsRef<Path>,
) -> Result<()> {
    let records = store.records_for_batch(batch_id)?;
    match format {
        EvidenceExportFormat::Json => export_json(&records, output),
        EvidenceExportFormat::Csv => export_csv(&records, output),
        EvidenceExportFormat::Pdf => export_pdf(&records, batch_id, output),
    }
}

fn export_json(records: &[EvidenceRecord], output: impl AsRef<Path>) -> Result<()> {
    let file = File::create(output.as_ref())
        .with_context(|| format!("create {}", output.as_ref().display()))?;
    serde_json::to_writer_pretty(file, records)?;
    Ok(())
}

fn export_csv(records: &[EvidenceRecord], output: impl AsRef<Path>) -> Result<()> {
    let mut writer = csv::Writer::from_path(output.as_ref())
        .with_context(|| format!("create {}", output.as_ref().display()))?;
    for record in records {
        writer.serialize(record)?;
    }
    writer.flush()?;
    Ok(())
}

fn export_pdf(records: &[EvidenceRecord], batch_id: Uuid, output: impl AsRef<Path>) -> Result<()> {
    let (doc, page1, layer1) = PdfDocument::new(
        "Blind Watermark Evidence Report",
        Mm(210.0),
        Mm(297.0),
        "Layer 1",
    );
    let layer = doc.get_page(page1).get_layer(layer1);
    let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
    let bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;

    layer.use_text(
        "Blind Watermark Evidence Report",
        18.0,
        Mm(18.0),
        Mm(278.0),
        &bold,
    );
    layer.use_text(
        format!("Batch ID: {batch_id}"),
        10.0,
        Mm(18.0),
        Mm(266.0),
        &font,
    );
    layer.use_text(
        format!("Records: {}", records.len()),
        10.0,
        Mm(18.0),
        Mm(259.0),
        &font,
    );

    let mut y = 246.0;
    for record in records.iter().take(26) {
        let line = format!(
            "{} | {} | {} | PSNR {:.2}",
            record.watermark_id,
            record.owner,
            record.output_path.display(),
            record.psnr
        );
        layer.use_text(line, 8.0, Mm(18.0), Mm(y), &font);
        y -= 8.0;
    }
    if records.len() > 26 {
        layer.use_text(
            format!(
                "... {} more records omitted from PDF preview; export JSON/CSV for complete data.",
                records.len() - 26
            ),
            8.0,
            Mm(18.0),
            Mm(y),
            &font,
        );
    }

    let mut writer = BufWriter::new(File::create(output.as_ref())?);
    doc.save(&mut writer)?;
    Ok(())
}
