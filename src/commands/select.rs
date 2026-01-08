use console::style;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

use crate::aws::{scan_profiles, validate_credentials, Profile};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let profiles = scan_profiles()?;

    if profiles.is_empty() {
        return Err("No AWS profiles found in ~/.aws/".into());
    }

    let current = std::env::var("AWS_PROFILE").ok();

    // Build display items with current marker
    let items: Vec<String> = profiles
        .iter()
        .map(|p| {
            let is_current = current.as_ref().map(|c| c == &p.raw_name).unwrap_or(false)
                || (current.is_none() && p.raw_name == "default");

            let marker = if is_current { "*" } else { " " };
            let region = p
                .region
                .as_ref()
                .map(|r| format!(" ({})", r))
                .unwrap_or_default();

            format!("{} {}{}", marker, p.name, region)
        })
        .collect();

    // Find default selection (current profile)
    let default_idx = profiles
        .iter()
        .position(|p| {
            current.as_ref().map(|c| c == &p.raw_name).unwrap_or(false)
                || (current.is_none() && p.raw_name == "default")
        })
        .unwrap_or(0);

    let selection = FuzzySelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Select AWS profile")
        .items(&items)
        .default(default_idx)
        .interact_opt()?;

    match selection {
        Some(idx) => {
            let profile = &profiles[idx];
            switch_to_profile(profile)?;
        }
        None => {
            eprintln!("{}", style("Cancelled").dim());
        }
    }

    Ok(())
}

fn switch_to_profile(profile: &Profile) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("{}", style("Validating credentials...").dim());

    match validate_credentials(profile) {
        Ok(result) => {
            output_export_commands(profile);
            eprintln!(
                "{} {} {} {}",
                style("Switched to").green(),
                style(&profile.name).green().bold(),
                style(format!("(Account: {})", result.account)).dim(),
                style(format!("ARN: {}", result.arn)).dim()
            );
            Ok(())
        }
        Err(e) => Err(format!("Failed to validate credentials: {}", e).into()),
    }
}

fn output_export_commands(profile: &Profile) {
    // Output export commands to stdout for shell to eval
    println!("export AWS_PROFILE='{}'", profile.raw_name);

    if let Some(ref config_file) = profile.config_file {
        println!("export AWS_CONFIG_FILE='{}'", config_file.display());
    } else {
        println!("unset AWS_CONFIG_FILE");
    }

    if let Some(ref creds_file) = profile.credentials_file {
        println!(
            "export AWS_SHARED_CREDENTIALS_FILE='{}'",
            creds_file.display()
        );
    } else {
        println!("unset AWS_SHARED_CREDENTIALS_FILE");
    }
}
