## Summary

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo test --locked`
- [ ] `cargo clippy --locked -- -D warnings`

## Security Checklist

- [ ] Does not store, log, or copy raw AWS credentials.
- [ ] Does not modify AWS SSO cache files.
- [ ] Keeps `awsp __shell` stdout reserved for shell-safe code only.
- [ ] Shell-emitted values are quoted safely.
- [ ] Redacts sensitive AWS details in tests, docs, and examples.
