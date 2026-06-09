# awsp

AWS CLI sessions manager. No hustle. Log in once - use everywhere.

`awsp` reads SSO profiles from your AWS config, opens a terminal picker, and
keeps the selected profile available across shell sessions. It does not store
AWS credentials. Login, logout, and token caching stay owned by the AWS CLI.

## Install

Install from the Homebrew tap:

```sh
brew tap arseniizakharov/formulae
brew install awsp
awsp setup zsh   # or: awsp setup bash
```

Restart the shell after setup, or source the generated integration immediately:

```sh
source "$HOME/.config/awsp/shell/awsp.sh"
```

The picker is built in. AWS CLI is required for commands that call AWS, such as
`login`, `logout`, `whoami`, and `status --verify`.

## Use

```sh
awsp                         # pick and activate an SSO profile
awsp prod                    # activate the unique match, or choose from matches
awsp --table                 # use the compact table picker
awsp profiles                # list complete SSO profiles
awsp status                  # show local SSO cache status
awsp login prod-admin        # run aws sso login for a profile
awsp off                     # clear AWS_PROFILE in this shell
awsp exec prod-admin -- aws s3 ls
awsp team login --app-url https://team.example.com
awsp team login --app-url https://team.example.com --browser-capture
awsp doctor                  # check AWS CLI, config, and profile diagnostics
```

For TEAM login, `--app-url` discovers the deployed app config. If TEAM's
Cognito app client has a localhost callback registered, pass it with
`--redirect-uri` and `awsp` will capture the browser redirect automatically.
If you cannot change Cognito callbacks, use `--browser-capture` to read the
existing TEAM web-app callback from browser navigation.

`awsp` reads `AWS_CONFIG_FILE` when set, otherwise `~/.aws/config`. Run
`aws configure sso` first; incomplete SSO profiles are hidden from normal
commands and reported by `awsp doctor`.

## What It Does

- Activates profiles through shell integration, so the current terminal gets
  `AWS_PROFILE`, `AWS_REGION`, and `AWS_SDK_LOAD_CONFIG`.
- Supports modern `sso_session` profiles and legacy inline SSO profiles.
- Reads AWS CLI SSO cache files locally to show whether sessions are valid,
  expiring, expired, or unknown.
- Keeps only non-secret selection state in `~/.config/awsp/state.json`.
- Can request TEAM temporary elevated access when that workflow is configured.

## More

- Homebrew release workflow: [docs/homebrew.md](docs/homebrew.md)
- Security model and reporting: [SECURITY.md](SECURITY.md)
