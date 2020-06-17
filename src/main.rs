#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use structopt::StructOpt;

mod options {
    use structopt::clap::AppSettings;
    use structopt::StructOpt;

    #[derive(Debug, StructOpt)]
    #[structopt(about = "The git, simply swift")]
    #[structopt(settings = &[AppSettings::SubcommandRequired,
                            AppSettings::ColoredHelp])]
    pub struct Args {
        #[structopt(subcommand)]
        pub cmd: Subcommands,
    }

    #[derive(Debug, StructOpt)]
    pub enum Subcommands {
        /// Initialize the repository in the current directory.
        #[structopt(alias = "initialize")]
        #[structopt(setting = AppSettings::ColoredHelp)]
        Init,
    }
}

fn main() -> Result<()> {
    let args = options::Args::from_args();
    match args.cmd {
        options::Subcommands::Init => {
            git_core::init::repository().with_context(|| "Repository initialization failed")
        }
    }?;
    Ok(())
}
