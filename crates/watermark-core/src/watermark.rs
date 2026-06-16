use crate::hash::{average_hash, psnr, sha256_file};
use crate::payload::{majority_decode, repeat_bits, WatermarkPayload, PAYLOAD_BITS};
use crate::transform::{
    dct_8, embed_bit_svd, extract_bit_svd, forward_haar_1_level, idct_8, inverse_haar_1_level,
    keyed_blocks, read_block, write_block,
};
use anyhow::{bail, Context, Result};
use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, RgbaImage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use uuid::Uuid;

pub const ALGORITHM_VERSION: &str = "dwt-dct-svd-qim-v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum AlgorithmProfile {
    Fidelity,
    Balanced,
    Strong,
}

impl AlgorithmProfile {
    pub fn repetitions(self) -> usize {
        match self {
            Self::Fidelity => 3,
            Self::Balanced => 5,
            Self::Strong => 7,
        }
    }

    pub fn default_strength(self) -> f32 {
        match self {
            Self::Fidelity => 8.0,
            Self::Balanced => 14.0,
            Self::Strong => 22.0,
        }
    }

    pub fn all() -> [Self; 3] {
        [Self::Balanced, Self::Strong, Self::Fidelity]
    }
}

impl std::fmt::Display for AlgorithmProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fidelity => write!(f, "fidelity"),
            Self::Balanced => write!(f, "balanced"),
            Self::Strong => write!(f, "strong"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum OutputFormat {
    Preserve,
    Jpeg,
    Png,
    Webp,
}

impl OutputFormat {
    pub fn extension(self, input: &Path) -> &'static str {
        match self {
            Self::Preserve => match input
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("png")
                .to_ascii_lowercase()
                .as_str()
            {
                "jpg" | "jpeg" => "jpg",
                "webp" => "webp",
                _ => "png",
            },
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbedOptions {
    pub profile: AlgorithmProfile,
    pub strength: Option<f32>,
    pub output_format: OutputFormat,
    pub quality: u8,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            profile: AlgorithmProfile::Balanced,
            strength: None,
            output_format: OutputFormat::Preserve,
            quality: 92,
        }
    }
}

impl EmbedOptions {
    pub fn effective_strength(&self) -> f32 {
        self.strength
            .unwrap_or_else(|| self.profile.default_strength())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedFile {
    pub input_path: PathBuf,
    pub output_path: PathBuf,
    pub watermark_id: Uuid,
    pub input_sha256: String,
    pub output_sha256: String,
    pub perceptual_hash: String,
    pub psnr: f32,
    pub algorithm_version: String,
    pub profile: AlgorithmProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EccStatus {
    Valid,
    Corrected,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub input_path: PathBuf,
    pub watermark_id: Option<Uuid>,
    pub confidence: f32,
    pub ecc_status: EccStatus,
    pub detected_profile: Option<AlgorithmProfile>,
    pub diagnostics: Vec<String>,
}

pub fn embed_file(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    key: &[u8],
    options: &EmbedOptions,
) -> Result<EmbeddedFile> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();
    let original =
        image::open(input_path).with_context(|| format!("decode {}", input_path.display()))?;
    let input_sha256 = sha256_file(input_path)?;
    let payload = WatermarkPayload::generate();
    let watermarked = embed_dynamic_image(&original, &payload, key, options)?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    save_image(&watermarked, output_path, input_path, options)?;

    let output_sha256 = sha256_file(output_path)?;
    let perceptual_hash = average_hash(&watermarked);
    let quality_psnr = psnr(&original, &watermarked);

    Ok(EmbeddedFile {
        input_path: input_path.to_path_buf(),
        output_path: output_path.to_path_buf(),
        watermark_id: payload.watermark_id,
        input_sha256,
        output_sha256,
        perceptual_hash,
        psnr: quality_psnr,
        algorithm_version: ALGORITHM_VERSION.to_string(),
        profile: options.profile,
    })
}

pub fn extract_file(input_path: impl AsRef<Path>, key: &[u8]) -> Result<ExtractResult> {
    let input_path = input_path.as_ref();
    let image =
        image::open(input_path).with_context(|| format!("decode {}", input_path.display()))?;
    let mut best = None;
    let mut diagnostics = Vec::new();

    for profile in AlgorithmProfile::all() {
        let attempt = match extract_dynamic_image_with_profile(&image, key, profile) {
            Ok(attempt) => attempt,
            Err(err) => {
                diagnostics.push(format!("profile {profile} skipped: {err:#}"));
                continue;
            }
        };
        if attempt.ecc_status == EccStatus::Valid || attempt.ecc_status == EccStatus::Corrected {
            return Ok(ExtractResult {
                input_path: input_path.to_path_buf(),
                watermark_id: attempt.payload.map(|payload| payload.watermark_id),
                confidence: attempt.confidence,
                ecc_status: attempt.ecc_status,
                detected_profile: Some(profile),
                diagnostics: attempt.diagnostics,
            });
        }
        if best
            .as_ref()
            .map(|current: &ExtractionAttempt| attempt.confidence > current.confidence)
            .unwrap_or(true)
        {
            best = Some(attempt);
        }
    }

    let Some(mut best) = best else {
        return Ok(ExtractResult {
            input_path: input_path.to_path_buf(),
            watermark_id: None,
            confidence: 0.0,
            ecc_status: EccStatus::Failed,
            detected_profile: None,
            diagnostics,
        });
    };
    best.diagnostics.extend(diagnostics);
    Ok(ExtractResult {
        input_path: input_path.to_path_buf(),
        watermark_id: None,
        confidence: best.confidence,
        ecc_status: EccStatus::Failed,
        detected_profile: None,
        diagnostics: best.diagnostics,
    })
}

pub fn verify_file(
    input_path: impl AsRef<Path>,
    watermark_id: Uuid,
    key: &[u8],
) -> Result<ExtractResult> {
    let result = extract_file(input_path, key)?;
    if result.watermark_id == Some(watermark_id) {
        Ok(result)
    } else {
        bail!(
            "watermark mismatch: expected {}, detected {:?}",
            watermark_id,
            result.watermark_id
        );
    }
}

pub fn embed_dynamic_image(
    image: &DynamicImage,
    payload: &WatermarkPayload,
    key: &[u8],
    options: &EmbedOptions,
) -> Result<DynamicImage> {
    let mut rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let transform_width = even_floor(width as usize);
    let transform_height = even_floor(height as usize);
    if transform_width < 128 || transform_height < 128 {
        bail!("image is too small for robust watermarking; minimum recommended size is 128x128");
    }

    let mut planes = extract_ycbcr(&rgba, transform_width, transform_height);
    forward_haar_1_level(&mut planes.y, transform_width, transform_height);

    let encoded_bits = repeat_bits(&payload.to_bits(), options.profile.repetitions());
    let blocks = keyed_blocks(transform_width, transform_height, key);
    if blocks.len() < encoded_bits.len() {
        bail!(
            "image capacity is {} bits, but watermark requires {} bits",
            blocks.len(),
            encoded_bits.len()
        );
    }

    let strength = options.effective_strength();
    for (bit, block_ref) in encoded_bits.iter().zip(blocks.iter()) {
        let spatial = read_block(&planes.y, transform_width, *block_ref);
        let dct = dct_8(spatial);
        let embedded = embed_bit_svd(dct, *bit, strength);
        let restored = idct_8(embedded);
        write_block(&mut planes.y, transform_width, *block_ref, restored);
    }

    inverse_haar_1_level(&mut planes.y, transform_width, transform_height);
    apply_ycbcr(&mut rgba, &planes, transform_width, transform_height);
    Ok(DynamicImage::ImageRgba8(rgba))
}

struct ExtractionAttempt {
    payload: Option<WatermarkPayload>,
    confidence: f32,
    ecc_status: EccStatus,
    diagnostics: Vec<String>,
}

fn extract_dynamic_image_with_profile(
    image: &DynamicImage,
    key: &[u8],
    profile: AlgorithmProfile,
) -> Result<ExtractionAttempt> {
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    let transform_width = even_floor(width as usize);
    let transform_height = even_floor(height as usize);
    if transform_width < 128 || transform_height < 128 {
        bail!("image is too small for robust watermark extraction");
    }

    let mut planes = extract_ycbcr(&rgba, transform_width, transform_height);
    forward_haar_1_level(&mut planes.y, transform_width, transform_height);

    let required_bits = PAYLOAD_BITS * profile.repetitions();
    let blocks = keyed_blocks(transform_width, transform_height, key);
    if blocks.len() < required_bits {
        bail!(
            "image capacity is {} bits, but profile requires {} bits",
            blocks.len(),
            required_bits
        );
    }

    let strength = profile.default_strength();
    let mut extracted = Vec::with_capacity(required_bits);
    for block_ref in blocks.iter().take(required_bits) {
        let spatial = read_block(&planes.y, transform_width, *block_ref);
        let dct = dct_8(spatial);
        extracted.push(extract_bit_svd(dct, strength));
    }

    let (decoded_bits, confidence) = majority_decode(&extracted, profile.repetitions());
    match WatermarkPayload::from_bits(&decoded_bits) {
        Ok(payload) => {
            let ecc_status = if confidence >= 0.999 {
                EccStatus::Valid
            } else {
                EccStatus::Corrected
            };
            Ok(ExtractionAttempt {
                payload: Some(payload),
                confidence,
                ecc_status,
                diagnostics: vec![format!("profile {profile} checksum passed")],
            })
        }
        Err(err) => Ok(ExtractionAttempt {
            payload: None,
            confidence,
            ecc_status: EccStatus::Failed,
            diagnostics: vec![format!(
                "profile {profile} failed payload validation: {err}"
            )],
        }),
    }
}

struct YCbCrPlanes {
    y: Vec<f32>,
    cb: Vec<f32>,
    cr: Vec<f32>,
}

fn extract_ycbcr(image: &RgbaImage, width: usize, height: usize) -> YCbCrPlanes {
    let mut y_plane = Vec::with_capacity(width * height);
    let mut cb_plane = Vec::with_capacity(width * height);
    let mut cr_plane = Vec::with_capacity(width * height);

    for row in 0..height {
        for col in 0..width {
            let pixel = image.get_pixel(col as u32, row as u32);
            let r = f32::from(pixel[0]);
            let g = f32::from(pixel[1]);
            let b = f32::from(pixel[2]);
            let y = 0.299 * r + 0.587 * g + 0.114 * b;
            let cb = 128.0 - 0.168_736 * r - 0.331_264 * g + 0.5 * b;
            let cr = 128.0 + 0.5 * r - 0.418_688 * g - 0.081_312 * b;
            y_plane.push(y);
            cb_plane.push(cb);
            cr_plane.push(cr);
        }
    }

    YCbCrPlanes {
        y: y_plane,
        cb: cb_plane,
        cr: cr_plane,
    }
}

fn apply_ycbcr(image: &mut RgbaImage, planes: &YCbCrPlanes, width: usize, height: usize) {
    for row in 0..height {
        for col in 0..width {
            let idx = row * width + col;
            let y = planes.y[idx].clamp(0.0, 255.0);
            let cb = planes.cb[idx] - 128.0;
            let cr = planes.cr[idx] - 128.0;
            let r = y + 1.402 * cr;
            let g = y - 0.344_136 * cb - 0.714_136 * cr;
            let b = y + 1.772 * cb;
            let pixel = image.get_pixel_mut(col as u32, row as u32);
            pixel[0] = clamp_u8(r);
            pixel[1] = clamp_u8(g);
            pixel[2] = clamp_u8(b);
        }
    }
}

fn save_image(
    image: &DynamicImage,
    output_path: &Path,
    input_path: &Path,
    options: &EmbedOptions,
) -> Result<()> {
    match options.output_format {
        OutputFormat::Jpeg => save_jpeg(image, output_path, options.quality),
        OutputFormat::Png => image
            .save_with_format(output_path, ImageFormat::Png)
            .with_context(|| format!("write {}", output_path.display())),
        OutputFormat::Webp => image
            .save_with_format(output_path, ImageFormat::WebP)
            .with_context(|| format!("write {}", output_path.display())),
        OutputFormat::Preserve => {
            let format = input_path
                .extension()
                .and_then(|ext| ext.to_str())
                .and_then(ImageFormat::from_extension)
                .unwrap_or(ImageFormat::Png);
            if matches!(format, ImageFormat::Jpeg) {
                save_jpeg(image, output_path, options.quality)
            } else {
                image
                    .save_with_format(output_path, format)
                    .with_context(|| format!("write {}", output_path.display()))
            }
        }
    }
}

fn save_jpeg(image: &DynamicImage, output_path: &Path, quality: u8) -> Result<()> {
    let file =
        File::create(output_path).with_context(|| format!("create {}", output_path.display()))?;
    let mut writer = BufWriter::new(file);
    let mut encoder = JpegEncoder::new_with_quality(&mut writer, quality);
    encoder
        .encode_image(&image.to_rgb8())
        .with_context(|| format!("write {}", output_path.display()))
}

fn even_floor(value: usize) -> usize {
    value - (value % 2)
}

fn clamp_u8(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Rgba};

    #[test]
    fn image_watermark_round_trip() {
        let img: RgbaImage = ImageBuffer::from_fn(384, 384, |x, y| {
            let v = ((x + y) % 255) as u8;
            Rgba([v, 255 - v, (x % 255) as u8, 255])
        });
        let image = DynamicImage::ImageRgba8(img);
        let payload = WatermarkPayload::generate();
        let key = b"test-key";
        let options = EmbedOptions::default();
        let watermarked = embed_dynamic_image(&image, &payload, key, &options).unwrap();
        let attempt =
            extract_dynamic_image_with_profile(&watermarked, key, AlgorithmProfile::Balanced)
                .unwrap();
        assert_eq!(attempt.payload.unwrap().watermark_id, payload.watermark_id);
        assert!(attempt.confidence > 0.9);
    }
}
