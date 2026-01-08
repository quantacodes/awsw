use crate::shell;

pub fn run(shell_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let output = shell::generate_init_script(shell_type)?;
    println!("{}", output);
    Ok(())
}
