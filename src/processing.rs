use crate::{cli::Cli, stats::Stats};

use anyhow::{Context, bail};
use std::{fs, path::Path, process::Command};
use tracing::{debug, error, trace, warn};

use self::file_data::FileData;
mod file_data;

pub fn walk_directory(root_path: &Path, cli: &Cli) -> anyhow::Result<Stats> {
    let mut result = Stats::new();
    if root_path.is_file() {
        match process_file(root_path, cli)
            .with_context(|| format!("Processing failed for: {root_path:?}"))
        {
            Ok(stats) => result += stats,
            Err(e) => {
                error!("{e:?}");
                result.inc_errors();
            }
        }
    } else {
        for entry in fs::read_dir(root_path)
            .with_context(|| format!("Failed to read directory: {root_path:?}"))?
        {
            let entry =
                entry.with_context(|| format!("Failed to extract a DirEntry in {root_path:?}"))?;
            let path = entry.path();
            result += walk_directory(&path, cli)?;
        }
    }

    Ok(result)
}

fn process_file(path: &Path, cli: &Cli) -> anyhow::Result<Stats> {
    let mut result = Stats::new();
    todo!("need to determine file type and")
    // if !should_skip_file(path) {
    //     let mut data = FileData::new_from_path(path)?;
    //     let last_edit_date =
    //         get_git_last_edit_date(path).context("Failed to get last edit
    // date from git")?;     data.update_front_matter(last_edit_date)
    //         .context("Failed to update front_matter")?;
    //     if data.is_changed() {
    //         result.inc_changed();
    //         if cli.should_check_only {
    //             warn!("(Change here) {path:?}");
    //         } else {
    //             data.write().context("Failed to write to file")?;
    //             trace!("(Changed)     {path:?}");
    //         }
    //     } else {
    //         result.inc_not_changed();
    //         trace!("(Not Changed) {path:?}");
    //     };
    // } else {
    //     result.inc_skipped();
    //     trace!("(Skipped)     {path:?}");
    // }
    // Ok(result)
}

fn should_skip_file(path: &Path) -> bool {
    path.extension().is_none_or(|ext| ext != "md") || path.ends_with("_index.md")
}
