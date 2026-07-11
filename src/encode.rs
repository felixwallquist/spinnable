use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::decode::AudioData;

/// Write `audio` as an AIFF file. No mature AIFF writer crate exists, so the
/// container is written by hand: a FORM wrapper holding a COMM chunk (format
/// description) and an SSND chunk (big-endian PCM sample data).
pub fn write_aiff(path: &Path, audio: &AudioData) -> Result<()> {
    let bytes_per_sample = audio.bits_per_sample.div_ceil(8) as usize;
    let data_len = audio.samples.len() * bytes_per_sample;
    let num_frames = audio.samples.len() / audio.channels as usize;

    let comm_len: u32 = 18;
    let ssnd_len: u32 = 8 + data_len as u32; // offset + block size + PCM data
    let pad = (data_len % 2) as u32; // chunks are padded to even length
    let form_len: u32 = 4 + (8 + comm_len) + (8 + ssnd_len) + pad;

    let file = File::create(path).context("cannot create output file")?;
    let mut w = BufWriter::new(file);

    w.write_all(b"FORM")?;
    w.write_all(&form_len.to_be_bytes())?;
    w.write_all(b"AIFF")?;

    w.write_all(b"COMM")?;
    w.write_all(&comm_len.to_be_bytes())?;
    w.write_all(&(audio.channels as i16).to_be_bytes())?;
    w.write_all(&(num_frames as u32).to_be_bytes())?;
    w.write_all(&(audio.bits_per_sample as i16).to_be_bytes())?;
    w.write_all(&extended_sample_rate(audio.sample_rate))?;

    w.write_all(b"SSND")?;
    w.write_all(&ssnd_len.to_be_bytes())?;
    w.write_all(&0u32.to_be_bytes())?; // offset into the data (unused)
    w.write_all(&0u32.to_be_bytes())?; // block alignment (unused)
    for &sample in &audio.samples {
        // Big-endian bytes of the i32; the sample occupies the low
        // `bytes_per_sample` bytes, i.e. the tail of the array.
        let bytes = sample.to_be_bytes();
        w.write_all(&bytes[4 - bytes_per_sample..])?;
    }
    if pad == 1 {
        w.write_all(&[0])?;
    }

    w.flush().context("failed writing AIFF data")?;
    Ok(())
}

/// AIFF stores the sample rate as an 80-bit IEEE 754 extended float:
/// 1 sign bit, 15 exponent bits (bias 16383), 64 mantissa bits with an
/// explicit leading 1. Sample rates are small positive integers, so this
/// only needs the integer case.
fn extended_sample_rate(rate: u32) -> [u8; 10] {
    let mut out = [0u8; 10];
    if rate == 0 {
        return out;
    }
    let top_bit = 31 - rate.leading_zeros();
    let exponent = 16383 + top_bit as u16;
    let mantissa = (rate as u64) << (63 - top_bit);
    out[..2].copy_from_slice(&exponent.to_be_bytes());
    out[2..].copy_from_slice(&mantissa.to_be_bytes());
    out
}

pub fn write_wav(path: &Path, audio: &AudioData) -> Result<()> {
    let spec = hound::WavSpec {
        channels: audio.channels,
        sample_rate: audio.sample_rate,
        bits_per_sample: audio.bits_per_sample as u16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer =
        hound::WavWriter::create(path, spec).context("cannot create output file")?;
    for &sample in &audio.samples {
        writer.write_sample(sample)?;
    }
    writer.finalize().context("failed writing WAV data")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extended_float_matches_known_sample_rates() {
        // Reference byte patterns from the AIFF-1.3 specification.
        assert_eq!(
            extended_sample_rate(44_100),
            [0x40, 0x0E, 0xAC, 0x44, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            extended_sample_rate(48_000),
            [0x40, 0x0E, 0xBB, 0x80, 0, 0, 0, 0, 0, 0]
        );
    }
}
