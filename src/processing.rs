use crate::{PREFERRED_RANGE, cli::Cli, stats::Stats};

use anyhow::Context;
use std::{fs, path::Path};
use toml_edit::DocumentMut;
use tracing::{error, trace, warn};

use self::file_data::FileData;
mod file_data;

pub fn validate_zola_config(path: &Path) -> anyhow::Result<Stats> {
    let contents = fs::read_to_string(path)
        .with_context(|| format!("failed to read config at: {:?}", path))?;
    let toml_doc = contents
        .parse::<DocumentMut>()
        .with_context(|| format!("Failed to parse config at: {path:?}"))?;
    let mut result = Stats::new();
    let Some(description) = toml_doc.get("description") else {
        result.inc_seo_warnings();
        warn!("(SEO) failed to find description in config at: {path:?}");
        return Ok(result);
    };
    let Some(description) = description.as_str() else {
        result.inc_seo_warnings();
        warn!(
            "(SEO) failed to use description as string in config at: {path:?}. Value found: {description:?}"
        );
        return Ok(result);
    };
    if !is_description_length_in_preferred_range(description) {
        result.inc_seo_warnings();
        warn!(
            "(SEO) config description outside of the preferred range at {}. Preferred range: {:?}, Config Path: {path:?}",
            description.len(),
            PREFERRED_RANGE
        );
    }
    Ok(result)
}

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
    // TODO 1: todo!("need to determine file type and")
    if !should_skip_file(path) {
        let data = FileData::new_from_path(path)?;
        // data.update_front_matter(last_edit_date)
        //     .context("Failed to update front_matter")?;
        if data.is_changed() {
            result.inc_changed();
            if cli.should_check_only {
                warn!("(Change here) {path:?}");
            } else {
                data.write().context("Failed to write to file")?;
                trace!("(Changed)     {path:?}");
            }
        } else {
            result.inc_not_changed();
            trace!("(Not Changed) {path:?}");
        };
    } else {
        result.inc_skipped();
        trace!("(Skipped)     {path:?}");
    }
    Ok(result)
}

fn should_skip_file(path: &Path) -> bool {
    path.extension().is_none_or(|ext| ext != "md") || path.ends_with("_index.md")
}

fn is_description_length_in_preferred_range(desc: &str) -> bool {
    PREFERRED_RANGE.contains(&desc.len())
}
