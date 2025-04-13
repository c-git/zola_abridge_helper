#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![forbid(unsafe_code)]
#![deny(unused_crate_dependencies)]
#![doc = include_str!("../README.md")]

mod cli;
mod processing;
mod stats;

use crate::processing::walk_directory;
use anyhow::Context;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    time::Instant,
};
use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt as _, util::SubscriberInitExt as _};
use version_control_clean_check::{CheckOptions, check_version_control};

pub use cli::Cli;
pub use stats::Stats;

/// Runs the body of the logic
pub fn run(cli: &Cli) -> anyhow::Result<Stats> {
    // This also checks that the path exists as that is required for
    // canonicalization
    let root_path = PathBuf::from(&cli.root_path)
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize path: '{}'", cli.root_path))?;

    let check_options = CheckOptions {
        // This makes it possible for the user to undo our changes if any so this is fine
        allow_staged: true,
        // Set when dirty is allowed (Either we aren't going to make changes so it's fine or the
        // user opted into allowing dirty files)
        allow_dirty: cli.should_check_only || cli.allow_dirty,
        ..Default::default()
    };

    // Confirm it is safe to make changes
    check_version_control(&root_path, &check_options).with_context(|| {
        format!(
            "Failed to find a clean version control system. Files must be at least staged before tool can run or you can opt-out of being able to revert changes. See help for more info.\nPath:{root_path:?}"
        )
    })?;

    // Confirm user wants to make changes
    if !cli.should_check_only && !cli.unattended && !confirm_proceed(&root_path) {
        println!("Aborted at users request");
        return Ok(Stats::default());
    }

    // Change current working directory to target folder
    // env::set_current_dir(&root_path).context("Failed change working directory to
    // {root_path:?}")?;

    // TODO 1: check if folder contains the config.toml so we can check it before we
    // start going down the content folder

    // Walk tree and process files
    let start = Instant::now();
    let result = walk_directory(&root_path, cli)?;
    info!(
        "Run duration: {} ms",
        Instant::now().duration_since(start).as_millis()
    );
    println!("Run Completed");
    Ok(result)
}

fn confirm_proceed(root_path: &Path) -> bool {
    print!("Do you whish to allow changes at {root_path:?}? (enter 'yes' to proceed) ");
    io::stdout().flush().expect("Failed to flush to stdout");

    let mut user_input = String::new();
    io::stdin()
        .read_line(&mut user_input)
        .expect("Failed to read line");

    user_input.trim().to_lowercase() == "yes"
}

/// Initializes tracing
pub fn init_tracing() {
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();
}
