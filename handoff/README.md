# awsp — Codex handoff

## Read first

1. **`AGENTS.md`** — what you're building, stack pins, ground rules.
2. **`SPEC.md`** — per-screen visual + behavioural spec.
3. **`PALETTE.md`** — exact color values.
4. **`reference/AWSP CLI.html`** — open in a browser. The section titled
   *"Tuned to your terminal"* contains the canonical mocks (F, G, I, J).

## Deliver

A Rust crate publishable as `awsp` (binary name `awsp`) implementing:

- Interactive profile picker (screen F)
- `--table` variant (screen G)
- `awsp status` and ambiguous-match disambiguation (screen I)
- Switch-result feedback (screen J)
- Typo suggestion (screen K)
- SSO device-authorization flow (screen L)
- `--emit-env` for shell-function integration

PR target: a single PR that creates the crate from scratch in a new
repo, with CI running `fmt`, `clippy`, `test` on Linux + macOS.

## Out of scope

H (tree grouped by SSO org) is **not** part of this handoff — it was
explored and dropped.

The earlier "Variant B (gum-style)" handoff is superseded by this one.

## Sample fixtures

`reference/profiles.jsx` has 10 sample profiles. Mirror them in
`tests/config_fixtures/sample.config`:

```ini
[sso-session acme-corp]
sso_start_url = https://acme-corp.awsapps.com/start
sso_region    = us-east-1

[profile acme-prod-admin]
sso_session     = acme-corp
sso_account_id  = 682471093210
sso_role_name   = AdministratorAccess
region          = us-east-1

[profile acme-prod-readonly]
sso_session     = acme-corp
sso_account_id  = 682471093210
sso_role_name   = ReadOnlyAccess
region          = us-east-1

# ... (see reference/profiles.jsx for the full list)
```

## Acceptance

- All snapshot tests pass.
- `awsp` with no args renders screen F pixel-for-pixel against the mock
  (colors, glyphs, spacing).
- Manual smoke test:
  - `awsp` opens picker, `3` switches to profile #3.
  - `awsp prod` shows screen I.
  - `awsp xyzzy` shows screen K.
  - On a profile with no cached SSO token, `awsp <name>` shows screen L,
    opens a browser, polls, then re-attempts the switch.
  - After successful switch, `awsp status` prints the new active profile.
- `eval "$(awsp --emit-env acme-staging-dev)"` sets `AWS_PROFILE` and
  `AWS_REGION` correctly in bash and zsh.
