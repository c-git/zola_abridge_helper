use std::borrow::Cow;

use crate::TOML_KEY_EXTRA;

/// Note: both `series` and `tag` must use same set of values
#[derive(Debug, Clone)]
pub struct SectionInfo {
    title: Option<String>,
    folder_name: String,
    pub disable_check_series: bool,
    pub disable_check_tag: bool,
    pub disable_check_description: bool,
}

impl SectionInfo {
    pub fn new(title: Option<String>, folder_name: String) -> Self {
        Self {
            title,
            folder_name,
            disable_check_series: false,
            disable_check_tag: false,
            disable_check_description: false,
        }
    }

    // Defaults to section title if set otherwise the section foldername is used
    pub fn section_name(&self) -> &str {
        self.title.as_ref().unwrap_or(&self.folder_name)
    }

    pub fn load_settings(&self, doc: &toml_edit::DocumentMut) -> Cow<Self> {
        let mut result = Cow::Borrowed(self);

        let Some(extra) = doc.get(TOML_KEY_EXTRA) else {
            return result;
        };

        let get_bool = |key_name: &str| extra.get(key_name).and_then(|x| x.as_bool());

        if let Some(disable_check_series) = get_bool("disable_check_series") {
            if disable_check_series != result.disable_check_series {
                result.to_mut().disable_check_series = disable_check_series;
            }
        }

        if let Some(disable_check_tag) = get_bool("disable_check_tag") {
            if disable_check_tag != result.disable_check_tag {
                result.to_mut().disable_check_tag = disable_check_tag;
            }
        }

        if let Some(disable_check_description) = get_bool("disable_check_description") {
            if disable_check_description != result.disable_check_description {
                result.to_mut().disable_check_description = disable_check_description;
            }
        }

        result
    }
}
