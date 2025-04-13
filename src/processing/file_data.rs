use std::{fs, io::Write, path::Path};

use anyhow::{Context, bail};
use chrono::Datelike;
use once_cell::sync::Lazy;
use regex::Regex;
use toml_edit::DocumentMut;
use tracing::warn;

static TOML_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^[[:space:]]*\+\+\+(\r?\n(?s).*?(?-s))\+\+\+[[:space:]]*(?:$|(?:\r?\n((?s).*(?-s))$))",
    )
    .unwrap()
});

static TODAY: Lazy<toml_edit::Item> = Lazy::new(|| {
    let now = chrono::Local::now();
    item_from_date(toml_edit::Date {
        year: now.year() as _,
        month: now.month() as _,
        day: now.day() as _,
    })
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

    /// See cli::Cli command.long for explanation of rules (or readme)
    pub fn update_front_matter(
        &mut self,
        last_edit_date: Option<toml_edit::Date>,
    ) -> anyhow::Result<()> {
        let key_date = "date";
        let key_updated = "updated";
        let toml = &self.front_matter[..];
        let mut doc = toml
            .parse::<DocumentMut>()
            .context("Failed to parse TOML in front matter")?;
        debug_assert_eq!(doc.to_string(), toml);
        let org_date = doc.get(key_date);
        let org_updated = doc.get(key_updated);

        if last_edit_date.is_some()
            && is_less_than_date(&TODAY, &item_from_date(last_edit_date.unwrap()))
        {
            bail!(
                "Got a LAST edit date in the future...? We think today is: {} and last edit date found is {} for path {:?}",
                date_to_display(Some(&TODAY)),
                date_to_display(Some(&item_from_date(last_edit_date.unwrap()))),
                self.path
            )
        }

        let (new_date, new_updated) =
            self.calculate_new_date_and_updated(org_date, org_updated, last_edit_date);

        if !is_new_same_as_org(org_date, org_updated, &new_date, &new_updated) {
            self.is_changed = true;
            match doc.entry(key_date) {
                toml_edit::Entry::Occupied(mut entry) => *entry.get_mut() = new_date,
                toml_edit::Entry::Vacant(entry) => {
                    entry.insert(new_date);
                }
            }
            if let Some(nu) = new_updated {
                match doc.entry(key_updated) {
                    toml_edit::Entry::Occupied(mut entry) => *entry.get_mut() = nu,
                    toml_edit::Entry::Vacant(entry) => {
                        entry.insert(nu);
                    }
                }
            } else {
                doc.remove(key_updated);
            }
            self.front_matter = doc.to_string();
        }

        Ok(())
    }

    fn calculate_new_date_and_updated(
        &self,
        mut date: Option<&toml_edit::Item>,
        mut updated: Option<&toml_edit::Item>,
        last_edit_date: Option<toml_edit::Date>,
    ) -> (toml_edit::Item, Option<toml_edit::Item>) {
        assert!(
            last_edit_date.is_none()
                || is_less_than_or_equal_date(&item_from_date(last_edit_date.unwrap()), &TODAY),
            "Precondition to call this function is that `last_edit_date` must be today or in the past"
        );
        // Check for wrong type
        if let Some(d) = date {
            if !d.is_datetime() {
                warn!("Non date value found for `date` in {:?}", self.path);
                date = None; // Only allow dates
            }
        }
        if let Some(u) = updated {
            if !u.is_datetime() {
                warn!("Non date value found for `updated` in {:?}", self.path);
                updated = Some(&TODAY); // Only allow dates (Must be updated now)
            }
        }

        // Ensure if updated exists it is greater than or equal to date otherwise
        // discard value
        if let Some(updated_date) = updated {
            if let Some(date) = date {
                if is_less_than_date(updated_date, date) {
                    warn!(
                        "`updated` is before `date` but this should never happen. `updated` being ignored in {:?}",
                        self.path
                    );
                    updated = Some(&TODAY); // Must be updated now
                }
            }
        }

        // Clear date if it is in the future
        if let Some(curr_date) = date {
            if is_less_than_date(&TODAY, curr_date) {
                warn!(
                    "date is set in the future. Date is being ignored in {:?}",
                    self.path
                );
                date = None;
            }
        }
        if let Some(curr_updated) = updated {
            if is_less_than_date(&TODAY, curr_updated) {
                warn!(
                    "updated is set in the future. updated is being ignored in {:?}",
                    self.path
                );
                updated = Some(&TODAY); // Must be updated now
            }
        }

        // Set new date values base on the rules.
        // If changing to a date, prefer copying original value cuz dates created do not
        // include times nor offset Assumptions are documented here but are
        // enforced above. Documented here for ease of reference and not repeated below.
        debug_assert!(
            date.is_none() || is_less_than_or_equal_date(date.unwrap(), &TODAY),
            "ASSUMPTION FAILED. Expected: `date` if set to be today or in the past"
        );
        debug_assert!(
            updated.is_none() || is_less_than_or_equal_date(updated.unwrap(), &TODAY),
            "ASSUMPTION FAILED. Expected: `updated` if set to be today or in the past"
        );
        debug_assert!(
            date.is_none()
                || updated.is_none()
                || is_less_than_or_equal_date(date.unwrap(), updated.unwrap()),
            "ASSUMPTION FAILED. Expected: date <= updated"
        );
        let (new_date, new_updated) = match (last_edit_date, date, updated) {
            (None, None, _) => {
                // No dates, set `date` to TODAY clearing updated if it's set
                (TODAY.clone(), None)
            }
            (None, Some(date), _) => {
                // This file has never been committed but has `date`
                if is_equal_date(date, &TODAY) {
                    // `date` is TODAY, clear updated if it's set
                    (date.clone(), None)
                } else {
                    // Keep existing `date`. `updated` becomes TODAY
                    (date.clone(), Some(TODAY.clone()))
                }
            }
            (Some(last), None, _) => {
                // Previously committed but no dates set
                let last = item_from_date(last);
                if is_equal_date(&last, &TODAY) {
                    (last, None)
                } else {
                    (last, Some(TODAY.clone()))
                }
            }
            (Some(last), Some(date), None) => {
                // Previously committed check and `date` set. Set updated only if needed (ie.
                // `date` < `last`)
                let last = item_from_date(last);
                if is_less_than_or_equal_date(&last, date) {
                    (date.clone(), None)
                } else {
                    // `date` < `last` need to set `updated`
                    (date.clone(), Some(TODAY.clone()))
                }
            }
            (Some(last), Some(date), Some(updated)) => {
                // All 3 dates set
                let last = item_from_date(last);
                if is_equal_date(date, &TODAY) {
                    (date.clone(), None)
                } else if is_less_than_or_equal_date(&last, updated) {
                    // Values are fine, keep same
                    (date.clone(), Some(updated.clone()))
                } else {
                    // `updated` is too old. Set `updated` to TODAY
                    (date.clone(), Some(TODAY.clone()))
                }
            }
        };
        (new_date, new_updated)
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
}

fn is_new_same_as_org(
    org_date: Option<&toml_edit::Item>,
    org_updated: Option<&toml_edit::Item>,
    new_date: &toml_edit::Item,
    new_updated: &Option<toml_edit::Item>,
) -> bool {
    // Check if we've changed the starting values
    // NB: - date must change if it was None
    //     - This approach is slower due to loss of short circuit evaluation but I
    //       can read it, before it was...
    let is_date_same = org_date.is_some() && is_equal_date(org_date.unwrap(), new_date);
    let did_update_start_and_end_none =
        org_updated.is_none() && org_updated.is_none() == new_updated.is_none();
    let did_updated_start_some_and_end_same_value = org_updated.is_some()
        && new_updated.is_some()
        && is_equal_date(org_updated.unwrap(), new_updated.as_ref().unwrap());
    let is_update_same = did_update_start_and_end_none || did_updated_start_some_and_end_same_value;

    is_date_same && is_update_same
}

/// Checks if both a and b are dates and if a < b
fn is_less_than_date(a: &toml_edit::Item, b: &toml_edit::Item) -> bool {
    match (a, b) {
        (toml_edit::Item::Value(a), toml_edit::Item::Value(b)) => match (a, b) {
            (toml_edit::Value::Datetime(a), toml_edit::Value::Datetime(b)) => {
                match (a.value().date, b.value().date) {
                    (Some(a), Some(b)) => match a.year.cmp(&b.year) {
                        std::cmp::Ordering::Less => true,
                        std::cmp::Ordering::Equal => match a.month.cmp(&b.month) {
                            std::cmp::Ordering::Less => true,
                            std::cmp::Ordering::Equal => a.day < b.day,
                            std::cmp::Ordering::Greater => false,
                        },
                        std::cmp::Ordering::Greater => false,
                    },
                    _ => false,
                }
            }
            _ => false,
        },
        _ => false,
    }
}

/// Check if both a and b are dates and if a <= b
fn is_less_than_or_equal_date(a: &toml_edit::Item, b: &toml_edit::Item) -> bool {
    is_less_than_date(a, b) || is_equal_date(a, b)
}
fn item_from_date(d: toml_edit::Date) -> toml_edit::Item {
    toml_edit::Item::Value(toml_edit::Value::Datetime(toml_edit::Formatted::new(
        toml_edit::Datetime {
            date: Some(d),
            time: None,
            offset: None,
        },
    )))
}

// Check if both a and b are dates and a == b
fn is_equal_date(a: &toml_edit::Item, b: &toml_edit::Item) -> bool {
    match (a, b) {
        (toml_edit::Item::Value(a), toml_edit::Item::Value(b)) => match (a, b) {
            (toml_edit::Value::Datetime(a), toml_edit::Value::Datetime(b)) => {
                match (a.value().date, b.value().date) {
                    (Some(a), Some(b)) => a.year == b.year && a.month == b.month && a.day == b.day,
                    _ => false,
                }
            }
            _ => false,
        },
        _ => false,
    }
}

/// Helper function to print dates in items. Panics if item is not a well formed
/// date item object or None
fn date_to_display(d: Option<&toml_edit::Item>) -> String {
    if let Some(d) = d {
        if let toml_edit::Item::Value(d) = d {
            if let toml_edit::Value::Datetime(d) = d {
                if let Some(d) = d.value().date {
                    format!("{:0>4}-{:0>2}-{:0>2}", d.year, d.month, d.day)
                } else {
                    panic!("Expected Some")
                }
            } else {
                panic!("Expected Datetime")
            }
        } else {
            panic!("Expected Value")
        }
    } else {
        "None".to_string()
    }
}

#[cfg(test)]
/// These test only test pure functions that are just building blocks for other
/// parts of the module
mod test_helpers {
    use super::*;

    #[test]
    fn test_is_equal_date() {
        assert!(is_equal_date(&TODAY, &TODAY));
    }

    #[test]
    fn test_is_less_than() {
        let past = item_from_date(toml_edit::Date {
            year: 1900,
            month: 1,
            day: 1,
        });
        assert!(is_less_than_date(&past, &TODAY));
        assert!(!is_less_than_date(&TODAY, &past));
        assert!(!is_less_than_date(&TODAY, &TODAY));
    }

    #[test]
    fn test_is_less_than_or_equal() {
        let past = item_from_date(toml_edit::Date {
            year: 1900,
            month: 1,
            day: 1,
        });
        assert!(is_less_than_or_equal_date(&past, &TODAY));
        assert!(!is_less_than_or_equal_date(&TODAY, &past));
        assert!(is_less_than_or_equal_date(&TODAY, &TODAY));
    }
}

#[cfg(test)]
/// Test the core logic of the module (and helps define it, by providing cases
/// and the expected behaviour)
mod test_function {
    use std::path::PathBuf;

    use rstest::rstest;

    use super::*;

    type DT = (u16, u8, u8); // Date Tuple
    type DTopt = Option<DT>; // Date Tuple Option

    static TODAY_TUPLE: Lazy<DTopt> = Lazy::new(|| {
        let now = chrono::Local::now();
        Some((now.year() as _, now.month() as _, now.day() as _))
    });

    fn item_from_tuple_opt(value: DTopt) -> toml_edit::Item {
        if let Some(tuple) = value {
            item_from_date(date_from_tuple(tuple))
        } else {
            toml_edit::Item::None
        }
    }

    fn date_from_tuple(value: DT) -> toml_edit::Date {
        toml_edit::Date {
            year: value.0,
            month: value.1,
            day: value.2,
        }
    }

    fn assert_same(
        actual: Option<&toml_edit::Item>,
        expected: Option<&toml_edit::Item>,
        variable_name: &str,
    ) {
        match (actual, expected) {
            (None, None) => (),
            (None, Some(_)) | (Some(_), None) => panic!(
                "{variable_name:?} actual does not match expected.\nactual: {}\nexpected: {}",
                date_to_display(actual),
                date_to_display(expected)
            ),
            (Some(a), Some(b)) => assert!(
                is_equal_date(a, b),
                "{variable_name:?} actual does not match expected.\nactual: {}\nexpected: {}",
                date_to_display(actual),
                date_to_display(expected)
            ),
        }
    }

    static PAST1: DTopt = Some((2001, 1, 1));
    static PAST2: DTopt = Some((2002, 1, 1));
    static PAST3: DTopt = Some((2003, 1, 1));
    static FUTURE: DTopt = Some((4000, 1, 1));

    // NB: Empty comments were added to last value to line up columns of values
    #[rstest]
    #[case(PAST2,        None,         None,         PAST2,        *TODAY_TUPLE, "01")]
    #[case(PAST2,        None,         PAST1,        PAST2,        *TODAY_TUPLE, "02")]
    #[case(PAST2,        None,         PAST2,        PAST2,        *TODAY_TUPLE, "03")]
    #[case(PAST2,        None,         *TODAY_TUPLE, PAST2,        *TODAY_TUPLE, "04")]
    #[case(PAST2,        PAST1,        None,         PAST1,        *TODAY_TUPLE, "05")]
    #[case(PAST2,        PAST1,        PAST1,        PAST1,        *TODAY_TUPLE, "06")]
    #[case(PAST2,        PAST1,        PAST2,        PAST1,        PAST2, /* */  "07")]
    #[case(PAST2,        PAST1,        PAST3,        PAST1,        PAST3, /* */  "08")]
    #[case(PAST2,        PAST1,        *TODAY_TUPLE, PAST1,        *TODAY_TUPLE, "09")]
    #[case(PAST2,        PAST2,        None,         PAST2,        None,  /* */  "10")]
    #[case(PAST2,        PAST2,        PAST1,        PAST2,        *TODAY_TUPLE, "11")]
    #[case(PAST2,        PAST2,        PAST2,        PAST2,        PAST2, /* */  "12")]
    #[case(PAST2,        PAST2,        PAST3,        PAST2,        PAST3, /* */  "13")]
    #[case(PAST2,        PAST2,        *TODAY_TUPLE, PAST2,        *TODAY_TUPLE, "14")]
    #[case(PAST2,        PAST3,        None,         PAST3,        None,  /* */  "15")]
    #[case(PAST2,        PAST3,        PAST1,        PAST3,        *TODAY_TUPLE, "16")]
    #[case(PAST2,        PAST3,        PAST2,        PAST3,        *TODAY_TUPLE, "17")]
    #[case(PAST2,        PAST3,        PAST3,        PAST3,        PAST3, /* */  "18")]
    #[case(PAST2,        PAST3,        *TODAY_TUPLE, PAST3,        *TODAY_TUPLE, "19")]
    #[case(PAST2,        *TODAY_TUPLE, None,         *TODAY_TUPLE, None,         "20")]
    #[case(PAST2,        *TODAY_TUPLE, PAST1,        *TODAY_TUPLE, None,         "21")]
    #[case(PAST2,        *TODAY_TUPLE, *TODAY_TUPLE, *TODAY_TUPLE, None,         "22")]
    #[case(None,         None,         None,         *TODAY_TUPLE, None,         "23")]
    #[case(None,         None,         PAST1,        *TODAY_TUPLE, None,         "24")]
    #[case(None,         None,         *TODAY_TUPLE, *TODAY_TUPLE, None,         "25")]
    #[case(None,         PAST1,        None,         PAST1,        *TODAY_TUPLE, "26")]
    #[case(None,         PAST1,        PAST1,        PAST1,        *TODAY_TUPLE, "27")]
    #[case(None,         PAST1,        PAST2,        PAST1,        *TODAY_TUPLE, "28")]
    #[case(None,         PAST1,        *TODAY_TUPLE, PAST1,        *TODAY_TUPLE, "29")]
    #[case(None,         *TODAY_TUPLE, None,         *TODAY_TUPLE, None,         "30")]
    #[case(None,         *TODAY_TUPLE, PAST1,        *TODAY_TUPLE, None,         "31")]
    #[case(None,         *TODAY_TUPLE, *TODAY_TUPLE, *TODAY_TUPLE, None,         "32")]
    #[case(None,         FUTURE,       None,         *TODAY_TUPLE, None,         "33")]
    #[case(None,         None,         FUTURE,       *TODAY_TUPLE, None,         "34")]
    #[case(None,         FUTURE,       FUTURE,       *TODAY_TUPLE, None,         "35")]
    #[case(None,         FUTURE,       *TODAY_TUPLE, *TODAY_TUPLE, None,         "36")]
    #[case(None,         *TODAY_TUPLE, FUTURE,       *TODAY_TUPLE, None,         "37")]
    #[case(PAST2,        FUTURE,       None,         PAST2,        *TODAY_TUPLE, "38")]
    #[case(PAST2,        None,         FUTURE,       PAST2,        *TODAY_TUPLE, "39")]
    #[case(PAST2,        FUTURE,       FUTURE,       PAST2,        *TODAY_TUPLE, "40")]
    #[case(PAST2,        FUTURE,       *TODAY_TUPLE, PAST2,        *TODAY_TUPLE, "41")]
    #[case(PAST2,        *TODAY_TUPLE, FUTURE,       *TODAY_TUPLE, None,         "42")]
    fn date_logic_case(
        #[case] last: DTopt,
        #[case] date: DTopt,
        #[case] updated: DTopt,
        #[case] expected_date: DTopt,
        #[case] expected_updated: DTopt,
        #[case] test_name: &str,
    ) {
        println!("Test Name: {test_name:?}");
        let path = PathBuf::new();
        let mock = FileData::new(&path, Default::default(), Default::default());

        // Set org_date
        let item = item_from_tuple_opt(date);
        let org_date = date.map(|_| &item);

        // Set org_updated
        let item = item_from_tuple_opt(updated);
        let org_updated = updated.map(|_| &item);

        // Set last
        let last_edit_date = last.map(date_from_tuple);

        // Set expected_date
        let item = item_from_tuple_opt(expected_date);
        let expected_date = expected_date.map(|_| &item);

        // Set expected_updated
        let item = item_from_tuple_opt(expected_updated);
        let expected_updated = expected_updated.map(|_| &item);

        // Set expected_is_changed
        let expected_is_changed = match (org_date, expected_date, org_updated, expected_updated) {
            (None, None, None, None) => false,
            (Some(a), Some(b), None, None) | (None, None, Some(a), Some(b)) => !is_equal_date(a, b),
            (None, None, None, Some(_))
            | (None, None, Some(_), None)
            | (None, Some(_), None, None)
            | (None, Some(_), None, Some(_))
            | (None, Some(_), Some(_), None)
            | (None, Some(_), Some(_), Some(_))
            | (Some(_), None, None, None)
            | (Some(_), None, None, Some(_))
            | (Some(_), None, Some(_), None)
            | (Some(_), None, Some(_), Some(_))
            | (Some(_), Some(_), None, Some(_))
            | (Some(_), Some(_), Some(_), None) => true, // Mismatched pairs (Some with None)
            (Some(a), Some(b), Some(c), Some(d)) => !is_equal_date(a, b) || !is_equal_date(c, d),
        };

        let (actual_date, actual_updated) =
            mock.calculate_new_date_and_updated(org_date, org_updated, last_edit_date);

        let actual_is_changed =
            !is_new_same_as_org(org_date, org_updated, &actual_date, &actual_updated);

        assert_same(Some(&actual_date), expected_date, "date");
        assert_same(actual_updated.as_ref(), expected_updated, "updated");
        assert_eq!(
            actual_is_changed, expected_is_changed,
            "is_changed doesn't match expectation"
        );
    }
}
