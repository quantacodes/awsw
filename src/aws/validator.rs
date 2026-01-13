use std::time::Duration;

use console::style;
use tokio::process::Command;
use tokio::time::timeout;

use super::Profile;

const DEFAULT_TIMEOUT_SECS: u64 = 15;
const MAX_RETRIES: u32 = 3;

/// Result of credential validation
#[derive(Debug)]
#[allow(dead_code)]
pub struct ValidationResult {
    pub account: String,
    pub arn: String,
    pub user_id: String,
}

/// Configuration for validation
pub struct ValidationConfig {
    pub timeout_secs: u64,
    pub retries: u32,
    pub verbose: bool,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            retries: MAX_RETRIES,
            verbose: false,
        }
    }
}

/// Known AWS CLI error types
#[derive(Debug)]
pub enum AwsErrorKind {
    InvalidSignature,
    ExpiredToken,
    ExpiredCredentials,
    AccessDenied,
    NoCredentialsFound,
    NetworkError,
    CliNotInstalled,
    Timeout,
    Unknown(String),
}

impl AwsErrorKind {
    fn user_message(&self) -> String {
        match self {
            Self::InvalidSignature => {
                "Invalid AWS credentials. Check your access key ID and secret access key.".into()
            }
            Self::ExpiredToken => {
                "Session token has expired. Run 'aws sso login' or refresh your credentials.".into()
            }
            Self::ExpiredCredentials => {
                "Credentials have expired. Refresh your credentials or run 'aws configure'.".into()
            }
            Self::AccessDenied => {
                "Access denied. Your credentials may not have permission for sts:GetCallerIdentity."
                    .into()
            }
            Self::NoCredentialsFound => {
                "No credentials found for this profile. Run 'aws configure --profile <name>'.".into()
            }
            Self::NetworkError => "Network error. Check your internet connection.".into(),
            Self::CliNotInstalled => {
                "AWS CLI not found. Install it from https://docs.aws.amazon.com/cli/latest/userguide/install-cliv2.html".into()
            }
            Self::Timeout => "Validation timed out. The AWS API may be slow or unreachable.".into(),
            Self::Unknown(msg) => msg.clone(),
        }
    }

    fn is_retryable(&self) -> bool {
        matches!(self, Self::NetworkError | Self::Timeout)
    }
}

fn parse_aws_error(stderr: &str) -> AwsErrorKind {
    let lower = stderr.to_lowercase();

    if lower.contains("invalidsignatureexception") || lower.contains("signature") {
        AwsErrorKind::InvalidSignature
    } else if lower.contains("expiredtoken") || lower.contains("token has expired") {
        AwsErrorKind::ExpiredToken
    } else if lower.contains("expired") {
        AwsErrorKind::ExpiredCredentials
    } else if lower.contains("accessdenied") || lower.contains("access denied") {
        AwsErrorKind::AccessDenied
    } else if lower.contains("unable to locate credentials") || lower.contains("no credentials") {
        AwsErrorKind::NoCredentialsFound
    } else if lower.contains("could not connect")
        || lower.contains("network")
        || lower.contains("connection")
    {
        AwsErrorKind::NetworkError
    } else {
        AwsErrorKind::Unknown(stderr.trim().to_string())
    }
}

/// Validate AWS credentials (sync wrapper for backward compatibility)
pub fn validate_credentials(profile: &Profile) -> Result<ValidationResult, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;
    rt.block_on(validate_credentials_async(profile, &ValidationConfig::default()))
}

/// Validate AWS credentials with verbose output (sync wrapper)
pub fn validate_credentials_verbose(profile: &Profile) -> Result<ValidationResult, String> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;
    let config = ValidationConfig {
        verbose: true,
        ..Default::default()
    };
    rt.block_on(validate_credentials_async(profile, &config))
}

/// Validate AWS credentials asynchronously with timeout and retry
pub async fn validate_credentials_async(
    profile: &Profile,
    config: &ValidationConfig,
) -> Result<ValidationResult, String> {
    let mut last_error = AwsErrorKind::Unknown("No attempts made".into());
    let mut delay = Duration::from_millis(500);

    for attempt in 0..config.retries {
        if attempt > 0 {
            if config.verbose {
                eprintln!(
                    "{} Retry attempt {} after {:?}",
                    style("[verbose]").dim(),
                    attempt + 1,
                    delay
                );
            }
            tokio::time::sleep(delay).await;
            delay *= 2; // Exponential backoff
        }

        match validate_with_timeout(profile, config).await {
            Ok(result) => return Ok(result),
            Err(error_kind) => {
                // Don't retry for non-retryable errors
                if !error_kind.is_retryable() {
                    return Err(error_kind.user_message());
                }
                last_error = error_kind;
            }
        }
    }

    Err(last_error.user_message())
}

async fn validate_with_timeout(
    profile: &Profile,
    config: &ValidationConfig,
) -> Result<ValidationResult, AwsErrorKind> {
    let duration = Duration::from_secs(config.timeout_secs);

    match timeout(duration, run_validation(profile, config.verbose)).await {
        Ok(result) => result,
        Err(_) => {
            if config.verbose {
                eprintln!(
                    "{} Timeout after {} seconds",
                    style("[verbose]").dim(),
                    config.timeout_secs
                );
            }
            Err(AwsErrorKind::Timeout)
        }
    }
}

async fn run_validation(profile: &Profile, verbose: bool) -> Result<ValidationResult, AwsErrorKind> {
    let mut cmd = Command::new("aws");
    cmd.arg("sts")
        .arg("get-caller-identity")
        .arg("--output")
        .arg("json");

    // Set the profile
    cmd.env("AWS_PROFILE", &profile.raw_name);

    // Set config/credentials files if from a project
    if let Some(ref config_file) = profile.config_file {
        cmd.env("AWS_CONFIG_FILE", config_file);
    }
    if let Some(ref creds_file) = profile.credentials_file {
        cmd.env("AWS_SHARED_CREDENTIALS_FILE", creds_file);
    }

    if verbose {
        eprintln!(
            "{} Running: aws sts get-caller-identity",
            style("[verbose]").dim()
        );
        eprintln!(
            "{} AWS_PROFILE={}",
            style("[verbose]").dim(),
            style(&profile.raw_name).cyan()
        );
    }

    let output = cmd.output().await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            AwsErrorKind::CliNotInstalled
        } else {
            AwsErrorKind::Unknown(format!("Failed to run aws CLI: {}", e))
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if verbose {
            eprintln!(
                "{} AWS CLI stderr: {}",
                style("[verbose]").dim(),
                style(stderr.trim()).red()
            );
        }
        return Err(parse_aws_error(&stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    if verbose {
        eprintln!("{} Validation successful", style("[verbose]").dim());
    }

    parse_caller_identity(&stdout).map_err(|e| AwsErrorKind::Unknown(e))
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
