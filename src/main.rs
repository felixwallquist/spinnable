mod cli;
mod convert;
mod decode;
mod encode;
mod scan;

use anyhow::{bail, Context, Result};
use clap::Parser;

use crate::cli::Cli;

fn main() -> Result<()> {
    let args = Cli::parse();

    let root = args
        .root
        .canonicalize()
        .with_context(|| format!("cannot access root path {}", args.root.display()))?;
    if !root.is_dir() {
        bail!("{} is not a directory", root.display());
    }

    let (planned, warnings) = scan::scan(&root, args.format.extension(), args.shallow);

    for warning in &warnings {
        eprintln!("warning: {warning}");
    }

    if planned.is_empty() {
        println!("No .flac files found under {}", root.display());
        return Ok(());
    }

    let (skipped, to_convert): (Vec<_>, Vec<_>) =
        planned.iter().partition(|p| p.output_exists);

    if args.dry_run {
        for plan in &to_convert {
            println!(
                "convert  {}  ->  {}",
                plan.source.display(),
                plan.output.display()
            );
        }
        for plan in &skipped {
            println!("skip     {}  (output already exists)", plan.source.display());
        }
        println!();
        println!(
            "{} to convert, {} skipped (already converted), {} warnings",
            to_convert.len(),
            skipped.len(),
            warnings.len()
        );
        return Ok(());
    }

    let mut converted = 0usize;
    let mut failures: Vec<String> = Vec::new();
    for plan in &to_convert {
        match convert::convert_file(plan, args.format, args.max_rate) {
            Ok(()) => {
                converted += 1;
                println!("converted  {}", plan.output.display());
            }
            Err(err) => {
                // One bad file must never abort the run: record and continue.
                // {:#} prints the whole context chain on one line.
                failures.push(format!("{}: {err:#}", plan.source.display()));
            }
        }
    }

    println!();
    println!(
        "{converted} converted, {} failed, {} skipped (already converted), {} warnings",
        failures.len(),
        skipped.len(),
        warnings.len()
    );
    for failure in &failures {
        eprintln!("failed: {failure}");
    }
    if !failures.is_empty() {
        bail!("{} file(s) failed to convert", failures.len());
    }

    Ok(())
}