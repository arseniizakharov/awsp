# PALETTE.md — design tokens

All RGB. Truecolor only — `awsp` requires a truecolor-capable terminal.
Detect via the `supports-color` crate; if missing, exit early with a
helpful error (do **not** try to map to ANSI 256 — the design depends on
specific shades).

## Tokens

| Name | Hex | RGB | Used for |
|---|---|---|---|
| `BG` | `#1a1b26` | `26, 27, 38` | Not painted; we just don't reset the bg |
| `FG` | `#c0caf5` | `192, 202, 245` | Primary text, profile names |
| `DIM` | `#565f89` | `86, 95, 137` | Separators, secondary text, labels |
| `MUTED` | `#787c99` | `120, 124, 153` | Account ids, footers, "no active" copy |
| `MINT` | `#73daca` | `115, 218, 202` | Active-profile marker `●`, path text |
| `PINK` | `#f7768e` | `247, 118, 142` | Prompt `»`, selection `▸`, hotkeys, errors |
| `PURPLE` | `#bb9af7` | `187, 154, 247` | Role names |
| `BLUE` | `#7aa2f7` | `122, 162, 247` | Secondary suggestions, URLs |
| `CYAN` | `#7dcfff` | `125, 207, 255` | Regions |
| `GREEN` | `#9ece6a` | `158, 206, 106` | Success `✓`, valid session, current marker |
| `YELLOW` | `#e0af68` | `224, 175, 104` | Session "expiring soon" |
| `ORANGE` | `#ff9e64` | `255, 158, 100` | Reserved (unused in v1) |
| `RED` | `#f7768e` | `247, 118, 142` | (= PINK) Errors, expired sessions |
| `ROW_SELECTED_BG` | `#252638` | `37, 38, 56` | Selected-row background in Screen G |

## Drop-in Rust module

```rust
// src/ui/palette.rs
use crossterm::style::Color;

pub const BG:               Color = Color::Rgb { r: 0x1a, g: 0x1b, b: 0x26 };
pub const FG:               Color = Color::Rgb { r: 0xc0, g: 0xca, b: 0xf5 };
pub const DIM:              Color = Color::Rgb { r: 0x56, g: 0x5f, b: 0x89 };
pub const MUTED:            Color = Color::Rgb { r: 0x78, g: 0x7c, b: 0x99 };
pub const MINT:             Color = Color::Rgb { r: 0x73, g: 0xda, b: 0xca };
pub const PINK:             Color = Color::Rgb { r: 0xf7, g: 0x76, b: 0x8e };
pub const PURPLE:           Color = Color::Rgb { r: 0xbb, g: 0x9a, b: 0xf7 };
pub const BLUE:             Color = Color::Rgb { r: 0x7a, g: 0xa2, b: 0xf7 };
pub const CYAN:             Color = Color::Rgb { r: 0x7d, g: 0xcf, b: 0xff };
pub const GREEN:            Color = Color::Rgb { r: 0x9e, g: 0xce, b: 0x6a };
pub const YELLOW:           Color = Color::Rgb { r: 0xe0, g: 0xaf, b: 0x68 };
pub const ORANGE:           Color = Color::Rgb { r: 0xff, g: 0x9e, b: 0x64 };
pub const RED:              Color = PINK;
pub const ROW_SELECTED_BG:  Color = Color::Rgb { r: 0x25, g: 0x26, b: 0x38 };
```

## Rules

1. **Never paint the background** except for the selected-row band in
   Screen G. The user's terminal background shows through everywhere
   else.
2. **Bold + color** rather than backgrounds. Selection is signalled by
   `▸` + bold name, not a fill.
3. **One bold per row maximum.** Bold the profile name on the selected
   row; everything else is regular weight.
4. **No size variation.** A terminal cannot size text. Mocks that look
   bigger are using bold + better contrast — both reproducible.
5. **Don't introduce env colors.** The earlier prod/staging/dev
   color-coded pills are removed. Profile name is the identifying
   signal.

## Theme compatibility note

These tokens are tuned to a dark Tokyo Night-ish background. On light
terminals they will look wrong. Out of scope for v1 to support light
themes — add a `--theme` flag later if requested.
