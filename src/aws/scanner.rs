use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use console::style;

use super::parser::parse_ini_file;
use super::profile::Profile;
use crate::cache::{CachedProfile, ProfileCache};

/// Scan ~/.aws/ directory for all profiles across all credentials/config files
pub fn scan_profiles() -> std::io::Result<Vec<Profile>> {
    scan_profiles_internal(false)
}

/// Scan ~/.aws/ directory with verbose output
pub fn scan_profiles_verbose() -> std::io::Result<Vec<Profile>> {
    scan_profiles_internal(true)
}

fn scan_profiles_internal(verbose: bool) -> std::io::Result<Vec<Profile>> {
    let aws_dir = get_aws_dir()?;

    if verbose {
        eprintln!(
            "{} Scanning AWS directory: {}",
            style("[verbose]").dim(),
            style(aws_dir.display()).cyan()
        );
    }

    if !aws_dir.exists() {
        return Ok(Vec::new());
    }

    // Try to use cache
    if let Some(cache) = ProfileCache::load() {
        if cache.is_valid(&aws_dir) {
            if verbose {
                eprintln!(
                    "{} Using cached profiles ({} profiles)",
                    style("[verbose]").dim(),
                    cache.profiles.len()
                );
            }
            return Ok(cache.profiles.into_iter().map(Profile::from).collect());
        } else if verbose {
            eprintln!("{} Cache invalid, rescanning", style("[verbose]").dim());
        }
    }

    // Step 1: Scan ALL credentials files and build a map of raw_name -> credentials_file
    let credentials_map = build_credentials_map(&aws_dir, verbose)?;

    // Step 2: Scan config files and build profiles
    let mut profiles: HashMap<String, Profile> = HashMap::new();

    let entries = fs::read_dir(&aws_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        // Process config files
        if filename == "config" || filename.starts_with("config_") {
            if verbose {
                eprintln!(
                    "{} Processing config file: {}",
                    style("[verbose]").dim(),
                    style(path.display()).cyan()
                );
            }
            let project = extract_project_name(filename, "config");
            process_config_file(&path, project, &credentials_map, &mut profiles)?;
        }
    }

    // Step 3: Add any credentials-only profiles (profiles in credentials but not in config)
    for (raw_name, cred_info) in &credentials_map {
        let profile_key = make_profile_key(raw_name, &cred_info.project);

        profiles.entry(profile_key).or_insert_with(|| {
            Profile::new(
                raw_name.clone(),
                cred_info.project.clone(),
                None,
                Some(cred_info.path.clone()),
                None,
            )
        });
    }

    let mut result: Vec<Profile> = profiles.into_values().collect();
    result.sort_by(|a, b| a.name.cmp(&b.name));

    if verbose {
        eprintln!(
            "{} Found {} profiles",
            style("[verbose]").dim(),
            style(result.len()).cyan()
        );
    }

    // Save to cache
    let cached_profiles: Vec<CachedProfile> = result.iter().map(CachedProfile::from).collect();
    if let Ok(cache) = ProfileCache::create(cached_profiles, &aws_dir) {
        let _ = cache.save(); // Ignore cache save errors
    }

    Ok(result)
}

#[derive(Clone)]
struct CredentialInfo {
    path: PathBuf,
    project: Option<String>,
}

/// Build a map of raw profile name -> credentials file info
fn build_credentials_map(
    aws_dir: &PathBuf,
    verbose: bool,
) -> std::io::Result<HashMap<String, CredentialInfo>> {
    let mut map: HashMap<String, CredentialInfo> = HashMap::new();

    let entries = fs::read_dir(aws_dir)?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
        };

        if filename == "credentials" || filename.starts_with("credentials_") {
            if verbose {
                eprintln!(
                    "{} Processing credentials file: {}",
                    style("[verbose]").dim(),
                    style(path.display()).cyan()
                );
            }
            let project = extract_project_name(filename, "credentials");
            let sections = parse_ini_file(&path, false)?;

            for section_name in sections.keys() {
                // Only store if not already present (first match wins)
                // This prioritizes default credentials file over project-specific ones
                map.entry(section_name.clone())
                    .or_insert_with(|| CredentialInfo {
                        path: path.clone(),
                        project: project.clone(),
                    });
            }
        }
    }

    Ok(map)
}

fn get_aws_dir() -> std::io::Result<PathBuf> {
    dirs::home_dir()
        .map(|h| h.join(".aws"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Could not find home directory"))
}

fn extract_project_name(filename: &str, prefix: &str) -> Option<String> {
    if filename == prefix {
        None
    } else {
        filename.strip_prefix(&format!("{}_", prefix)).map(String::from)
    }
}

fn process_config_file(
    path: &PathBuf,
    project: Option<String>,
    credentials_map: &HashMap<String, CredentialInfo>,
    profiles: &mut HashMap<String, Profile>,
) -> std::io::Result<()> {
    let sections = parse_ini_file(path, true)?;

    for (section_name, section) in sections.iter() {
        let profile_key = make_profile_key(section_name, &project);
        let region = section.values.get("region").cloned();

        // Parse MFA-related fields
        let mfa_serial = section.values.get("mfa_serial").cloned();
        let source_profile = section.values.get("source_profile").cloned();
        let role_arn = section.values.get("role_arn").cloned();

        // Find credentials file by raw profile name (not by project)
        let credentials_file = credentials_map.get(section_name).map(|c| c.path.clone());

        profiles
            .entry(profile_key)
            .and_modify(|p| {
                p.config_file = Some(path.clone());
                if region.is_some() {
                    p.region = region.clone();
                }
                if credentials_file.is_some() {
                    p.credentials_file = credentials_file.clone();
                }
                // Update MFA fields
                if mfa_serial.is_some() {
                    p.mfa_serial = mfa_serial.clone();
                }
                if source_profile.is_some() {
                    p.source_profile = source_profile.clone();
                }
                if role_arn.is_some() {
                    p.role_arn = role_arn.clone();
                }
            })
            .or_insert_with(|| {
                Profile::new_with_mfa(
                    section_name.clone(),
                    project.clone(),
                    region.clone(),
                    credentials_file.clone(),
                    Some(path.clone()),
                    mfa_serial.clone(),
                    source_profile.clone(),
                    role_arn.clone(),
                )
            });
    }

    Ok(())
}

fn make_profile_key(name: &str, project: &Option<String>) -> String {
    match project {
        Some(proj) => format!("{}/{}", proj, name),
        None => name.to_string(),
    }
}

/// Find a profile by name
pub fn find_profile(name: &str) -> std::io::Result<Option<Profile>> {
    let profiles = scan_profiles()?;
    Ok(profiles.into_iter().find(|p| p.name == name))
}

/// Get the current profile from environment
#[allow(dead_code)]
pub fn get_current_profile() -> Option<String> {
    std::env::var("AWS_PROFILE").ok()
}
