mod aws;
mod cache;
mod commands;
mod mfa;
mod shell;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "awsw-bin")]
#[command(about = "AWS Profile Switcher - manage multiple AWS profiles with ease")]
#[command(version)]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available AWS profiles
    List {
        /// Filter profiles by name (fuzzy match)
        filter: Option<String>,
    },
    /// Switch to a specific profile by name
    Use {
        /// Profile name (e.g., "default", "work/prod")
        name: String,
        /// Skip credential validation
        #[arg(long)]
        skip_validate: bool,
    },
    /// Show the currently active profile
    Current,
    /// Unset the current profile (return to default)
    Unset,
    /// Output shell integration function
    Init {
        /// Shell type (bash, zsh, fish)
        shell: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let verbose = cli.verbose;

    let result = match cli.command {
        None => commands::select::run(verbose),
        Some(Commands::List { filter }) => commands::list::run(filter.as_deref(), verbose),
        Some(Commands::Use { name, skip_validate }) => {
            commands::use_cmd::run(&name, skip_validate, verbose)
        }
        Some(Commands::Current) => commands::current::run(),
        Some(Commands::Unset) => commands::unset::run(),
        Some(Commands::Init { shell }) => commands::init::run(&shell),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
