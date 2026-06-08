# AGENTS.md — awsp

This file is the entry point for coding agents working on `awsp`.
Read this end-to-end before writing code.

## What awsp is

`awsp` is a small Rust CLI that switches between AWS SSO profiles defined
in `~/.aws/config`. Three modes of operation:

1. **Interactive picker** — `awsp` with no args. Renders a profile list
   in the current terminal (no alternate-screen), lets the user pick one
   with `↑↓` / number keys / fuzzy filter, prints an `export` block on
   selection.
2. **Direct switch** — `awsp <fragment>`. If exactly one profile matches,
   switch immediately. If multiple match, show a numbered disambiguation
   list (Screen I in this handoff). If none match, suggest by Levenshtein
   (Screen K).
3. **Status** — `awsp status` prints one line summarising the active
   profile and session expiry. Machine-readable variant: `awsp status --json`.

The binary cannot mutate the parent shell's environment by itself. Users
install a shell function (`awsp() { eval "$(command awsp --emit-env "$@")"; }`)
and the binary emits `export AWS_PROFILE=...; export AWS_REGION=...` on
stdout. Everything else (UI, errors, prompts) goes to **stderr**.

## Repository layout (target)

```
src/
  main.rs              # arg parsing, dispatch
  cli.rs               # clap definitions
  config.rs            # parse ~/.aws/config → Vec<Profile>
  sso.rs               # SSO token cache, expiry detection, device-flow login
  ui/
    mod.rs             # public render entry points
    palette.rs         # Tokyo Night palette (see Design Tokens)
    picker.rs          # Screen F — interactive numbered picker (primary)
    table.rs           # Screen G — table variant (--table flag)
    status.rs          # Screen I — `awsp status` + ambiguous list
    success.rs         # Screen J — switch-result line
    flows.rs           # device auth, did-you-mean, errors
    term.rs            # crossterm helpers (raw mode, redraw, key parsing)
  shell.rs             # `--emit-env` printer + shell-function helpers
tests/
  config_fixtures/     # sample ~/.aws/config files
  ui_snapshots/        # insta snapshots of rendered terminal output
```

## Stack — non-negotiable choices

| Concern | Pin to |
|---|---|
| MSRV | 1.75 |
| Terminal | `crossterm = "0.27"` |
| Fuzzy match | `nucleo-matcher = "0.3"` |
| Arg parsing | `clap = { version = "4", features = ["derive"] }` |
| AWS config parse | `rust-ini = "0.21"` (`aws-config` does NOT enumerate profiles) |
| SSO device flow | `aws-sdk-ssooidc = "1"`, `aws-sdk-sso = "1"` |
| Spinner | `indicatif = "0.17"` |
| Clipboard | `arboard = "3"` (only when user hits Ctrl-Y) |
| Open URL | `opener = "0.7"` |
| Fuzzy correction | `strsim = "0.11"` |
| Snapshot tests | `insta = "1"` |

Do not add other deps without justification in the PR description.

## Design fidelity

**High-fidelity.** Match the reference HTML in `reference/AWSP CLI.html`
(open it in a browser; scroll to "Tuned to your terminal"). Colors,
spacing, glyphs and column alignment are final.

The reference files are React + inline styles — they mock the **rendered
terminal output**, not the implementation. Reproduce the visual result in
Rust + crossterm. Do not try to port the React.

## Screens

See `SPEC.md` (sibling file) for full per-screen spec: row anatomy,
keybindings, scroll behavior, error states.

The screens you must implement:

| ID | Name | Trigger | File |
|---|---|---|---|
| F | Interactive picker | `awsp` (no args) | `ui/picker.rs` |
| G | Table variant | `awsp --table` | `ui/table.rs` |
| I | Status + ambiguous match | `awsp status` / `awsp <fragment>` matching multiple | `ui/status.rs` |
| J | Switch result | Any successful switch | `ui/success.rs` |
| K | Did-you-mean | `awsp <fragment>` matching zero | `ui/flows.rs` |
| L | SSO device flow | Selected profile's session is expired | `ui/flows.rs` |

H (tree grouped by SSO org) was explored and dropped — do not implement it.

## What "env" used to be — and isn't now

An earlier round of mocks coloured each row by a derived `env` field
(prod/staging/dev/...). **This is removed.** Profile *name* is the
identifying signal — don't re-introduce env regexes, env colors, or env
pills anywhere. The exception is the **prompt indicator in Screen J**:
when the new profile's name contains `prod` (case-insensitive),
`--emit-env` includes an extra `export AWSP_PROD=1` so the user's shell
prompt can show a red marker. That's the *shell's* responsibility — the
binary just sets the variable.

## Behaviour worth getting right

- **Picker renders in-place.** Print N lines; on every redraw,
  `MoveUp(N) + Clear(FromCursorDown)`. Never `EnterAlternateScreen` — the
  user wants the picker output to stay in scrollback.
- **Selection is keyboard-only.** No mouse. No clicks.
- **`/` for filter mode.** Characters typed update a `nucleo-matcher`
  pattern; matches re-sort by score, non-matches hide. `esc` clears, `⏎`
  confirms-and-leaves-filter-mode.
- **Numbered hotkeys 1-9.** Press `3` → activate profile #3 *immediately*
  (no enter needed). Only the first 9 visible rows get a hotkey.
- **Current profile detection.** `$AWS_PROFILE` first, then `[default]`
  from config, else "none". Render with the `●` glyph in mint.
- **Session expiry.** Read `~/.aws/sso/cache/*.json`, find the cache file
  whose `startUrl` matches the profile's `sso_session.sso_start_url`,
  parse `expiresAt`. Show as `3h 12m`, `12m`, `expired (4d ago)`.
- **Stderr vs stdout.** UI to stderr always. Only `--emit-env` output and
  `--json` output go to stdout.

## Testing

- Unit tests for `config.rs` against fixtures in
  `tests/config_fixtures/`.
- **Snapshot test every UI screen** via `insta`. The pattern:
  render to a `Vec<u8>` buffer with `crossterm`'s `QueueableCommand`,
  strip ANSI for the snapshot (or keep it — `insta` handles both).
- One integration test per public command in `tests/cli.rs` using
  `assert_cmd`.

## Style

- `rustfmt` defaults, `clippy --all-targets -- -D warnings` must pass.
- No `unwrap()`/`expect()` in non-test code. Errors propagate via `anyhow`
  to `main`; one human-readable message is printed there.
- Public types and functions have doc comments. Private ones don't need
  them unless non-obvious.
- Keep modules small. If `picker.rs` grows past 400 LOC, split a
  `picker/render.rs` and `picker/input.rs`.

## Out of scope

- Multi-account assume-role chains.
- Profile editing / creation (`~/.aws/config` is read-only as far as awsp
  is concerned).
- Windows-specific UI tweaks beyond what crossterm already handles.
- MRU / last-used tracking. (Sketched in earlier rounds, cut for v1.)

## Reference files

| Path | What it is |
|---|---|
| `README.md` | High-level summary, install story, what to deliver. |
| `SPEC.md` | Detailed visual + behavioural spec for every screen. |
| `PALETTE.md` | Color tokens with RGB values, in copy-paste-to-Rust form. |
| `reference/AWSP CLI.html` | Open in browser. Section "Tuned to your terminal" is canonical. |
| `reference/v6-native.jsx` | JSX source for the canonical mocks. Read it to confirm exact spacing, glyphs, and color usage when SPEC.md is ambiguous. |
| `reference/profiles.jsx` | 10 sample profiles. Mirror as a fixture in `tests/config_fixtures/sample.config`. |
| `reference/v1..v5*.jsx` | Earlier exploration variants. Informational only; do **not** implement. |
