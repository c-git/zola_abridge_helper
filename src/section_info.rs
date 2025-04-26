use std::borrow::Cow;

use crate::TOML_KEY_EXTRA;

#[derive(Debug, Clone)]
pub struct SectionInfo {
    pub section_title: Option<String>,
    pub section_folder: String,
    pub disable_check_series: bool,
    pub disable_check_tag: bool,
    pub disable_check_description: bool,
}

impl SectionInfo {
    pub fn new(section_folder: String) -> Self {
        Self {
            section_title: None,
            section_folder,
            disable_check_series: false,
            disable_check_tag: false,
            disable_check_description: false,
        }
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
