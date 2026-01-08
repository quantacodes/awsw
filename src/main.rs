mod aws;
mod commands;
mod shell;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "awsw-bin")]
#[command(about = "AWS Profile Switcher - manage multiple AWS profiles with ease")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available AWS profiles
    List,
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

    let result = match cli.command {
        None => commands::select::run(),
        Some(Commands::List) => commands::list::run(),
        Some(Commands::Use { name, skip_validate }) => commands::use_cmd::run(&name, skip_validate),
        Some(Commands::Current) => commands::current::run(),
        Some(Commands::Unset) => commands::unset::run(),
        Some(Commands::Init { shell }) => commands::init::run(&shell),
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
