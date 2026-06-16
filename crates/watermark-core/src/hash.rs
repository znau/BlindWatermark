use anyhow::{Context, Result};
use image::{imageops::FilterType, DynamicImage, GenericImageView};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub fn sha256_file(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let mut file = File::open(path).with_context(|| format!("open {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn average_hash(image: &DynamicImage) -> String {
    let gray = image.resize_exact(8, 8, FilterType::Triangle).to_luma8();
    let pixels = gray.pixels().map(|p| p[0] as u32).collect::<Vec<_>>();
    let mean = pixels.iter().sum::<u32>() as f32 / pixels.len() as f32;
    let mut bits = 0_u64;
    for (idx, value) in pixels.iter().enumerate() {
        if (*value as f32) >= mean {
            bits |= 1_u64 << (63 - idx);
        }
    }
    format!("{bits:016x}")
}

pub fn psnr(original: &DynamicImage, watermarked: &DynamicImage) -> f32 {
    let (width, height) = original.dimensions();
    if watermarked.dimensions() != (width, height) {
        return 0.0;
    }
    let a = original.to_rgb8();
    let b = watermarked.to_rgb8();
    let mut mse = 0.0_f64;
    let mut count = 0_u64;
    for (pa, pb) in a.pixels().zip(b.pixels()) {
        for channel in 0..3 {
            let delta = f64::from(pa[channel]) - f64::from(pb[channel]);
            mse += delta * delta;
            count += 1;
        }
    }
    if mse == 0.0 {
        return f32::INFINITY;
    }
    mse /= count as f64;
    (20.0 * (255.0_f64 / mse.sqrt()).log10()) as f32
}
