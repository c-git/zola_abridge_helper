use anyhow::bail;
use clap::Parser;
use tracing::{debug, error};

use zola_abridge_helper::{self, Cli, init_tracing, run};

fn main() -> anyhow::Result<()> {
    let cli: Cli = Cli::parse();
    init_tracing();
    debug!("Cli: {cli:#?}");
    let stats = run(&cli)?;
    println!("File Stats: {stats}");
    if stats.errors() == 0 {
        if cli.should_check_only && stats.changed() > 0 {
            println!("{} files would have been changed", stats.changed());
            std::process::exit(2);
        }
    } else {
        let msg = format!("Run FAILED! {} errors", stats.errors());
        error!("{msg}");
        bail!("{msg}");
    }
    if !cli.ignore_seo && stats.seo_warnings() > 0 {
        println!("There are {} SEO warnings", stats.seo_warnings());
        std::process::exit(3);
    }
    Ok(())
}
