use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use console::style;
use dialoguer::Input;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::aws::Profile;

const MFA_CACHE_FILE: &str = ".awsp_mfa_cache";

#[derive(Serialize, Deserialize)]
pub struct MfaCredentialCache {
    pub credentials: HashMap<String, CachedCredentials>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expiration: u64, // Unix timestamp
}

impl CachedCredentials {
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Consider expired 5 minutes before actual expiration
        self.expiration.saturating_sub(300) <= now
    }
}

impl MfaCredentialCache {
    fn cache_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".aws").join(MFA_CACHE_FILE))
    }

    pub fn load() -> Self {
        Self::cache_path()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or(MfaCredentialCache {
                credentials: HashMap::new(),
            })
    }

    pub fn save(&self) {
        if let Some(path) = Self::cache_path() {
            if let Ok(json) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, json);
            }
        }
    }

    pub fn get(&self, profile_name: &str) -> Option<&CachedCredentials> {
        self.credentials
            .get(profile_name)
            .filter(|c| !c.is_expired())
    }

    pub fn set(&mut self, profile_name: &str, creds: CachedCredentials) {
        self.credentials.insert(profile_name.to_string(), creds);
        self.save();
    }
}

/// Prompt user for MFA token and obtain session credentials
pub async fn obtain_mfa_credentials(
    profile: &Profile,
    verbose: bool,
) -> Result<CachedCredentials, String> {
    // Check cache first
    let mut cache = MfaCredentialCache::load();
    if let Some(cached) = cache.get(&profile.name) {
        if verbose {
            eprintln!(
                "{} Using cached MFA credentials (expires in {} minutes)",
                style("[verbose]").dim(),
                (cached.expiration.saturating_sub(
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0)
                )) / 60
            );
        }
        return Ok(cached.clone());
    }

    // Prompt for MFA code
    let mfa_serial = profile
        .mfa_serial
        .as_ref()
        .ok_or("Profile has no mfa_serial configured")?;

    eprintln!(
        "{} MFA required for profile '{}'",
        style("🔐").bold(),
        style(&profile.name).cyan()
    );
    eprintln!("   Device: {}", style(mfa_serial).dim());

    let mfa_code = prompt_mfa_code()?;

    // Determine which STS call to make
    let creds = if let Some(role_arn) = &profile.role_arn {
        // Use assume-role with MFA
        if verbose {
            eprintln!(
                "{} Assuming role with MFA: {}",
                style("[verbose]").dim(),
                role_arn
            );
        }
        assume_role_with_mfa(profile, role_arn, mfa_serial, &mfa_code).await?
    } else {
        // Use get-session-token with MFA
        if verbose {
            eprintln!(
                "{} Getting session token with MFA",
                style("[verbose]").dim()
            );
        }
        get_session_token_with_mfa(profile, mfa_serial, &mfa_code).await?
    };

    // Cache the credentials
    cache.set(&profile.name, creds.clone());

    eprintln!("{} MFA authentication successful", style("✓").green());

    Ok(creds)
}

fn prompt_mfa_code() -> Result<String, String> {
    Input::new()
        .with_prompt("Enter MFA token code")
        .validate_with(|input: &String| {
            if input.len() == 6 && input.chars().all(|c| c.is_ascii_digit()) {
                Ok(())
            } else {
                Err("MFA code must be 6 digits")
            }
        })
        .interact_text()
        .map_err(|e| format!("Failed to read MFA code: {}", e))
}

async fn get_session_token_with_mfa(
    profile: &Profile,
    mfa_serial: &str,
    mfa_code: &str,
) -> Result<CachedCredentials, String> {
    let source_profile = profile.source_profile.as_ref().unwrap_or(&profile.raw_name);

    let mut cmd = Command::new("aws");
    cmd.args([
        "sts",
        "get-session-token",
        "--serial-number",
        mfa_serial,
        "--token-code",
        mfa_code,
        "--profile",
        source_profile,
        "--output",
        "json",
    ]);

    if let Some(ref config_file) = profile.config_file {
        cmd.env("AWS_CONFIG_FILE", config_file);
    }
    if let Some(ref creds_file) = profile.credentials_file {
        cmd.env("AWS_SHARED_CREDENTIALS_FILE", creds_file);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run aws sts get-session-token: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("MFA authentication failed: {}", stderr.trim()));
    }

    parse_sts_credentials(&String::from_utf8_lossy(&output.stdout))
}

async fn assume_role_with_mfa(
    profile: &Profile,
    role_arn: &str,
    mfa_serial: &str,
    mfa_code: &str,
) -> Result<CachedCredentials, String> {
    let source_profile = profile.source_profile.as_ref().unwrap_or(&profile.raw_name);

    let session_name = format!("awsp-{}", std::process::id());

    let mut cmd = Command::new("aws");
    cmd.args([
        "sts",
        "assume-role",
        "--role-arn",
        role_arn,
        "--role-session-name",
        &session_name,
        "--serial-number",
        mfa_serial,
        "--token-code",
        mfa_code,
        "--profile",
        source_profile,
        "--output",
        "json",
    ]);

    if let Some(ref config_file) = profile.config_file {
        cmd.env("AWS_CONFIG_FILE", config_file);
    }
    if let Some(ref creds_file) = profile.credentials_file {
        cmd.env("AWS_SHARED_CREDENTIALS_FILE", creds_file);
    }

    let output = cmd
        .output()
        .await
        .map_err(|e| format!("Failed to run aws sts assume-role: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Role assumption with MFA failed: {}", stderr.trim()));
    }

    parse_sts_credentials(&String::from_utf8_lossy(&output.stdout))
}

fn parse_sts_credentials(json: &str) -> Result<CachedCredentials, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("Failed to parse STS response: {}", e))?;

    let creds = &parsed["Credentials"];

    let access_key_id = creds["AccessKeyId"]
        .as_str()
        .ok_or("Missing AccessKeyId")?
        .to_string();
    let secret_access_key = creds["SecretAccessKey"]
        .as_str()
        .ok_or("Missing SecretAccessKey")?
        .to_string();
    let session_token = creds["SessionToken"]
        .as_str()
        .ok_or("Missing SessionToken")?
        .to_string();
    let expiration_str = creds["Expiration"]
        .as_str()
        .ok_or("Missing Expiration")?;

    // Parse ISO 8601 timestamp (e.g., "2023-01-01T12:00:00Z")
    let expiration = parse_iso8601_timestamp(expiration_str)?;

    Ok(CachedCredentials {
        access_key_id,
        secret_access_key,
        session_token,
        expiration,
    })
}

fn parse_iso8601_timestamp(s: &str) -> Result<u64, String> {
    // Simple parsing for AWS timestamp format: "2023-01-01T12:00:00Z" or "2023-01-01T12:00:00+00:00"
    // AWS always returns UTC timestamps

    // Remove timezone suffix
    let s = s.trim_end_matches('Z').split('+').next().unwrap_or(s);

    let parts: Vec<&str> = s.split('T').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid timestamp format: {}", s));
    }

    let date_parts: Vec<&str> = parts[0].split('-').collect();
    let time_parts: Vec<&str> = parts[1].split(':').collect();

    if date_parts.len() != 3 || time_parts.len() < 2 {
        return Err(format!("Invalid timestamp format: {}", s));
    }

    let year: i64 = date_parts[0]
        .parse()
        .map_err(|_| "Invalid year")?;
    let month: i64 = date_parts[1]
        .parse()
        .map_err(|_| "Invalid month")?;
    let day: i64 = date_parts[2]
        .parse()
        .map_err(|_| "Invalid day")?;
    let hour: i64 = time_parts[0]
        .parse()
        .map_err(|_| "Invalid hour")?;
    let minute: i64 = time_parts[1]
        .parse()
        .map_err(|_| "Invalid minute")?;
    let second: i64 = time_parts[2]
        .split('.')
        .next()
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    // Simple approximation of Unix timestamp
    // This is a simplified calculation, good enough for expiration checking
    let days_since_epoch = (year - 1970) * 365
        + (year - 1969) / 4  // leap years
        + day_of_year(month, day);

    let seconds = days_since_epoch * 86400 + hour * 3600 + minute * 60 + second;

    Ok(seconds as u64)
}

fn day_of_year(month: i64, day: i64) -> i64 {
    let days_before_month = [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334];
    if month >= 1 && month <= 12 {
        days_before_month[(month - 1) as usize] + day - 1
    } else {
        day - 1
    }
}

/// Output export commands with MFA temporary credentials
pub fn output_export_commands_with_mfa(profile: &Profile, mfa_creds: &CachedCredentials) {
    // Export temporary credentials directly
    println!(
        "export AWS_ACCESS_KEY_ID='{}'",
        mfa_creds.access_key_id
    );
    println!(
        "export AWS_SECRET_ACCESS_KEY='{}'",
        mfa_creds.secret_access_key
    );
    println!(
        "export AWS_SESSION_TOKEN='{}'",
        mfa_creds.session_token
    );
    // Also set profile for reference
    println!("export AWS_PROFILE='{}'", profile.raw_name);

    // Clear any file overrides since we're using direct credentials
    println!("unset AWS_CONFIG_FILE");
    println!("unset AWS_SHARED_CREDENTIALS_FILE");
}
