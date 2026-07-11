use std::path::PathBuf;

use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "spinnable",
    version,
    about = "Convert FLAC files in a music library to AIFF/WAV"
)]
pub struct Cli {
    /// Root directory to scan recursively for .flac files
    pub root: PathBuf,

    /// List what would be converted without touching anything
    #[arg(long)]
    pub dry_run: bool,

    /// Output format (AIFF has better metadata support on Pioneer gear)
    #[arg(long, value_enum, default_value_t = OutputFormat::Aiff)]
    pub format: OutputFormat,

    /// Resample ceiling in Hz (the XDJ-RX2 plays at most 48kHz)
    #[arg(long, default_value_t = 48_000)]
    pub max_rate: u32,

    /// Delete the source FLAC after successful conversion (default: keep)
    #[arg(long)]
    pub delete_original: bool,

    /// Only convert files directly in the root folder (default: recurse into subfolders)
    #[arg(long)]
    pub shallow: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputFormat {
    Aiff,
    Wav,
}

impl OutputFormat {
    pub fn extension(self) -> &'static str {
        match self {
            OutputFormat::Aiff => "aiff",
            OutputFormat::Wav => "wav",
        }
    }
}
