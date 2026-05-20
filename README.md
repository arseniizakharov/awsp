# awsp

`awsp` is a Rust CLI for switching between AWS SSO profiles and carrying the selected profile across shell sessions without copying AWS credentials.

The tool stores only non-secret local state in `~/.config/awsp/state.json`. AWS SSO login and token caching remain owned by the AWS CLI.

## MVP behavior

- `awsp` opens a built-in terminal picker for complete AWS SSO profiles.
- `awsp --table` opens the compact table picker.
- `awsp <fragment>` switches when a fragment is unique, disambiguates when it is not, and suggests near profile names on typos.
- `awsp --emit-env <fragment>` prints shell-safe exports for shell-function integration.
- `awsp use <profile>` activates an exact profile name. `awsp activate <profile>` is an alias.
- `awsp login <profile>` runs `aws sso login --profile <profile>`.
- `awsp login-session <session>` runs `aws sso login --sso-session <session>`.
- `awsp off` unsets the active AWS profile for the current shell.
- `awsp exec <profile> -- <command>` runs one command with that profile.
- `awsp logout --all` runs `aws sso logout` and clears local awsp state.
- `awsp status` reads AWS CLI SSO cache files under `~/.aws/sso/cache` without network calls.
- `awsp status --json` prints the active profile status as a single JSON object.
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

`awsp setup` is a one-time install step. It writes the static shell integration to `~/.config/awsp/shell/awsp.sh` and adds a small source block to shell startup files. For zsh this is `~/.zshrc` and `~/.zprofile`; if `ZDOTDIR` is exported or set in a simple `~/.zshenv`, setup also updates `$ZDOTDIR/.zshrc` and `$ZDOTDIR/.zprofile`. For bash this is `~/.bashrc` plus the first existing login file among `~/.bash_profile`, `~/.bash_login`, and `~/.profile`; if none exists, `~/.bash_profile` is created. New terminal tabs do not run `awsp init`.

After integration is active, `awsp`, `awsp --table`, direct fragments like `awsp prod-readonly`, `awsp use`, `awsp activate`, `awsp off`, `awsp clear`, and `awsp restore` can update the current shell by evaluating shell-safe code emitted by the hidden `awsp __shell` command. Successful switches print a short confirmation to stderr.

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
export AWS_REGION='<profile-region>'
export AWS_SDK_LOAD_CONFIG='1'
```

Regions shown in the picker come from the profile `region`, then `[default]` `region`, then `unset`. A trailing `*` means the region was inherited from `[default]`. `AWSP_PROD=1` is exported only when the active profile name contains `prod`; otherwise it is unset.

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

The Homebrew formula installs the `awsp` binary only. The interactive picker is built in. AWS CLI is a runtime requirement only for commands that shell out to `aws`, such as SSO login, logout, `whoami`, and `status --verify`. `awsp doctor` reports whether `aws` is available and prints an install hint when it is missing.

## Homebrew Beta

The app repo is intended to live at `github.com/nomadsre/awsp`. The Homebrew tap should live at `github.com/nomadsre/homebrew-awsp`.

See `docs/homebrew.md` and `packaging/homebrew/awsp-beta.rb` for the beta formula workflow.

Once the tap repo contains `Formula/awsp-beta.rb`, install from another machine with:

```sh
brew install nomadsre/awsp/awsp-beta
```

On Apple Silicon macOS, Homebrew installs a prebuilt `awsp` binary. It does not install AWS CLI and does not modify `~/.zshrc`, `~/.bashrc`, or other shell startup files. Run `awsp setup zsh` or `awsp setup bash` once after install to add the shell hook.

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
