use std::process::Command;

use super::Profile;

/// Result of credential validation
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

/// Validate AWS credentials by calling aws sts get-caller-identity
pub fn validate_credentials(profile: &Profile) -> Result<ValidationResult, String> {
    let mut cmd = Command::new("aws");
    cmd.arg("sts").arg("get-caller-identity").arg("--output").arg("json");

    // Set the profile
    cmd.env("AWS_PROFILE", &profile.raw_name);

    // Set config/credentials files if from a project
    if let Some(ref config_file) = profile.config_file {
        cmd.env("AWS_CONFIG_FILE", config_file);
    }
    if let Some(ref creds_file) = profile.credentials_file {
        cmd.env("AWS_SHARED_CREDENTIALS_FILE", creds_file);
    }

    let output = cmd.output().map_err(|e| format!("Failed to run aws CLI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Invalid credentials: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_caller_identity(&stdout)
}

fn parse_caller_identity(json: &str) -> Result<ValidationResult, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse response: {}", e))?;

    let account = parsed["Account"]
        .as_str()
        .ok_or("Missing Account in response")?
        .to_string();

    let arn = parsed["Arn"]
        .as_str()
        .ok_or("Missing Arn in response")?
        .to_string();

    let user_id = parsed["UserId"]
        .as_str()
        .ok_or("Missing UserId in response")?
        .to_string();

    Ok(ValidationResult {
        account,
        arn,
        user_id,
    })
}
