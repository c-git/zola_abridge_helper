use std::{fs, io::Write, path::Path};

use anyhow::{Context, bail};
use once_cell::sync::Lazy;
use regex::Regex;
use toml_edit::DocumentMut;
use tracing::error;

use crate::{Cli, Stats};

use super::check_description;

static TOML_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[[:space:]]*\+\+\+(\r?\n(?s).*?(?-s))\+\+\+[[:space:]]*(?:$|(?:\r?\n((?s).*(?-s))$))",
    )
    .unwrap()
});

pub struct FileData<'a> {
    is_changed: bool,
    path: &'a Path,
    front_matter: String,
    content: String,
}

impl<'a> FileData<'a> {
    /// Write changes to disk.
    ///
    /// Precondition: Data is changed. If not changed function returns an error
    /// to avoid writing out the same data read in.
    pub fn write(&self) -> anyhow::Result<()> {
        if !self.is_changed() {
            bail!("No change detected. Write aborted. Path: {:?}", self.path);
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(self.path)?;
        let mut s = "+++".to_string();
        s.push_str(&self.front_matter);
        s.push_str("+++\n");
        if !self.content.is_empty() {
            // Added a space between to match `dprint`
            s.push('\n');
        }
        s.push_str(&self.content);
        file.write_all(s.as_bytes())?;
        Ok(())
    }

    fn front_matter_as_toml(&self) -> anyhow::Result<DocumentMut> {
        let toml = &self.front_matter[..];
        let result = toml
            .parse::<DocumentMut>()
            .context("Failed to parse TOML in front matter")?;
        debug_assert_eq!(result.to_string(), toml);
        Ok(result)
    }

    /// Extract info about a section (Name and Stats)
    /// Checks the transparent is set and is a boolean
    pub fn extract_section_info(&self) -> anyhow::Result<(String, Stats)> {
        let doc = self.front_matter_as_toml()?;
        let Some(result_name) = self
            .path
            .parent()
            .and_then(|x| x.file_name().map(|x| x.to_string_lossy().to_string()))
        else {
            bail!("failed to get section Name for file at: {:?}", self.path);
        };
        let mut result_stats = Stats::new();
        if !doc.get("transparent").is_some_and(|x| x.is_bool()) {
            error!(
                "Transparent not set or not bool for section in file at: {:?}",
                self.path
            );
            result_stats.inc_errors();
        }
        Ok((result_name, result_stats))
    }

    fn new(path: &'a Path, front_matter: String, content: String) -> Self {
        Self {
            is_changed: false,
            path,
            front_matter,
            content,
        }
    }

    pub(crate) fn is_changed(&self) -> bool {
        self.is_changed
    }

    /// Build a FileData from a path
    ///
    /// Splits the file data into front matter and content
    /// Patterned on zola code https://github.com/c-git/zola/blob/3a73c9c5449f2deda0d287f9359927b0440a77af/components/content/src/front_matter/split.rs#L46
    pub fn new_from_path(path: &Path) -> anyhow::Result<FileData> {
        let content = fs::read_to_string(path).context("Failed to read file")?;

        // 2. extract the front matter and the content
        let caps = if let Some(caps) = TOML_RE.captures(&content) {
            caps
        } else {
            bail!("Failed to find front matter");
        };
        // caps[0] is the full match
        // caps[1] => front matter
        // caps[2] => content
        let front_matter = caps.get(1).unwrap().as_str().to_string();
        let content = caps.get(2).map_or("", |m| m.as_str()).to_string();

        Ok(FileData::new(path, front_matter, content))
    }

    pub(crate) fn check_description(&self, cli: &Cli) -> anyhow::Result<Stats> {
        let toml_doc = self.front_matter_as_toml()?;
        Ok(check_description(&toml_doc, cli, self.path))
    }

    pub(crate) fn update_series_and_tags(
        &mut self,
        section_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let Some(section_name) = section_name else {
            return Ok(());
        };
        let mut doc = self.front_matter_as_toml()?;

        // Set series
        let key_extra = "extra";
        let key_series = "series";
        if let Some(extra) = doc.get_mut(key_extra) {
            if let Some(series) = extra.get_mut(key_series) {
                if Some(section_name) != series.as_str() {
                    self.is_changed = true;
                    *series = section_name.into();
                }
            }
        } else {
            self.is_changed = true;
            doc[key_extra][key_series] = section_name.into();
        }

        // Set tags
        let key_taxonomies = "taxonomies";
        let key_tags = "tags";
        let mut force_set_tag = |doc: &mut DocumentMut| {
            self.is_changed = true;
            let mut array = toml_edit::Array::new();
            array.push(section_name);
            doc[key_taxonomies][key_tags] = array.into();
        };
        if let Some(taxonomies) = doc.get_mut(key_taxonomies) {
            if let Some(tags) = taxonomies.get_mut(key_tags).and_then(|x| x.as_array_mut()) {
                if !tags.iter().any(|x| x.as_str() == Some(section_name)) {
                    self.is_changed = true;
                    tags.push(section_name);
                }
            } else {
                force_set_tag(&mut doc);
            }
        } else {
            force_set_tag(&mut doc);
        }

        if self.is_changed {
            self.front_matter = doc.to_string();
        }
        Ok(())
    }
}
