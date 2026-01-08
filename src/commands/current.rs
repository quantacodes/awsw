use console::style;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let current = std::env::var("AWS_PROFILE").ok();

    match current {
        Some(profile) => {
            eprintln!(
                "Current profile: {}",
                style(&profile).green().bold()
            );
        }
        None => {
            eprintln!(
                "Current profile: {} {}",
                style("default").yellow(),
                style("(AWS_PROFILE not set)").dim()
            );
        }
    }

    Ok(())
}
