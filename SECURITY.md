# Security Policy

`awsp` is shell-adjacent software that affects AWS identity selection. Treat bugs here as potentially security-sensitive even though the tool must not store AWS credentials.

## Supported Versions

Security fixes are provided for the latest released version and the current `main` branch while the project is pre-1.0.

## Reporting a Vulnerability

Use GitHub private vulnerability reporting when it is enabled for the repository. If private reporting is unavailable, open a public issue that says a security report exists, but do not include exploit details.

Do not paste real AWS credentials, AWS SSO cache files, unredacted account IDs, start URLs, or terminal output containing tokens into public issues.

Useful security reports include:

- shell injection through profile names or generated shell output
- any case where `awsp __shell` writes non-shell-code to stdout
- reading, logging, modifying, or exfiltrating AWS SSO cache data
- storing raw `AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`, or `AWS_SESSION_TOKEN`
- unsafe rc-file modification behavior
- release, Homebrew, or GitHub Actions supply-chain risks

## Security Invariants

- `awsp` stores only non-secret local state in `~/.config/awsp/state.json`.
- AWS login and token cache ownership stays with the AWS CLI.
- `awsp` does not modify AWS SSO cache files.
- Shell mode reserves stdout for shell-safe code only; user-facing text goes to stderr or the terminal.
- Activation exports only `AWS_PROFILE` and `AWS_SDK_LOAD_CONFIG`, and unsets credential environment variables.
