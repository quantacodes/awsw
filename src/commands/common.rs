use crate::aws::Profile;

/// Output shell export commands to stdout for shell to eval
pub fn output_export_commands(profile: &Profile) {
    println!("export AWS_PROFILE='{}'", profile.raw_name);

    if let Some(ref config_file) = profile.config_file {
        println!("export AWS_CONFIG_FILE='{}'", config_file.display());
    } else {
        println!("unset AWS_CONFIG_FILE");
    }

    if let Some(ref creds_file) = profile.credentials_file {
        println!(
            "export AWS_SHARED_CREDENTIALS_FILE='{}'",
            creds_file.display()
        );
    } else {
        println!("unset AWS_SHARED_CREDENTIALS_FILE");
    }
}
