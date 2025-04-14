//! Stores Command Line Interface (cli)  configuration
use clap::Parser;

#[derive(Parser, Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Default)]
#[command(
    author,
    version,
    about,
    long_about = "Performs a few updates and SEO validations as listed below:
    
    Updates page's front matter if the page is part of a section
    1. Ensures `tags` includes the name of the section
    2. Ensures the series is set to the section name
    
    Ensures each section has the `transparent` value set (can be true/false but must be present)
    
    SEO Verifications
    1. Ensures that the description in the config.toml is within 140-180 characters
    2. Ensures that the description on the pages is also in the same range    
"
)]
/// Stores the configurations acquired via the command line
pub struct Cli {
    #[arg(value_name = "PATH", default_value = ".")]
    /// The root folder to start at
    ///
    /// Usually you want to point this to the root of the zola repo.
    /// It is required for it to be in a repository with a clean working tree.
    pub root_path: String,

    /// If set will not modify any files and only report how many files would
    /// have been changed
    ///
    /// Return codes in this mode: (0) No files would have been changed (1)
    /// Error Occurred (2) Files would have been changed
    #[arg(long = "check", short = 'c')]
    pub should_check_only: bool,

    /// Allows changes to be made even if there are dirty files in the vcs.
    /// WARNING: This means that there will be no easy way to undo changes made
    ///
    /// Prefer at least staging files if possible over using this option. Only
    /// provided in case users really prefer not needing to stage their files.
    #[arg(long)]
    pub allow_dirty: bool,
}

#[cfg(test)]
mod tests {

    #[test]
    fn verify_cli() {
        // Source: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html#testing
        // My understanding it reports most development errors without additional effort
        use clap::CommandFactory;
        super::Cli::command().debug_assert()
    }
}
