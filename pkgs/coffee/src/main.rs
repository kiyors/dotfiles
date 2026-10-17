//! `coffee` — a developer cleanup and maintenance utility.
//!
//! This module only wires the command line to the subcommand implementations.
//! All real work lives in the sibling modules:
//!
//! - [`cli`]     — argument definitions and validation
//! - [`reap`]    — stale, orphaned developer processes
//! - [`cleanup`] — Nix garbage collection policy
//! - [`tmp`]     — stale `/tmp` scratch files
//! - [`deep`]    — optional developer caches and macOS housekeeping
//! - [`doctor`]  — read-only health report
//! - [`notify`]  — macOS notifications
//! - [`tool`]    — locating external tools
//! - [`ui`]      — shared terminal formatting

mod cleanup;
mod cli;
mod deep;
mod doctor;
mod notify;
mod reap;
mod space;
mod space_cmd;
mod tmp;
mod tool;
mod ui;

use clap::{CommandFactory, Parser};
use clap_complete::generate;
use cli::{Cli, Commands};
use std::process::ExitCode;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let succeeded = match &cli.command {
        Commands::ReapZombies(args) => reap::run(args),
        Commands::SystemCleanup(args) => cleanup::run(args),
        Commands::CleanTmp(args) => {
            tmp::clean(args.dry_run, args.quiet, args.min_age);
            true
        }
        Commands::Doctor(args) => doctor::run(args),
        Commands::Space(args) => space_cmd::run(args),
        Commands::GenerateCompletion(args) => {
            let mut command = Cli::command();
            generate(
                args.shell,
                &mut command,
                args.bin_name.clone(),
                &mut std::io::stdout(),
            );
            true
        }
    };

    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
