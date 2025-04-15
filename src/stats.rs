//! Information Related to Run Statistics

use std::{fmt::Display, ops::AddAssign};

/// Tracks files changed (NB: Stops counting at 2^16 and saturates)
#[derive(Debug, Default)]
#[must_use]
pub struct Stats {
    changed: u16,
    not_changed: u16,
    skipped: u16,
    seo_warnings: u16,
    errors: u16,
}

impl Stats {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Gets the current value of `changed`
    pub fn changed(&self) -> u16 {
        self.changed
    }

    /// Gets the current value of `not_changed`
    pub fn not_changed(&self) -> u16 {
        self.not_changed
    }

    /// Gets the current value of `skipped`
    pub fn skipped(&self) -> u16 {
        self.skipped
    }

    /// Gets the current value of `errors`
    pub fn errors(&self) -> u16 {
        self.errors
    }

    /// Gets the current value of `seo_failures`
    pub fn seo_warnings(&self) -> u16 {
        self.seo_warnings
    }

    /// Increments `changed` by 1 (saturating if applicable)
    pub fn inc_changed(&mut self) {
        self.changed = self.changed.saturating_add(1);
    }

    /// Increments `not_changed`` by 1 (saturating if applicable)
    pub fn inc_not_changed(&mut self) {
        self.not_changed = self.not_changed.saturating_add(1);
    }

    /// Increments `skipped` by 1 (saturating if applicable)
    pub fn inc_skipped(&mut self) {
        self.skipped = self.skipped.saturating_add(1);
    }

    /// Increments `errors` by 1 (saturating if applicable)
    pub fn inc_errors(&mut self) {
        self.errors = self.errors.saturating_add(1);
    }

    /// Increments `seo_warnings` by 1 (saturating if applicable)
    pub fn inc_seo_warnings(&mut self) {
        self.seo_warnings = self.seo_warnings.saturating_add(1);
    }
}

impl AddAssign for Stats {
    fn add_assign(&mut self, rhs: Self) {
        let Self {
            changed,
            not_changed,
            skipped,
            seo_warnings,
            errors,
        } = self;
        *changed += rhs.changed;
        *not_changed += rhs.not_changed;
        *skipped += rhs.skipped;
        *seo_warnings += rhs.seo_warnings;
        *errors += rhs.errors;
    }
}

impl Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            changed,
            not_changed,
            skipped,
            seo_warnings,
            errors,
        } = self;
        write!(
            f,
            "Changed: {changed}, Not Changed: {not_changed}, SEO Warnings: {seo_warnings}, Skipped: {skipped}, Errors: {errors}",
        )
    }
}
