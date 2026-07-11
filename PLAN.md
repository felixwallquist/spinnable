# spinnable — Project Plan

## What it is
A CLI tool that recursively scans a DJ music library (e.g. a USB drive), finds FLAC files, and converts them to a format playable on the Pioneer XDJ-RX2 (which supports MP3, AAC, WAV, AIFF from USB — but not FLAC).

Written in Rust, compiled to a fully self-contained static binary (no ffmpeg, no runtime dependencies) so it can be shared with non-technical DJ friends.

## Hardware constraints (Pioneer XDJ-RX2)
- No FLAC or ALAC playback from USB (export mode).
- WAV/AIFF supported up to **48kHz / 24-bit**. Hi-res sources (88.2/96kHz) must be resampled down to 48kHz.
- Output must be integer PCM (no 32-bit float).
- Prefer **AIFF** over WAV as the default output: same PCM audio, but far better metadata (ID3 tags) support on Pioneer gear. WAV metadata handling is inconsistent.

## CLI shape
```
spinnable <ROOT_PATH>
  --dry-run            # list what would convert, touch nothing
  --format aiff|wav    # default: aiff
  --keep-original      # default behavior is to KEEP originals (non-destructive);
                       # provide --delete-original to remove flacs after successful conversion
  --max-rate 48000     # resample ceiling (default 48000 for the RX2)
```

Behavior rules:
- Non-destructive by default: write `track.aiff` next to `track.flac`, never delete unless explicitly flagged.
- Idempotent: skip files whose output already exists.
- Never abort the whole run on one bad file: collect errors, print a summary at the end ("142 converted, 3 failed: ...").

## Dependencies (crates)
- `clap` (derive feature) — CLI argument parsing
- `walkdir` — recursive directory traversal
- `symphonia` (flac feature) — FLAC decoding to PCM
- `rubato` — resampling for >48kHz sources
- `metaflac` — read FLAC tags (Vorbis comments)
- `id3` — write ID3 tags (supports AIFF natively)
- `anyhow` — error handling
- `rayon` — parallel conversion across files
- `indicatif` — progress bar

Note: no mature AIFF *writer* crate exists. AIFF is a simple container (FORM header + COMM + SSND chunks, big-endian PCM); write the encoder by hand (~100 lines). Quirk: AIFF stores sample rate as an 80-bit extended float. For WAV output, use `hound`.

## Module layout
```
src/
├── main.rs        # entry point: parse args, kick off pipeline
├── cli.rs         # clap definitions
├── scan.rs        # walkdir: find all .flac under the root
├── decode.rs      # symphonia: FLAC → raw PCM samples + source specs
├── resample.rs    # rubato: only invoked when sample rate > max-rate
├── encode.rs      # PCM → AIFF (or WAV via hound)
├── metadata.rs    # FLAC Vorbis comments → ID3 (title, artist, album, genre, BPM, key)
└── convert.rs     # orchestrates one file end-to-end; rayon fans out across files
```

Design principle: each file conversion in `convert.rs` is a pure function returning a `Result`. No shared mutable state across threads — this makes rayon parallelism a one-line change (`iter()` → `par_iter()`).

## Milestones (each one runnable)

### Milestone 1 — Scan + dry-run
- clap CLI with root path and `--dry-run`
- walkdir finds every `.flac` under root
- Print each found file with its planned output path
- Skip logic: note which outputs already exist
- Rust concepts covered: ownership basics, `PathBuf`, iterators

### Milestone 2 — Single-file conversion
- Decode one FLAC with symphonia to PCM
- Write PCM out as AIFF (hand-written encoder) or WAV (hound)
- **Acceptance test: the output file loads and plays on the XDJ-RX2 / in rekordbox**

### Milestone 3 — Resampling & bit depth
- Detect sources >48kHz → resample with rubato to 48kHz
- Detect 32-bit float / >24-bit sources → convert to 24-bit (or 16-bit) integer PCM
- Test with a 96kHz FLAC

### Milestone 4 — Metadata
- Read Vorbis comments from source FLAC (metaflac)
- Write ID3 tags to output (title, artist, album, genre, BPM, initial key)
- Verify tags appear correctly in rekordbox / on the deck

### Milestone 5 — Polish
- rayon parallel conversion across all files
- indicatif progress bar
- Skip-existing logic finalized
- Error summary at end of run instead of dying on first bad file
- `--delete-original` flag

## Distribution (later)
- `cargo build --release` per platform (macOS arm64/x86_64, Windows, Linux)
- Single static binary, no runtime dependencies — just send the file to friends
- Consider GitHub Releases with CI-built binaries
