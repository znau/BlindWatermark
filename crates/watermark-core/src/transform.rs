use nalgebra::{SMatrix, SVD};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use sha2::{Digest, Sha256};

pub const BLOCK: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subband {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockRef {
    pub subband: Subband,
    pub x: usize,
    pub y: usize,
}

pub fn forward_haar_1_level(data: &mut [f32], width: usize, height: usize) {
    let mut temp = vec![0.0_f32; width.max(height)];

    for y in 0..height {
        let row_start = y * width;
        for x in 0..(width / 2) {
            let a = data[row_start + 2 * x];
            let b = data[row_start + 2 * x + 1];
            temp[x] = (a + b) * 0.5;
            temp[x + width / 2] = (a - b) * 0.5;
        }
        data[row_start..row_start + width].copy_from_slice(&temp[..width]);
    }

    for x in 0..width {
        for y in 0..(height / 2) {
            let a = data[(2 * y) * width + x];
            let b = data[(2 * y + 1) * width + x];
            temp[y] = (a + b) * 0.5;
            temp[y + height / 2] = (a - b) * 0.5;
        }
        for y in 0..height {
            data[y * width + x] = temp[y];
        }
    }
}

pub fn inverse_haar_1_level(data: &mut [f32], width: usize, height: usize) {
    let mut temp = vec![0.0_f32; width.max(height)];

    for x in 0..width {
        for y in 0..(height / 2) {
            let avg = data[y * width + x];
            let detail = data[(y + height / 2) * width + x];
            temp[2 * y] = avg + detail;
            temp[2 * y + 1] = avg - detail;
        }
        for y in 0..height {
            data[y * width + x] = temp[y];
        }
    }

    for y in 0..height {
        let row_start = y * width;
        for x in 0..(width / 2) {
            let avg = data[row_start + x];
            let detail = data[row_start + x + width / 2];
            temp[2 * x] = avg + detail;
            temp[2 * x + 1] = avg - detail;
        }
        data[row_start..row_start + width].copy_from_slice(&temp[..width]);
    }
}

pub fn candidate_blocks(width: usize, height: usize) -> Vec<BlockRef> {
    let half_w = width / 2;
    let half_h = height / 2;
    let blocks_x = half_w / BLOCK;
    let blocks_y = half_h / BLOCK;
    let mut blocks = Vec::with_capacity(blocks_x * blocks_y * 2);

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            blocks.push(BlockRef {
                subband: Subband::Horizontal,
                x: half_w + bx * BLOCK,
                y: by * BLOCK,
            });
            blocks.push(BlockRef {
                subband: Subband::Vertical,
                x: bx * BLOCK,
                y: half_h + by * BLOCK,
            });
        }
    }

    blocks
}

pub fn keyed_blocks(width: usize, height: usize, key: &[u8]) -> Vec<BlockRef> {
    let mut blocks = candidate_blocks(width, height);
    let seed = derive_seed(key, width, height);
    let mut rng = ChaCha20Rng::from_seed(seed);
    blocks.shuffle(&mut rng);
    blocks
}

pub fn read_block(data: &[f32], width: usize, block_ref: BlockRef) -> [[f32; BLOCK]; BLOCK] {
    let mut block = [[0.0_f32; BLOCK]; BLOCK];
    for y in 0..BLOCK {
        for x in 0..BLOCK {
            block[y][x] = data[(block_ref.y + y) * width + block_ref.x + x];
        }
    }
    block
}

pub fn write_block(
    data: &mut [f32],
    width: usize,
    block_ref: BlockRef,
    block: [[f32; BLOCK]; BLOCK],
) {
    for y in 0..BLOCK {
        for x in 0..BLOCK {
            data[(block_ref.y + y) * width + block_ref.x + x] = block[y][x];
        }
    }
}

pub fn dct_8(block: [[f32; BLOCK]; BLOCK]) -> [[f32; BLOCK]; BLOCK] {
    let mut out = [[0.0_f32; BLOCK]; BLOCK];
    for u in 0..BLOCK {
        for v in 0..BLOCK {
            let mut sum = 0.0_f32;
            for x in 0..BLOCK {
                for y in 0..BLOCK {
                    sum += block[x][y] * basis(x, u) * basis(y, v);
                }
            }
            out[u][v] = alpha(u) * alpha(v) * sum;
        }
    }
    out
}

pub fn idct_8(block: [[f32; BLOCK]; BLOCK]) -> [[f32; BLOCK]; BLOCK] {
    let mut out = [[0.0_f32; BLOCK]; BLOCK];
    for x in 0..BLOCK {
        for y in 0..BLOCK {
            let mut sum = 0.0_f32;
            for u in 0..BLOCK {
                for v in 0..BLOCK {
                    sum += alpha(u) * alpha(v) * block[u][v] * basis(x, u) * basis(y, v);
                }
            }
            out[x][y] = sum;
        }
    }
    out
}

pub fn embed_bit_svd(
    dct_block: [[f32; BLOCK]; BLOCK],
    bit: bool,
    step: f32,
) -> [[f32; BLOCK]; BLOCK] {
    let matrix = to_matrix(dct_block);
    let svd = SVD::new(matrix, true, true);
    let (Some(u), Some(v_t)) = (svd.u, svd.v_t) else {
        return dct_block;
    };

    let mut sigma = SMatrix::<f32, BLOCK, BLOCK>::zeros();
    let mut singular_values = svd.singular_values;
    singular_values[0] = quantize_for_bit(singular_values[0], bit, step);
    for i in 0..BLOCK {
        sigma[(i, i)] = singular_values[i];
    }

    from_matrix(u * sigma * v_t)
}

pub fn extract_bit_svd(dct_block: [[f32; BLOCK]; BLOCK], step: f32) -> bool {
    let matrix = to_matrix(dct_block);
    let svd = SVD::new(matrix, false, false);
    let s0 = svd.singular_values[0].abs();
    let frac = (s0 / step).fract();
    frac >= 0.5
}

fn quantize_for_bit(value: f32, bit: bool, step: f32) -> f32 {
    let target = if bit { 0.75_f32 } else { 0.25_f32 };
    let base = (value / step).floor();
    let mut best = (base + target) * step;
    let mut best_delta = (best - value).abs();

    for offset in [-1.0_f32, 1.0_f32] {
        let candidate_base = (base + offset).max(0.0);
        let candidate = (candidate_base + target) * step;
        let delta = (candidate - value).abs();
        if delta < best_delta {
            best = candidate;
            best_delta = delta;
        }
    }

    best.max(0.0)
}

fn derive_seed(key: &[u8], width: usize, height: usize) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"blind-watermark-blocks-v1");
    hasher.update(key);
    hasher.update((width as u64).to_be_bytes());
    hasher.update((height as u64).to_be_bytes());
    hasher.finalize().into()
}

fn alpha(index: usize) -> f32 {
    if index == 0 {
        (1.0_f32 / 8.0_f32).sqrt()
    } else {
        (2.0_f32 / 8.0_f32).sqrt()
    }
}

fn basis(pixel: usize, freq: usize) -> f32 {
    let numerator = ((2 * pixel + 1) as f32) * (freq as f32) * std::f32::consts::PI;
    (numerator / 16.0).cos()
}

fn to_matrix(block: [[f32; BLOCK]; BLOCK]) -> SMatrix<f32, BLOCK, BLOCK> {
    SMatrix::<f32, BLOCK, BLOCK>::from_fn(|row, col| block[row][col])
}

fn from_matrix(matrix: SMatrix<f32, BLOCK, BLOCK>) -> [[f32; BLOCK]; BLOCK] {
    let mut out = [[0.0_f32; BLOCK]; BLOCK];
    for row in 0..BLOCK {
        for col in 0..BLOCK {
            out[row][col] = matrix[(row, col)];
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn haar_round_trip() {
        let width = 16;
        let height = 16;
        let mut data = (0..width * height).map(|v| v as f32).collect::<Vec<_>>();
        let original = data.clone();
        forward_haar_1_level(&mut data, width, height);
        inverse_haar_1_level(&mut data, width, height);
        for (actual, expected) in data.iter().zip(original.iter()) {
            assert!((actual - expected).abs() < 0.001);
        }
    }

    #[test]
    fn dct_round_trip() {
        let mut block = [[0.0_f32; BLOCK]; BLOCK];
        for y in 0..BLOCK {
            for x in 0..BLOCK {
                block[y][x] = (x * y) as f32;
            }
        }
        let restored = idct_8(dct_8(block));
        for y in 0..BLOCK {
            for x in 0..BLOCK {
                assert!((restored[y][x] - block[y][x]).abs() < 0.01);
            }
        }
    }

    #[test]
    fn svd_bit_round_trip() {
        let mut block = [[0.0_f32; BLOCK]; BLOCK];
        for y in 0..BLOCK {
            for x in 0..BLOCK {
                block[y][x] = ((x + 1) * (y + 2)) as f32;
            }
        }
        let embedded = embed_bit_svd(block, true, 12.0);
        assert!(extract_bit_svd(embedded, 12.0));
    }
}
