use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use symphonia::core::codecs::audio::AudioDecoderOptions;
use symphonia::core::formats::probe::Hint;
use symphonia::core::formats::{FormatOptions, TrackType};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;

/// Decoded audio: interleaved integer PCM at the source's original bit depth.
pub struct AudioData {
    pub sample_rate: u32,
    pub channels: u16,
    pub bits_per_sample: u32,
    /// Interleaved (L R L R ...) samples, right-aligned: each i32 holds one
    /// sample whose value fits in `bits_per_sample` bits.
    pub samples: Vec<i32>,
}

pub fn decode_flac(path: &Path) -> Result<AudioData> {
    let file = File::open(path).context("cannot open file")?;
    let mss = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());

    let mut hint = Hint::new();
    hint.with_extension("flac");

    let mut reader = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .context("not a recognized FLAC file")?;

    let track = reader
        .default_track(TrackType::Audio)
        .ok_or_else(|| anyhow!("no audio track found"))?;
    let track_id = track.id;

    // Clone the codec parameters so we stop borrowing `track` (and through it
    // `reader`) — the packet loop below needs `reader` mutably.
    let params = track
        .codec_params
        .as_ref()
        .and_then(|p| p.audio())
        .ok_or_else(|| anyhow!("missing audio codec parameters"))?
        .clone();

    let sample_rate = params
        .sample_rate
        .ok_or_else(|| anyhow!("unknown sample rate"))?;
    let bits_per_sample = params
        .bits_per_sample
        .ok_or_else(|| anyhow!("unknown bit depth"))?;
    let channels = params
        .channels
        .as_ref()
        .ok_or_else(|| anyhow!("unknown channel layout"))?
        .count() as u16;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(&params, &AudioDecoderOptions::default())?;

    // The decoder emits every sample left-aligned in 32 bits (full scale);
    // shifting right restores the original `bits_per_sample`-sized values.
    let shift = 32 - bits_per_sample;

    let mut samples: Vec<i32> = Vec::new();
    let mut packet_samples: Vec<i32> = Vec::new();
    while let Some(packet) = reader.next_packet()? {
        if packet.track_id != track_id {
            continue;
        }
        let decoded = decoder.decode(&packet)?;
        decoded.copy_to_vec_interleaved(&mut packet_samples);
        samples.extend(packet_samples.iter().map(|s| s >> shift));
    }

    Ok(AudioData {
        sample_rate,
        channels,
        bits_per_sample,
        samples,
    })
}
