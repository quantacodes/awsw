use console::style;

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    // Output unset commands to stdout for shell to eval
    println!("unset AWS_PROFILE");
    println!("unset AWS_CONFIG_FILE");
    println!("unset AWS_SHARED_CREDENTIALS_FILE");

    // Status message to stderr
    eprintln!(
        "{}",
        style("Cleared AWS profile (using default)").green()
    );

    Ok(())
}
