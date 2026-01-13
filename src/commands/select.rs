use console::style;
use dialoguer::{theme::ColorfulTheme, FuzzySelect};

use super::common::output_export_commands;
use crate::aws::{scan_profiles, scan_profiles_verbose, validate_credentials, validate_credentials_verbose, Profile};
use crate::mfa::{obtain_mfa_credentials, output_export_commands_with_mfa};

pub fn run(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let profiles = if verbose {
        scan_profiles_verbose()?
    } else {
        scan_profiles()?
    };

    if profiles.is_empty() {
        return Err("No AWS profiles found in ~/.aws/".into());
    }

    let current = std::env::var("AWS_PROFILE").ok();

    // Build display items with current marker and MFA indicator
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
            let mfa_indicator = if p.requires_mfa() { " 🔐" } else { "" };

            format!("{} {}{}{}", marker, p.name, region, mfa_indicator)
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
            switch_to_profile(profile, verbose)?;
        }
        None => {
            eprintln!("{}", style("Cancelled").dim());
        }
    }

    Ok(())
}

fn switch_to_profile(profile: &Profile, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    // Check if profile requires MFA
    if profile.requires_mfa() {
        // Handle MFA authentication
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        let mfa_creds = rt.block_on(obtain_mfa_credentials(profile, verbose))?;

        output_export_commands_with_mfa(profile, &mfa_creds);
        eprintln!(
            "{} {} {}",
            style("Switched to").green(),
            style(&profile.name).green().bold(),
            style("(with MFA)").dim()
        );

        return Ok(());
    }

    // Normal validation flow
    eprintln!("{}", style("Validating credentials...").dim());

    let result = if verbose {
        validate_credentials_verbose(profile)
    } else {
        validate_credentials(profile)
    };

    match result {
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
