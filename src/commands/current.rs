use console::style;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let current = std::env::var("AWS_PROFILE").ok();
    let config_file = std::env::var("AWS_CONFIG_FILE").ok();
    let creds_file = std::env::var("AWS_SHARED_CREDENTIALS_FILE").ok();

    match current {
        Some(profile) => {
            eprintln!("Current profile: {}", style(&profile).green().bold());
        }
        None => {
            eprintln!(
                "Current profile: {} {}",
                style("default").yellow(),
                style("(AWS_PROFILE not set)").dim()
            );
        }
    }

    // Show config files if set
    if let Some(config) = config_file {
        eprintln!("Config file: {}", style(&config).cyan());
    }
    if let Some(creds) = creds_file {
        eprintln!("Credentials file: {}", style(&creds).cyan());
    }

    Ok(())
}
