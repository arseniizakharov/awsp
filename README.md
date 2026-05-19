# awsp

`awsp` is a Rust CLI for switching between AWS SSO profiles and carrying the selected profile across shell sessions without copying AWS credentials.

The tool stores only non-secret local state in `~/.config/awsp/state.json`. AWS SSO login and token caching remain owned by the AWS CLI.

## MVP behavior

- `awsp` opens an `fzf` picker for complete AWS SSO profiles.
- `awsp use <profile>` activates an exact profile name. `awsp activate <profile>` is an alias.
- `awsp login <profile>` runs `aws sso login --profile <profile>`.
- `awsp login-session <session>` runs `aws sso login --sso-session <session>`.
- `awsp off` unsets the active AWS profile for the current shell.
- `awsp exec <profile> -- <command>` runs one command with that profile.
- `awsp logout --all` runs `aws sso logout` and clears local awsp state.
- `awsp status` reads AWS CLI SSO cache files under `~/.aws/sso/cache` without network calls.
- `awsp status --verify <profile>` calls `aws sts get-caller-identity`.
- `awsp current` reports local env/state only.
- `awsp whoami` calls AWS STS for the active profile.
- `awsp doctor` checks dependencies and malformed SSO config.
- `awsp profiles` is an alias for `awsp list`.

## Shell integration

For zsh:

```sh
awsp setup zsh
```

For bash:

```sh
awsp setup bash
```

`awsp setup` is a one-time install step. It writes the static shell integration to `~/.config/awsp/shell/awsp.sh` and adds a small source block to shell startup files. For zsh this is `~/.zshrc`. For bash this is `~/.bashrc` plus the first existing login file among `~/.bash_profile`, `~/.bash_login`, and `~/.profile`; if none exists, `~/.bash_profile` is created. New terminal tabs do not run `awsp init`.

After integration is active, `awsp`, `awsp use`, `awsp activate`, `awsp off`, `awsp clear`, and `awsp restore` can update the current shell by evaluating shell-safe code emitted by the hidden `awsp __shell` command. Successful switches print a short confirmation to stderr.

If `awsp` resolves to the binary instead of the shell function, activation commands stop before opening the picker because a child process cannot export `AWS_PROFILE` into its parent shell. Check with:

```sh
type awsp
```

Expected output should say that `awsp` is a shell function. If it points at the Homebrew binary, restart the shell or run:

```sh
source "$HOME/.config/awsp/shell/awsp.sh"
```

Activation exports:

```sh
unset AWS_ACCESS_KEY_ID
unset AWS_SECRET_ACCESS_KEY
unset AWS_SESSION_TOKEN
unset AWS_SESSION_EXPIRATION
export AWS_PROFILE='<profile>'
export AWS_SDK_LOAD_CONFIG='1'
```

Region variables are not exported. Regions shown in the picker come from the profile `region`, then `[default]` `region`, then `unset`. A trailing `*` means the region was inherited from `[default]`.

## First run

Running plain `awsp` without shell integration prompts to install an rc-file hook. The command writes `~/.config/awsp/shell/awsp.sh` and can append:

```sh
# >>> awsp shell integration >>>
if [ -r "$HOME/.config/awsp/shell/awsp.sh" ]; then
  . "$HOME/.config/awsp/shell/awsp.sh"
fi
# <<< awsp shell integration <<<
```

A child process cannot mutate the parent shell, so the current shell must be restarted after installing the hook. To enable it immediately in the current shell:

```sh
source ~/.config/awsp/shell/awsp.sh
```

## Dependencies

The intended Homebrew formula should declare:

```ruby
depends_on "awscli"
depends_on "fzf"
```

`fzf` is mandatory for interactive profile selection. Explicit profile commands such as `awsp use prod-admin` do not need `fzf`.

## Homebrew Beta

The app repo is intended to live at `github.com/nomadsre/awsp`. The Homebrew tap should live at `github.com/nomadsre/homebrew-awsp`.

See `docs/homebrew.md` and `packaging/homebrew/awsp-beta.rb` for the beta formula workflow.

Once the tap repo contains `Formula/awsp-beta.rb`, install from another machine with:

```sh
brew install nomadsre/awsp/awsp-beta
```

Homebrew installs the binary and dependencies only. It does not modify `~/.zshrc`, `~/.bashrc`, or other shell startup files. Run `awsp setup zsh` or `awsp setup bash` once after install to add the shell hook.

For latest `main` from the tap:

```sh
brew install --HEAD nomadsre/awsp/awsp-beta
```

## Security

`awsp` is intentionally not a credential store. It delegates SSO login and token cache ownership to the AWS CLI, stores only non-secret selection state, and reserves shell-mode stdout for shell-safe code only.

See `SECURITY.md` before reporting shell injection, credential handling, SSO cache, or release supply-chain issues.

## AWS config support

`awsp` reads `AWS_CONFIG_FILE` when set, otherwise `~/.aws/config`.

Modern SSO profile:

```ini
[profile prod-admin]
sso_session = corp
sso_account_id = 123456789012
sso_role_name = AdministratorAccess
region = eu-central-1

[sso-session corp]
sso_start_url = https://example.awsapps.com/start
sso_region = us-east-1
```

Legacy SSO profile:

```ini
[profile prod-admin]
sso_start_url = https://example.awsapps.com/start
sso_region = us-east-1
sso_account_id = 123456789012
sso_role_name = AdministratorAccess
```

Incomplete SSO profiles are hidden from normal commands and reported by `awsp doctor`.
