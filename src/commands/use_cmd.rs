use console::style;

use super::common::output_export_commands;
use crate::aws::{find_profile, validate_credentials, validate_credentials_verbose};
use crate::mfa::{obtain_mfa_credentials, output_export_commands_with_mfa};

pub fn run(name: &str, skip_validate: bool, verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    let profile = find_profile(name)?
        .ok_or_else(|| format!("Profile '{}' not found", name))?;

    if verbose {
        eprintln!(
            "{} Profile: {}",
            style("[verbose]").dim(),
            style(&profile.name).cyan()
        );
        if let Some(ref creds) = profile.credentials_file {
            eprintln!(
                "{} Credentials file: {}",
                style("[verbose]").dim(),
                style(creds.display()).cyan()
            );
        }
        if let Some(ref config) = profile.config_file {
            eprintln!(
                "{} Config file: {}",
                style("[verbose]").dim(),
                style(config.display()).cyan()
            );
        }
        if profile.requires_mfa() {
            eprintln!(
                "{} MFA required: {}",
                style("[verbose]").dim(),
                style("yes").yellow()
            );
        }
    }

    // Check if profile requires MFA
    if profile.requires_mfa() {
        // Handle MFA authentication
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create runtime: {}", e))?;

        let mfa_creds = rt.block_on(obtain_mfa_credentials(&profile, verbose))?;

        output_export_commands_with_mfa(&profile, &mfa_creds);
        eprintln!(
            "{} {} {}",
            style("Switched to").green(),
            style(&profile.name).green().bold(),
            style("(with MFA)").dim()
        );

        return Ok(());
    }

    // Normal flow for non-MFA profiles
    if !skip_validate {
        eprintln!("{}", style("Validating credentials...").dim());

        let result = if verbose {
            validate_credentials_verbose(&profile)
        } else {
            validate_credentials(&profile)
        };

        match result {
            Ok(result) => {
                output_export_commands(&profile);
                eprintln!(
                    "{} {} {} {}",
                    style("Switched to").green(),
                    style(&profile.name).green().bold(),
                    style(format!("(Account: {})", result.account)).dim(),
                    style(format!("ARN: {}", result.arn)).dim()
                );
            }
            Err(e) => {
                return Err(format!("Failed to validate credentials for '{}': {}", name, e).into());
            }
        }
    } else {
        output_export_commands(&profile);
        eprintln!(
            "{} {} {}",
            style("Switched to").green(),
            style(&profile.name).green().bold(),
            style("(validation skipped)").dim()
        );
    }

    Ok(())
}
