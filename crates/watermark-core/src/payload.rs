use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

pub const PAYLOAD_VERSION: u8 = 1;
pub const WATERMARK_ID_BYTES: usize = 16;
pub const CHECKSUM_BYTES: usize = 4;
pub const PAYLOAD_BYTES: usize = 1 + WATERMARK_ID_BYTES + CHECKSUM_BYTES;
pub const PAYLOAD_BITS: usize = PAYLOAD_BYTES * 8;

#[derive(Debug, Error)]
pub enum PayloadError {
    #[error("payload must contain {expected} bytes, got {actual}")]
    InvalidLength { expected: usize, actual: usize },
    #[error("unsupported payload version {0}")]
    UnsupportedVersion(u8),
    #[error("payload checksum mismatch")]
    ChecksumMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WatermarkPayload {
    pub version: u8,
    pub watermark_id: Uuid,
    pub checksum: u32,
}

impl WatermarkPayload {
    pub fn new(watermark_id: Uuid) -> Self {
        let checksum = checksum_for(PAYLOAD_VERSION, watermark_id.as_bytes());
        Self {
            version: PAYLOAD_VERSION,
            watermark_id,
            checksum,
        }
    }

    pub fn generate() -> Self {
        Self::new(Uuid::new_v4())
    }

    pub fn to_bytes(&self) -> [u8; PAYLOAD_BYTES] {
        let mut out = [0_u8; PAYLOAD_BYTES];
        out[0] = self.version;
        out[1..17].copy_from_slice(self.watermark_id.as_bytes());
        out[17..21].copy_from_slice(&self.checksum.to_be_bytes());
        out
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PayloadError> {
        if bytes.len() != PAYLOAD_BYTES {
            return Err(PayloadError::InvalidLength {
                expected: PAYLOAD_BYTES,
                actual: bytes.len(),
            });
        }
        let version = bytes[0];
        if version != PAYLOAD_VERSION {
            return Err(PayloadError::UnsupportedVersion(version));
        }

        let mut id = [0_u8; WATERMARK_ID_BYTES];
        id.copy_from_slice(&bytes[1..17]);
        let watermark_id = Uuid::from_bytes(id);

        let mut checksum_bytes = [0_u8; CHECKSUM_BYTES];
        checksum_bytes.copy_from_slice(&bytes[17..21]);
        let checksum = u32::from_be_bytes(checksum_bytes);
        if checksum != checksum_for(version, watermark_id.as_bytes()) {
            return Err(PayloadError::ChecksumMismatch);
        }

        Ok(Self {
            version,
            watermark_id,
            checksum,
        })
    }

    pub fn to_bits(&self) -> Vec<bool> {
        bytes_to_bits(&self.to_bytes())
    }

    pub fn from_bits(bits: &[bool]) -> Result<Self, PayloadError> {
        let bytes = bits_to_bytes(bits);
        Self::from_bytes(&bytes)
    }
}

pub fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut out = Vec::with_capacity(bytes.len() * 8);
    for byte in bytes {
        for bit in (0..8).rev() {
            out.push(((byte >> bit) & 1) == 1);
        }
    }
    out
}

pub fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    let mut out = Vec::with_capacity((bits.len() + 7) / 8);
    for chunk in bits.chunks(8) {
        let mut byte = 0_u8;
        for (idx, bit) in chunk.iter().enumerate() {
            if *bit {
                byte |= 1 << (7 - idx);
            }
        }
        out.push(byte);
    }
    out
}

pub fn repeat_bits(bits: &[bool], repetitions: usize) -> Vec<bool> {
    let mut out = Vec::with_capacity(bits.len() * repetitions);
    for bit in bits {
        for _ in 0..repetitions {
            out.push(*bit);
        }
    }
    out
}

pub fn majority_decode(bits: &[bool], repetitions: usize) -> (Vec<bool>, f32) {
    let mut out = Vec::with_capacity(bits.len() / repetitions);
    let mut confidence_sum = 0.0_f32;
    let mut groups = 0_usize;

    for chunk in bits.chunks(repetitions) {
        if chunk.len() != repetitions {
            continue;
        }
        let ones = chunk.iter().filter(|bit| **bit).count();
        let zeros = repetitions - ones;
        out.push(ones >= zeros);
        confidence_sum += (ones.max(zeros) as f32) / (repetitions as f32);
        groups += 1;
    }

    let confidence = if groups == 0 {
        0.0
    } else {
        confidence_sum / (groups as f32)
    };
    (out, confidence)
}

fn checksum_for(version: u8, id: &[u8; WATERMARK_ID_BYTES]) -> u32 {
    let mut hasher = Hasher::new();
    hasher.update(&[version]);
    hasher.update(id);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_round_trip() {
        let payload = WatermarkPayload::generate();
        let bits = payload.to_bits();
        let decoded = WatermarkPayload::from_bits(&bits).unwrap();
        assert_eq!(payload, decoded);
    }

    #[test]
    fn repeated_bits_majority_decode() {
        let payload = WatermarkPayload::generate();
        let mut repeated = repeat_bits(&payload.to_bits(), 5);
        repeated[0] = !repeated[0];
        repeated[7] = !repeated[7];
        let (decoded_bits, confidence) = majority_decode(&repeated, 5);
        let decoded = WatermarkPayload::from_bits(&decoded_bits).unwrap();
        assert_eq!(payload.watermark_id, decoded.watermark_id);
        assert!(confidence > 0.8);
    }
}
