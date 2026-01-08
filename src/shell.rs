/// Generate shell initialization script for the given shell type
pub fn generate_init_script(shell_type: &str) -> Result<String, Box<dyn std::error::Error>> {
    match shell_type.to_lowercase().as_str() {
        "bash" => Ok(generate_bash()),
        "zsh" => Ok(generate_zsh()),
        "fish" => Ok(generate_fish()),
        _ => Err(format!(
            "Unsupported shell: '{}'. Supported shells: bash, zsh, fish",
            shell_type
        )
        .into()),
    }
}

fn generate_bash() -> String {
    r#"# AWS Profile Switcher - add this to your ~/.bashrc
awsw() {
  local output
  output=$(awsw-bin "$@")
  local exit_code=$?

  # Only eval if there's output and command succeeded
  if [[ -n "$output" ]]; then
    eval "$output"
  fi

  return $exit_code
}
"#
    .to_string()
}

fn generate_zsh() -> String {
    r#"# AWS Profile Switcher - add this to your ~/.zshrc
awsw() {
  local output
  output=$(awsw-bin "$@")
  local exit_code=$?

  # Only eval if there's output and command succeeded
  if [[ -n "$output" ]]; then
    eval "$output"
  fi

  return $exit_code
}
"#
    .to_string()
}

fn generate_fish() -> String {
    r#"# AWS Profile Switcher - add this to your ~/.config/fish/config.fish
function awsw
  set -l output (awsw-bin $argv)
  set -l exit_code $status

  # Only source if there's output
  if test -n "$output"
    echo "$output" | source
  end

  return $exit_code
end
"#
    .to_string()
}
