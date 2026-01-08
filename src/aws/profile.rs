use std::path::PathBuf;

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
