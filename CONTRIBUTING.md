# Contributing

Thanks for helping improve `awsp`.

## Local Setup

Install Rust, AWS CLI, and `fzf`.

```sh
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```

## Design Constraints

- Do not store or copy raw AWS credentials.
- Do not read more from AWS SSO cache files than is needed for best-effort expiry status.
- Do not modify AWS config, credentials, or SSO cache files.
- Keep explicit profile arguments exact-match only.
- Keep `awsp __shell` stdout reserved for shell-safe code. Human text must go to stderr or `/dev/tty`.
- Shell-emitted values must be single-quote escaped.
- Region display is informational; do not export `AWS_REGION` or `AWS_DEFAULT_REGION` by default.

## Pull Requests

Open a focused PR with tests for behavior that affects parsing, shell output, state writes, or AWS command execution.

For reports and examples, redact AWS account IDs, SSO start URLs, and any cache/token data.
