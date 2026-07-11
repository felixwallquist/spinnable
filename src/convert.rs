use std::fs;

use anyhow::{ensure, Context, Result};

use crate::cli::OutputFormat;
use crate::decode;
use crate::encode;
use crate::scan::PlannedConversion;

/// Convert one FLAC end-to-end. A pure function of its inputs — no shared
/// state — so fanning out across threads with rayon stays trivial.
pub fn convert_file(plan: &PlannedConversion, format: OutputFormat, max_rate: u32) -> Result<()> {
    let audio = decode::decode_flac(&plan.source)
        .with_context(|| format!("decoding {}", plan.source.display()))?;

    // Milestone 3 adds resampling and bit-depth reduction; until then,
    // hi-res sources get a clean error instead of a broken output file.
    ensure!(
        audio.sample_rate <= max_rate,
        "sample rate {} Hz exceeds {} Hz (resampling lands in Milestone 3)",
        audio.sample_rate,
        max_rate
    );
    ensure!(
        audio.bits_per_sample <= 24,
        "{}-bit audio not yet supported (bit-depth conversion lands in Milestone 3)",
        audio.bits_per_sample
    );

    // Write to a temp name and rename into place on success: an interrupted
    // run never leaves a half-written file that a later run would "skip".
    let tmp = plan.output.with_extension("spinnable-tmp");
    let written = match format {
        OutputFormat::Aiff => encode::write_aiff(&tmp, &audio),
        OutputFormat::Wav => encode::write_wav(&tmp, &audio),
    }
    .with_context(|| format!("encoding {}", plan.output.display()));

    if let Err(err) = written {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }

    fs::rename(&tmp, &plan.output)
        .with_context(|| format!("moving into place: {}", plan.output.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use flacenc::component::BitRepr;
    use flacenc::error::Verify;

    use super::*;

    const RATE: u32 = 44_100;
    const CHANNELS: usize = 2;
    const BITS: usize = 16;

    /// A second of interleaved stereo: sine wave left, ramp right.
    fn test_samples() -> Vec<i32> {
        (0..RATE as usize)
            .flat_map(|t| {
                let sine = (8000.0 * (t as f64 * 0.05).sin()) as i32;
                let ramp = (t % 1000) as i32 - 500;
                [sine, ramp]
            })
            .collect()
    }

    fn write_test_flac(path: &Path, samples: &[i32]) {
        let mut config = flacenc::config::Encoder::default();
        // Must divide the frame count evenly: flacenc counts a short final
        // block in STREAMINFO's min_block_size (the FLAC spec says to exclude
        // it), and that min != max mismatch makes symphonia reject every frame.
        config.block_size = 4410;
        let config = config.into_verified().expect("valid encoder config");
        let source =
            flacenc::source::MemSource::from_samples(samples, CHANNELS, BITS, RATE as usize);
        let stream =
            flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
                .expect("flac encoding succeeds");
        let mut sink = flacenc::bitsink::ByteSink::new();
        stream.write(&mut sink).expect("flac serialization succeeds");
        fs::write(path, sink.as_slice()).expect("write flac fixture");
    }

    #[test]
    fn decode_roundtrips_exact_pcm() {
        let dir = tempfile::tempdir().unwrap();
        let flac = dir.path().join("test.flac");
        let samples = test_samples();
        write_test_flac(&flac, &samples);

        let audio = decode::decode_flac(&flac).unwrap();

        assert_eq!(audio.sample_rate, RATE);
        assert_eq!(audio.channels, CHANNELS as u16);
        assert_eq!(audio.bits_per_sample, BITS as u32);
        assert_eq!(audio.samples, samples, "decoded PCM must be bit-identical");
    }

    #[test]
    fn convert_writes_valid_aiff() {
        let dir = tempfile::tempdir().unwrap();
        let flac = dir.path().join("test.flac");
        let samples = test_samples();
        write_test_flac(&flac, &samples);

        let plan = PlannedConversion {
            source: flac,
            output: dir.path().join("test.aiff"),
            output_exists: false,
        };
        convert_file(&plan, OutputFormat::Aiff, 48_000).unwrap();

        let bytes = fs::read(&plan.output).unwrap();
        assert_eq!(&bytes[0..4], b"FORM");
        assert_eq!(&bytes[8..12], b"AIFF");
        assert_eq!(&bytes[12..16], b"COMM");
        // COMM body: channels(2) frames(4) bits(2) rate(10), starting at byte 20
        assert_eq!(&bytes[20..22], &(CHANNELS as i16).to_be_bytes());
        let frames = (samples.len() / CHANNELS) as u32;
        assert_eq!(&bytes[22..26], &frames.to_be_bytes());
        assert_eq!(&bytes[26..28], &(BITS as i16).to_be_bytes());
        // First PCM sample lives after SSND header: 38 + 8 + 8 = byte 54
        assert_eq!(&bytes[38..42], b"SSND");
        let first = i16::from_be_bytes([bytes[54], bytes[55]]) as i32;
        assert_eq!(first, samples[0]);
        // File length: 8-byte FORM header + reported FORM size
        let form_len = u32::from_be_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!(bytes.len(), 8 + form_len as usize);
    }

    #[test]
    fn convert_rejects_hi_res_until_milestone_3() {
        let dir = tempfile::tempdir().unwrap();
        let flac = dir.path().join("test.flac");
        write_test_flac(&flac, &test_samples());

        let plan = PlannedConversion {
            source: flac,
            output: dir.path().join("test.aiff"),
            output_exists: false,
        };
        // 22.05kHz ceiling makes our 44.1kHz fixture "hi-res"
        let err = convert_file(&plan, OutputFormat::Aiff, 22_050).unwrap_err();
        assert!(err.to_string().contains("exceeds"));
        assert!(!plan.output.exists(), "no output file on failure");
    }
}
