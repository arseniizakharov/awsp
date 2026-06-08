# SPEC.md — per-screen specification

All terminal renders use the palette in `PALETTE.md`. Token names below
(e.g. `MINT`, `PINK`) refer to that file.

The user's shell prompt in mocks is:

```
~/projects/nomadsre/awsp [main*]
» <command>
```

Lines starting with `»` are user input — your binary doesn't render
those. They are shown in mocks for context.

---

## Screen F — Interactive picker

Trigger: `awsp` with no arguments.

### Layout

```
                                                          ← (just after the prompt)
● active acme-staging-dev  ·  447091823641  ·  us-west-2  ·  DeveloperAccess  ·  3h 12m

  1 acme-prod-admin       682471093210   us-east-1       AdministratorAccess
  2 acme-prod-readonly    682471093210   us-east-1       ReadOnlyAccess
  3 acme-prod-billing     682471093210   us-east-1       BillingAccess
  4 acme-staging-admin    447091823641   us-west-2       AdministratorAccess
▸ 5 acme-staging-dev      447091823641   us-west-2       DeveloperAccess         ◀ current
  6 acme-dev-sandbox      910237842156   us-west-2       PowerUserAccess
  7 acme-dev-data         910237842156   eu-west-1       DataEngineerAccess
  8 personal-playground   201938475610   eu-central-1    AdministratorAccess
  9 client-northwind      584012736092   ap-southeast-2  ConsultantAccess
    client-globex-prod    739102648351   us-east-2       ReadOnlyAccess

  1-9 jump  ↑↓ nav  / filter  ⏎ switch  r re-login  q quit
```

### Status line (top)

| Token | What | Color |
|---|---|---|
| `●` | active marker | `MINT` |
| `active` label | | `DIM` |
| profile name | `$AWS_PROFILE` value | `FG` bold |
| account id | | `MUTED` |
| region | | `CYAN` |
| role | | `PURPLE` |
| session expiry | `Nh Mm` / `Nm` / `expired (Nd ago)` | `GREEN` if >1h, `YELLOW` if <1h, `RED` if expired |
| separators | `  ·  ` (two spaces, middle dot, two spaces) | `DIM` |

If no profile is active, replace the entire line with:
```
○ no active profile  ·  hint: pick one below
```
(`○` in `DIM`, "no active profile" in `MUTED`).

### List rows

Three states per row: selected, current, neither.

| Cell | Width | Content | Style |
|---|---|---|---|
| Marker | 1 cell + space | `▸` if selected else ` ` | `PINK` bold when selected |
| Hotkey | 3 cells `" N "` | `1`-`9` for first 9 rows, blanks for rest | `PINK` bold |
| Name | pad to 22 | profile name | `FG` (or `GREEN` if current); bold if selected |
| Account | pad to 15 | account id | `MUTED` |
| Region | pad to 18 | region | `CYAN` |
| Role | rest | role name | `PURPLE` |
| Current tag | (only on current row, after role) | `  ◀ current` | `GREEN` |

Padding is `String::padEnd`-equivalent (`format!("{:<22}", name)`).
Trailing whitespace on rows is fine.

### Footer

```
  1-9 jump  ↑↓ nav  / filter  ⏎ switch  r re-login  q quit
```

Glyph keys in `PINK` bold; words in `DIM`. Two-space gap between
glyph and word; two-space gap between groups.

### Keys

| Key | Action |
|---|---|
| `↑` / `k` | move up; wraps |
| `↓` / `j` | move down; wraps |
| `g` / `Home` | first |
| `G` / `End` | last |
| `⏎` | activate selected → Screen J or Screen L |
| `1`-`9` | activate row N immediately |
| `/` | enter filter mode |
| `r` | re-login current SSO session |
| `Ctrl-y` | copy selected profile name (arboard) |
| `q` / `esc` / `Ctrl-c` | exit code 130, no change |

### Filter mode

While `/` is engaged, the status line at top is replaced with:

```
/ <query>_
```

(`/` in `PINK`, query in `FG`, underscore is the blinking cursor.)

The list shrinks to matching rows, ordered by `nucleo-matcher` score
(highest first). Matched characters within each name are bolded *and*
colored `PINK` (everything else in name remains `FG`).

When filtered, hotkeys 1-9 still apply to the *currently visible* rows
in order.

`esc` clears the filter and returns to full list. `⏎` while in filter
mode activates the top match.

### Rendering

In-place: track lines written, on redraw `MoveUp(N)` + `Clear(FromCursorDown)`,
re-print. **Do not** use the alternate screen.

When more than ~15 profiles, scroll the list with a 1-row buffer on
either side of the selection. Don't draw a scrollbar — instead, when
the list is scrolled, replace the leftmost two cells of the first / last
visible row with `↑↑` / `↓↓` in `DIM`.

---

## Screen G — Table variant

Trigger: `awsp --table`.

Same data, column headers added, slightly different row style.

```
  PROFILE                   ACCOUNT        REGION            ROLE
  ────────────────────────────────────────────────────────────────────────────
  acme-prod-admin           682471093210   us-east-1         AdministratorAccess
  acme-prod-readonly        682471093210   us-east-1         ReadOnlyAccess
  acme-prod-billing         682471093210   us-east-1         BillingAccess
  acme-staging-admin        447091823641   us-west-2         AdministratorAccess
▸ acme-staging-dev          447091823641   us-west-2         DeveloperAccess
● acme-staging-dev          447091823641   us-west-2         DeveloperAccess         ← (illustrative; this line is the current marker treatment when the current is not the selected row)
  acme-dev-sandbox          910237842156   us-west-2         PowerUserAccess
  ...

  10 profiles · ✓ SSO valid · expires 3h 12m  ────  ↑↓·⏎·/·q
```

### Differences from F

- Header row: column names in `MUTED` bold; followed by a `─` rule line
  in `DIM` across 76 cells.
- No hotkey column. (Use the picker if you want hotkeys.)
- Markers: selected = `▸ ` in `PINK`; current (when not selected) =
  `● ` in `GREEN`; neither = `  `.
- Selected row gets a background fill `#252638` across its width.
- Footer is condensed: counts + SSO status on the left, glyph keys
  separated by middle-dots on the right, separated by `  ────  `.

### Keys

Identical to F **except** no `1`-`9` hotkeys.

---

## Screen I — `awsp status` and ambiguous match

Two sub-screens that share the same renderer.

### I.a — `awsp status`

Trigger: `awsp status`. Non-interactive; prints and exits 0.

```
● acme-staging-dev   ·  us-west-2  ·  DeveloperAccess  ·  valid 3h 12m
  └─ 447091823641 · acme-corp.awsapps.com/start
```

- Line 1: `●` (mint) + profile (FG bold) + region (cyan) + role (purple)
  + session status (`valid 3h 12m` green, `expires soon (12m)` yellow,
  `EXPIRED (4d ago)` red bold).
- Line 2: indented `└─ ` (DIM) + account id (MUTED) + ` · ` + SSO start
  URL (MUTED).

`awsp status --json` instead emits a single line of JSON:

```json
{"profile":"acme-staging-dev","account":"447091823641","region":"us-west-2","role":"DeveloperAccess","sso_start_url":"https://acme-corp.awsapps.com/start","session_state":"valid","expires_in_seconds":11520}
```

### I.b — Ambiguous match

Trigger: `awsp <fragment>` where multiple profiles match.

```
  matches 3 profiles:
  1 acme-prod-admin     682471093210  us-east-1  AdministratorAccess
  2 acme-prod-readonly  682471093210  us-east-1  ReadOnlyAccess
  3 acme-prod-billing   682471093210  us-east-1  BillingAccess

  pick 1-3 · or refine: awsp prod-r_
```

- Header: `matches N profiles:` in `DIM`.
- Rows: `  N ` hotkey (PINK bold) + name (FG bold, pad to 20) + account
  (MUTED, pad to 14) + region (CYAN, pad to 11) + role (PURPLE).
- Hint line at bottom: `  pick ` (DIM) + `1-N` (PINK bold) + ` · or
  refine: ` (DIM) + `awsp <fragment>_` (FG) where the trailing `_` is
  the blinking cursor. The fragment shown is the user's original
  fragment + the first character that would have disambiguated.

Number keys 1-9 activate immediately; any other character starts a new
filter query. The user can also just type a longer fragment and press
enter (interactive line edit; `crossterm::event::read` loop).

Match algorithm: case-insensitive substring first, fall back to
`nucleo-matcher` if zero substring matches.

---

## Screen J — Switch result

Trigger: any successful profile switch (from F, G, I.b, or direct
`awsp <name>` with exactly one match).

```
  ✓  switched  acme-staging-dev  →  acme-prod-readonly
     682471093210 · us-east-1 · ReadOnlyAccess · session 7h 58m
```

- `✓` (GREEN bold) + `switched` (DIM) + old name (FG bold) + ` → ` (DIM)
  + new name (FG bold).
- Indented details line: account · region · role · session expiry, all
  `MUTED`.

Then the binary exits 0 after printing on **stdout**:

```
export AWS_PROFILE='acme-prod-readonly'
export AWS_REGION='us-east-1'
```

(If profile name contains `prod` case-insensitively, also emit
`export AWSP_PROD=1`; otherwise emit `unset AWSP_PROD`.)

The user's shell function evals stdout. The visual feedback above
appeared on stderr, so it doesn't pollute the eval'd block.

---

## Screen K — Did-you-mean

Trigger: `awsp <fragment>` matching zero profiles.

```
  ✗ no profile named acme-prod-redonly
    did you mean    acme-prod-readonly  ?
       or           acme-prod-admin,  acme-prod-billing

  → run awsp with no args for the interactive picker
```

- `✗` RED bold + the offending fragment (RED).
- Top suggestion: `acme-prod-readonly` in `PINK` bold underlined.
- Secondary suggestions in `BLUE` (cool-blue accent — see palette).
- Final line: `→ ` (DIM) + `run awsp...` (MUTED) + `awsp` (FG bold) +
  `with no args...` (MUTED).

Algorithm: `strsim::levenshtein` with threshold = max(2, len/3).
Up to 3 suggestions, ordered by distance.

Exit code: `1`.

---

## Screen L — SSO device authorization

Trigger: selected profile's SSO session is missing or expired.

```
  !  SSO session for acme-corp expired  (4d ago)
  →  Launching device authorization…

  ┌─────────────────────────────────────────────────┐
  │ Open this URL in your browser:                  │
  │   https://device.sso.us-east-1.amazonaws.com/   │
  │                                                 │
  │ Confirm the code:  WXKQ-MTRP                    │
  │ Waiting for confirmation ⠋                      │
  │ (auto-retry, ctrl-c to cancel)                  │
  └─────────────────────────────────────────────────┘
```

- `!` RED bold + message; `4d ago` in MUTED parens.
- Arrow line: `→ ` (DIM) + message (FG).
- Box drawn with `┌─┐│└─┘`; border in `#2a2d33` (rendered with
  `Color::Rgb`).
- URL in `BLUE` bold underlined; user code in `PINK` bold (no
  letter-spacing in terminal; just bold).
- Spinner: `indicatif` `dots` style, 80ms frames.
- On success: clear the box and proceed to original switch → Screen J.
- On user cancel: exit 130.
- On polling timeout (default 90s, configurable in
  `~/.config/awsp/config.toml`): clear box, print
  `✗ timed out waiting for confirmation` in RED, exit 1.

`opener::open(&url)` is called once when the box appears; ignore its
result — the manual URL is the fallback.

---

## Cross-cutting

### Sort order

Profiles are sorted in this order, stable:

1. SSO org name (`sso_session` ini field, alphabetical).
2. Profile name within org (alphabetical).

The currently active profile is **not** floated to the top — the user
will use the `●` marker to find it.

### Width

Assume minimum terminal width = 80. If `crossterm::terminal::size()`
returns less, drop the role column from F and G (and let the rest
flow). Below 60 columns, drop region too. Below 50, exit with error
"terminal too narrow" — don't try to render.

### Empty config

If no profiles are defined, all entry points print:

```
  awsp: no SSO profiles found in ~/.aws/config
  → add a [sso-session ...] block and at least one [profile ...]
    see https://docs.aws.amazon.com/cli/latest/userguide/sso-configure-profile-token.html
```

Exit 1.

### Resize handling

Subscribe to `crossterm::event::Event::Resize`; on resize, full redraw.
