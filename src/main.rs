mod cli;
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

    for plan in &to_convert {
        println!("convert  {}  ->  {}", plan.source.display(), plan.output.display());
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

    if !args.dry_run && !to_convert.is_empty() {
        println!("note: conversion is not implemented yet (Milestone 2) — no files were written");
    }

    Ok(())
}
