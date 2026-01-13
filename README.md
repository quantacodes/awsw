# awsw - AWS Profile Switcher

Fast, ergonomic AWS profile management for the terminal.

## Features

- **Fuzzy search** - quickly filter through profiles
- **Multi-file support** - works with `credentials_*` and `config_*` files
- **Credential validation** - validates with `aws sts get-caller-identity` on switch
- **Shell integration** - seamless bash/zsh/fish support
- **Project prefixes** - profiles from `config_work` appear as `work/profile-name`

## Requirements

- macOS (Linux support coming soon)
- Rust toolchain (for building)
- AWS CLI (for credential validation)

## Installation (macOS)

### Homebrew (Recommended)

```bash
brew tap quantacodes/tap
brew install awsw
```

### Build from source

```bash
# Clone the repository
git clone https://github.com/quantacodes/awsw.git
cd awsw

# Build release binary
cargo build --release

# Copy to PATH
sudo cp target/release/awsw /usr/local/bin/
```

### Shell Setup

Add to your shell config file:

**Zsh** (`~/.zshrc`):
```bash
eval "$(awsw init zsh)"
```

**Bash** (`~/.bashrc`):
```bash
eval "$(awsw init bash)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
awsw init fish | source
```

Then reload your shell:
```bash
source ~/.zshrc  # or ~/.bashrc
```

## Usage

```bash
awsw                              # Interactive profile selector with fuzzy search
awsw list                         # List all profiles
awsw use <profile>                # Switch to profile (validates credentials)
awsw use <profile> --skip-validate  # Switch without validation
awsw current                      # Show currently active profile
awsw unset                        # Clear profile, return to default
awsw init <shell>                 # Output shell integration function
```

## Multi-File Support

awsw scans `~/.aws/` for all credential and config files:

```
~/.aws/
├── credentials           # profiles: default, dev
├── credentials_work      # profiles: work/staging, work/prod
├── config
├── config_work
```

Profiles from `config_work` or `credentials_work` are prefixed with `work/`.

## Example

```
$ awsw
? Select AWS profile
  default
  work/dev (us-west-2)
> work/prod (eu-west-1)
  personal/sandbox (ap-south-1)

Validating credentials...
Switched to work/prod (Account: 123456789012) ARN: arn:aws:iam::123456789012:user/admin
```

## TODO

- [ ] Linux support
- [x] Homebrew formula
- [ ] Pre-built binaries for releases

## License

MIT
