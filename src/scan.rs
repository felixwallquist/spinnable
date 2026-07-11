use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// One source FLAC and where its converted file would go.
pub struct PlannedConversion {
    pub source: PathBuf,
    pub output: PathBuf,
    pub output_exists: bool,
}

/// Find every .flac under `root` and plan an output path next to each
/// source file (non-destructive: the source is never touched).
/// Recurses into subfolders unless `shallow` is set.
/// Unreadable directory entries are collected as warnings, not fatal errors.
pub fn scan(
    root: &Path,
    output_extension: &str,
    shallow: bool,
) -> (Vec<PlannedConversion>, Vec<String>) {
    let mut planned = Vec::new();
    let mut warnings = Vec::new();

    let mut walker = WalkDir::new(root);
    if shallow {
        // depth 1 = the root's direct children; the root itself is depth 0
        walker = walker.max_depth(1);
    }

    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                warnings.push(format!("skipping unreadable entry: {err}"));
                continue;
            }
        };

        if !entry.file_type().is_file() || !is_flac(entry.path()) {
            continue;
        }

        let source = entry.into_path();
        let output = source.with_extension(output_extension);
        let output_exists = output.exists();
        planned.push(PlannedConversion {
            source,
            output,
            output_exists,
        });
    }

    (planned, warnings)
}

fn is_flac(path: &Path) -> bool {
    path.extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("flac"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_flac_case_insensitively() {
        assert!(is_flac(Path::new("/music/track.flac")));
        assert!(is_flac(Path::new("/music/track.FLAC")));
        assert!(!is_flac(Path::new("/music/track.mp3")));
        assert!(!is_flac(Path::new("/music/flac"))); // no extension at all
    }
}
