use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

const CACHE_FILE: &str = ".awsp_cache";

#[derive(Serialize, Deserialize)]
pub struct ProfileCache {
    /// Map of file path -> mtime (as unix timestamp)
    pub file_mtimes: HashMap<PathBuf, u64>,
    /// Cached profiles
    pub profiles: Vec<CachedProfile>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedProfile {
    pub name: String,
    pub raw_name: String,
    pub region: Option<String>,
    pub project: Option<String>,
    pub credentials_file: Option<PathBuf>,
    pub config_file: Option<PathBuf>,
    pub mfa_serial: Option<String>,
    pub source_profile: Option<String>,
    pub role_arn: Option<String>,
}

impl ProfileCache {
    fn cache_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".aws").join(CACHE_FILE))
    }

    pub fn load() -> Option<Self> {
        let path = Self::cache_path()?;
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(path) = Self::cache_path() {
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }

    /// Check if cache is valid by comparing mtimes
    pub fn is_valid(&self, aws_dir: &PathBuf) -> bool {
        // Get current mtimes of all relevant files
        let current_mtimes = match Self::collect_file_mtimes(aws_dir) {
            Ok(m) => m,
            Err(_) => return false,
        };

        // Compare with cached mtimes
        if current_mtimes.len() != self.file_mtimes.len() {
            return false;
        }

        for (path, mtime) in &current_mtimes {
            match self.file_mtimes.get(path) {
                Some(cached_mtime) if cached_mtime == mtime => continue,
                _ => return false,
            }
        }

        true
    }

    pub fn collect_file_mtimes(aws_dir: &PathBuf) -> std::io::Result<HashMap<PathBuf, u64>> {
        let mut mtimes = HashMap::new();

        if !aws_dir.exists() {
            return Ok(mtimes);
        }

        for entry in fs::read_dir(aws_dir)?.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

            // Only track config and credentials files
            if filename == "config"
                || filename.starts_with("config_")
                || filename == "credentials"
                || filename.starts_with("credentials_")
            {
                if let Ok(metadata) = fs::metadata(&path) {
                    if let Ok(mtime) = metadata.modified() {
                        if let Ok(duration) = mtime.duration_since(SystemTime::UNIX_EPOCH) {
                            mtimes.insert(path, duration.as_secs());
                        }
                    }
                }
            }
        }

        Ok(mtimes)
    }

    pub fn create(profiles: Vec<CachedProfile>, aws_dir: &PathBuf) -> std::io::Result<Self> {
        let file_mtimes = Self::collect_file_mtimes(aws_dir)?;

        Ok(ProfileCache {
            file_mtimes,
            profiles,
        })
    }
}
