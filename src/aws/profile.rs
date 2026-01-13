use std::path::PathBuf;

use crate::cache::CachedProfile;

/// Represents an AWS profile with its source files and metadata
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Profile {
    /// Display name (e.g., "default", "work/prod")
    pub name: String,
    /// Original profile name in the file (without "profile " prefix)
    pub raw_name: String,
    /// Region if specified in config
    pub region: Option<String>,
    /// Project prefix (None for default files, Some("work") for credentials_work)
    pub project: Option<String>,
    /// Path to the credentials file
    pub credentials_file: Option<PathBuf>,
    /// Path to the config file
    pub config_file: Option<PathBuf>,
    /// MFA device serial number (ARN)
    pub mfa_serial: Option<String>,
    /// Source profile for role assumption
    pub source_profile: Option<String>,
    /// Role ARN to assume
    pub role_arn: Option<String>,
}

impl From<CachedProfile> for Profile {
    fn from(cp: CachedProfile) -> Self {
        Profile {
            name: cp.name,
            raw_name: cp.raw_name,
            region: cp.region,
            project: cp.project,
            credentials_file: cp.credentials_file,
            config_file: cp.config_file,
            mfa_serial: cp.mfa_serial,
            source_profile: cp.source_profile,
            role_arn: cp.role_arn,
        }
    }
}

impl From<&Profile> for CachedProfile {
    fn from(p: &Profile) -> Self {
        CachedProfile {
            name: p.name.clone(),
            raw_name: p.raw_name.clone(),
            region: p.region.clone(),
            project: p.project.clone(),
            credentials_file: p.credentials_file.clone(),
            config_file: p.config_file.clone(),
            mfa_serial: p.mfa_serial.clone(),
            source_profile: p.source_profile.clone(),
            role_arn: p.role_arn.clone(),
        }
    }
}

impl Profile {
    /// Check if this profile requires MFA authentication
    pub fn requires_mfa(&self) -> bool {
        self.mfa_serial.is_some() && (self.source_profile.is_some() || self.role_arn.is_some())
    }
}

#[allow(dead_code)]
impl Profile {
    pub fn new(
        raw_name: String,
        project: Option<String>,
        region: Option<String>,
        credentials_file: Option<PathBuf>,
        config_file: Option<PathBuf>,
    ) -> Self {
        Self::new_with_mfa(
            raw_name,
            project,
            region,
            credentials_file,
            config_file,
            None,
            None,
            None,
        )
    }

    pub fn new_with_mfa(
        raw_name: String,
        project: Option<String>,
        region: Option<String>,
        credentials_file: Option<PathBuf>,
        config_file: Option<PathBuf>,
        mfa_serial: Option<String>,
        source_profile: Option<String>,
        role_arn: Option<String>,
    ) -> Self {
        let name = match &project {
            Some(proj) => format!("{}/{}", proj, raw_name),
            None => raw_name.clone(),
        };

        Self {
            name,
            raw_name,
            region,
            project,
            credentials_file,
            config_file,
            mfa_serial,
            source_profile,
            role_arn,
        }
    }

    /// Returns display string for the profile (name with region)
    pub fn display(&self) -> String {
        match &self.region {
            Some(r) => format!("{} ({})", self.name, r),
            None => self.name.clone(),
        }
    }

    /// Returns the AWS_PROFILE value to set
    pub fn aws_profile_value(&self) -> &str {
        &self.raw_name
    }
}
