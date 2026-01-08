use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// Parsed section from an INI file
#[derive(Debug, Default)]
pub struct Section {
    pub values: HashMap<String, String>,
}

/// Parse an INI file into sections
/// For credentials files: [name] -> section name is "name"
/// For config files: [profile name] -> section name is "name", [default] -> "default"
pub fn parse_ini_file(path: &Path, is_config: bool) -> std::io::Result<HashMap<String, Section>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);

    let mut sections: HashMap<String, Section> = HashMap::new();
    let mut current_section: Option<String> = None;

    for line in reader.lines() {
        let line = line?;
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            let section_name = &line[1..line.len() - 1];

            // For config files, strip "profile " prefix (except for "default")
            let name = if is_config && section_name.starts_with("profile ") {
                section_name.strip_prefix("profile ").unwrap().to_string()
            } else {
                section_name.to_string()
            };

            current_section = Some(name.clone());
            sections.entry(name).or_default();
            continue;
        }

        // Key-value pair
        if let Some(ref section) = current_section {
            if let Some((key, value)) = parse_key_value(line) {
                if let Some(sec) = sections.get_mut(section) {
                    sec.values.insert(key, value);
                }
            }
        }
    }

    Ok(sections)
}

fn parse_key_value(line: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() == 2 {
        let key = parts[0].trim().to_string();
        let value = parts[1].trim().to_string();
        Some((key, value))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_credentials_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[default]
aws_access_key_id = AKIAIOSFODNN7EXAMPLE
aws_secret_access_key = secret

[dev]
aws_access_key_id = AKIAI44QH8DHBEXAMPLE
aws_secret_access_key = devsecret"#
        )
        .unwrap();

        let sections = parse_ini_file(file.path(), false).unwrap();

        assert!(sections.contains_key("default"));
        assert!(sections.contains_key("dev"));
        assert_eq!(
            sections["default"].values.get("aws_access_key_id"),
            Some(&"AKIAIOSFODNN7EXAMPLE".to_string())
        );
    }

    #[test]
    fn test_parse_config_file() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(
            file,
            r#"[default]
region = us-east-1

[profile dev]
region = us-west-2

[profile prod]
region = eu-west-1"#
        )
        .unwrap();

        let sections = parse_ini_file(file.path(), true).unwrap();

        assert!(sections.contains_key("default"));
        assert!(sections.contains_key("dev"));
        assert!(sections.contains_key("prod"));
        assert_eq!(
            sections["dev"].values.get("region"),
            Some(&"us-west-2".to_string())
        );
    }
}
