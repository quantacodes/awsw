use console::style;

use crate::aws::{find_profile, validate_credentials, Profile};

pub fn run(name: &str, skip_validate: bool) -> Result<(), Box<dyn std::error::Error>> {
    let profile = find_profile(name)?
        .ok_or_else(|| format!("Profile '{}' not found", name))?;

    if !skip_validate {
        eprintln!("{}", style("Validating credentials...").dim());

        match validate_credentials(&profile) {
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
