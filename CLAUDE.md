# spinnable

CLI tool (Rust) that converts FLAC files in a DJ music library to AIFF/WAV playable on the Pioneer XDJ-RX2. Ships as a fully self-contained static binary — no ffmpeg, no external runtime dependencies, ever.

The full roadmap and architecture live in @PLAN.md.

## Commands
- Build: `cargo build`
- Run: `cargo run -- <args>`
- Test: `cargo test`
- Lint: `cargo clippy -- -D warnings`
- Format: `cargo fmt`

## Hard constraints
- Output must be playable on the XDJ-RX2: max 48kHz sample rate, max 24-bit, integer PCM only (no float).
- Default output format is AIFF (better metadata support on Pioneer gear than WAV).
- The tool is non-destructive by default: never delete or overwrite source FLAC files unless `--delete-original` is passed.
- One bad input file must never abort the whole run — collect errors and report a summary.

## Conventions
- Each file conversion is a pure function returning `Result` — no shared mutable state across worker threads (keeps rayon parallelism trivial).
- Use `anyhow` for error propagation in the binary; add context to errors (`with_context`) so failure messages name the offending file.
- The developer is experienced (Kotlin backend) but new to Rust: when introducing a non-obvious Rust idiom (lifetimes, trait objects, borrow-checker workarounds), briefly explain the why, not just the what.
