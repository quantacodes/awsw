use crate::aws::{scan_profiles, Profile};
use console::style;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let profiles = scan_profiles()?;

    if profiles.is_empty() {
        eprintln!("No AWS profiles found in ~/.aws/");
        return Ok(());
    }

    let current = std::env::var("AWS_PROFILE").ok();

    for profile in profiles {
        print_profile(&profile, current.as_deref());
    }

    Ok(())
}

fn print_profile(profile: &Profile, current: Option<&str>) {
    let is_current = current.map(|c| c == profile.raw_name).unwrap_or(false)
        || (current.is_none() && profile.raw_name == "default");

    let marker = if is_current {
        style("*").green().bold().to_string()
    } else {
        " ".to_string()
    };

    let name = if is_current {
        style(&profile.name).green().bold().to_string()
    } else {
        profile.name.clone()
    };

    let region = profile
        .region
        .as_ref()
        .map(|r| style(format!("({})", r)).dim().to_string())
        .unwrap_or_default();

    // Output to stderr so it doesn't get eval'd by shell
    eprintln!("{} {} {}", marker, name, region);
}
