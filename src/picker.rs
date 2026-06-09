use crate::aws_config::SsoProfile;
use crate::cache::CacheStatus;
use crate::palette;
use crate::picker_model::{PickerCommand, PickerEntry, PickerMode, PickerModel, PickerOutcome};
use anyhow::{bail, Context, Result};
use crossterm::cursor::{Hide, MoveToColumn, MoveUp, Show};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::style::{
    Attribute, Print, ResetColor, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{self, disable_raw_mode, enable_raw_mode, Clear, ClearType};
use crossterm::{execute, queue};
use std::io::{self, Stderr, Write};

const NAME_WIDTH: usize = 24;
const ACCOUNT_WIDTH: usize = 15;
const REGION_WIDTH: usize = 18;
const TABLE_RULE_WIDTH: usize = 76;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerView {
    Numbered,
    Table,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerSelection {
    Profile(String),
    Relogin(String),
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter(stderr: &mut Stderr) -> Result<Self> {
        enable_raw_mode().context("failed to enable terminal raw mode")?;
        execute!(stderr, Hide).context("failed to enter awsp picker screen")?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stderr(), Show, ResetColor);
    }
}

pub fn select_profile(
    profiles: &[SsoProfile],
    statuses: &[CacheStatus],
    current_profile: Option<&str>,
    view: PickerView,
) -> Result<PickerSelection> {
    if profiles.is_empty() {
        bail_no_profiles();
    }

    let model = PickerModel::new(profiles, statuses, current_profile);
    let mut state = PickerState::new(model, view);
    state.run()
}

pub fn bail_no_profiles() -> ! {
    eprintln!("  awsp: no SSO profiles found in ~/.aws/config");
    eprintln!("  → add a [sso-session ...] block and at least one [profile ...]");
    eprintln!(
        "    see https://docs.aws.amazon.com/cli/latest/userguide/sso-configure-profile-token.html"
    );
    std::process::exit(1);
}

struct PickerState<'a> {
    model: PickerModel<'a>,
    view: PickerView,
    rendered_lines: u16,
}

impl<'a> PickerState<'a> {
    fn new(model: PickerModel<'a>, view: PickerView) -> Self {
        Self {
            model,
            view,
            rendered_lines: 0,
        }
    }

    fn run(&mut self) -> Result<PickerSelection> {
        let mut stderr = io::stderr();
        let _guard = TerminalGuard::enter(&mut stderr)?;

        loop {
            self.render(&mut stderr)?;

            let event = event::read().context("failed to read terminal input")?;
            let command = match event {
                Event::Key(key) => key_to_command(key, self.model.mode(), self.view),
                Event::Resize(_, _) => PickerCommand::Noop,
                _ => continue,
            };

            match self.model.handle(command) {
                PickerOutcome::Selected(selection) => {
                    self.clear_rendered(&mut stderr)?;
                    return Ok(PickerSelection::Profile(selection));
                }
                PickerOutcome::Relogin(selection) => {
                    self.clear_rendered(&mut stderr)?;
                    return Ok(PickerSelection::Relogin(selection));
                }
                PickerOutcome::Continue => {}
                PickerOutcome::Cancelled => {
                    self.clear_rendered(&mut stderr)?;
                    std::process::exit(130);
                }
                PickerOutcome::NoMatch => {
                    self.clear_rendered(&mut stderr)?;
                    bail!("no profiles match the current filter");
                }
            }
        }
    }

    fn render(&mut self, stderr: &mut Stderr) -> Result<()> {
        let (width, height) = terminal::size().unwrap_or((100, 24));
        if width < 50 {
            bail!("terminal too narrow");
        }

        self.model.set_terminal_height(height as usize);
        let columns = Columns::for_width(width);
        let mut lines = 0_u16;

        if self.rendered_lines > 0 {
            queue!(
                stderr,
                MoveUp(self.rendered_lines),
                MoveToColumn(0),
                Clear(ClearType::FromCursorDown)
            )?;
        }

        if self.view == PickerView::Table {
            self.render_table_header(stderr, &columns)?;
            lines += 2;
        } else if self.render_filter_or_empty_state(stderr)? {
            lines += 1;
            queue!(stderr, Print("\r\n"))?;
            lines += 1;
        } else {
            queue!(stderr, MoveToColumn(0))?;
        }

        if self.model.filtered_len() == 0 {
            queue!(
                stderr,
                Print("  "),
                SetForegroundColor(palette::MUTED),
                Print("no profiles match"),
                ResetColor,
                Print("\r\n")
            )?;
            lines += 1;
        } else {
            for (visible_index, entry) in self.model.visible_entries() {
                let selected = visible_index == self.model.selected();
                match self.view {
                    PickerView::Numbered => {
                        self.render_numbered_row(stderr, visible_index, entry, selected, &columns)?
                    }
                    PickerView::Table => {
                        self.render_table_row(stderr, entry, selected, &columns)?
                    }
                }
                lines += 1;
            }
        }

        queue!(stderr, Print("\r\n"))?;
        lines += 1;
        match self.view {
            PickerView::Numbered => self.render_numbered_footer(stderr)?,
            PickerView::Table => self.render_table_footer(stderr)?,
        }
        lines += 1;
        self.rendered_lines = lines;
        stderr.flush().context("failed to render awsp picker")
    }

    fn clear_rendered(&mut self, stderr: &mut Stderr) -> Result<()> {
        if self.rendered_lines == 0 {
            return Ok(());
        }

        queue!(
            stderr,
            MoveUp(self.rendered_lines),
            MoveToColumn(0),
            Clear(ClearType::FromCursorDown),
            Show,
            ResetColor
        )?;
        stderr.flush().context("failed to clear awsp picker")?;
        self.rendered_lines = 0;
        Ok(())
    }

    fn render_filter_or_empty_state(&self, stderr: &mut Stderr) -> io::Result<bool> {
        queue!(stderr, MoveToColumn(0))?;
        if self.model.filter_is_visible() {
            queue!(
                stderr,
                SetForegroundColor(palette::PINK),
                SetAttribute(Attribute::Bold),
                Print("/ "),
                SetAttribute(Attribute::Reset),
                SetForegroundColor(palette::FG),
                Print(self.model.filter()),
                Print("_"),
                ResetColor,
                Print("\r\n")
            )?;
            return Ok(true);
        }

        if self.model.current_entry().is_some() {
            return Ok(false);
        }

        queue!(
            stderr,
            SetForegroundColor(palette::DIM),
            Print("○"),
            SetForegroundColor(palette::MUTED),
            Print(" no active profile"),
            SetForegroundColor(palette::DIM),
            Print("  ·  hint: pick one below"),
            ResetColor,
            Print("\r\n")
        )?;
        Ok(true)
    }

    fn render_current_row_status(
        &self,
        stderr: &mut Stderr,
        entry: &PickerEntry<'_>,
    ) -> io::Result<()> {
        if entry.is_current {
            queue!(
                stderr,
                SetForegroundColor(palette::GREEN),
                Print("  ◀ current")
            )?;
            separator(stderr)?;
            render_session_status(stderr, &entry.status)?;
        }
        Ok(())
    }

    fn render_numbered_row(
        &self,
        stderr: &mut Stderr,
        visible_index: usize,
        entry: &PickerEntry<'_>,
        selected: bool,
        columns: &Columns,
    ) -> io::Result<()> {
        let scroll_marker = scroll_marker_for(&self.model, visible_index);
        let marker = if scroll_marker.is_empty() {
            if selected {
                "▸"
            } else {
                " "
            }
        } else {
            scroll_marker
        };
        queue!(
            stderr,
            SetForegroundColor(if selected {
                palette::PINK
            } else {
                palette::DIM
            }),
            SetAttribute(if selected {
                Attribute::Bold
            } else {
                Attribute::Reset
            }),
            Print(marker),
            SetAttribute(Attribute::Reset)
        )?;

        let hotkey = if visible_index < 9 {
            format!(" {} ", visible_index + 1)
        } else {
            "   ".to_string()
        };
        queue!(
            stderr,
            SetForegroundColor(palette::PINK),
            SetAttribute(Attribute::Bold),
            Print(hotkey),
            SetAttribute(Attribute::Reset)
        )?;
        self.render_profile_cells(stderr, entry, selected, columns)?;
        self.render_current_row_status(stderr, entry)?;
        queue!(stderr, ResetColor, Print("\r\n"))
    }

    fn render_table_header(&self, stderr: &mut Stderr, columns: &Columns) -> io::Result<()> {
        queue!(
            stderr,
            MoveToColumn(0),
            Print("  "),
            SetForegroundColor(palette::MUTED),
            SetAttribute(Attribute::Bold),
            Print(format!("{:<26}", "PROFILE")),
            Print(format!("{:<15}", "ACCOUNT"))
        )?;
        if columns.show_region {
            queue!(stderr, Print(format!("{:<18}", "REGION")))?;
        }
        if columns.show_role {
            queue!(stderr, Print("ROLE"))?;
        }
        queue!(
            stderr,
            SetAttribute(Attribute::Reset),
            ResetColor,
            Print("\r\n  "),
            SetForegroundColor(palette::DIM),
            Print("─".repeat(TABLE_RULE_WIDTH)),
            ResetColor,
            Print("\r\n")
        )
    }

    fn render_table_row(
        &self,
        stderr: &mut Stderr,
        entry: &PickerEntry<'_>,
        selected: bool,
        columns: &Columns,
    ) -> io::Result<()> {
        if selected {
            queue!(
                stderr,
                SetBackgroundColor(palette::ROW_SELECTED_BG),
                MoveToColumn(0),
                Clear(ClearType::CurrentLine)
            )?;
        }

        let marker = if selected {
            "▸ "
        } else if entry.is_current {
            "● "
        } else {
            "  "
        };
        queue!(
            stderr,
            SetForegroundColor(if selected {
                palette::PINK
            } else if entry.is_current {
                palette::GREEN
            } else {
                palette::DIM
            }),
            SetAttribute(if selected {
                Attribute::Bold
            } else {
                Attribute::Reset
            }),
            Print(marker),
            SetAttribute(Attribute::Reset)
        )?;
        self.render_profile_cells(stderr, entry, selected, columns)?;
        queue!(stderr, ResetColor, Print("\r\n"))
    }

    fn render_profile_cells(
        &self,
        stderr: &mut Stderr,
        entry: &PickerEntry<'_>,
        selected: bool,
        columns: &Columns,
    ) -> io::Result<()> {
        queue!(
            stderr,
            SetForegroundColor(if entry.is_current {
                palette::GREEN
            } else {
                palette::FG
            }),
            SetAttribute(if selected {
                Attribute::Bold
            } else {
                Attribute::Reset
            }),
            Print(pad_or_truncate(&entry.profile.name, NAME_WIDTH)),
            SetAttribute(Attribute::Reset),
            SetForegroundColor(palette::MUTED),
            Print(pad_or_truncate(&entry.profile.account_id, ACCOUNT_WIDTH))
        )?;
        if columns.show_region {
            queue!(
                stderr,
                SetForegroundColor(palette::CYAN),
                Print(pad_or_truncate(&entry.profile.region.label(), REGION_WIDTH))
            )?;
        }
        if columns.show_role {
            queue!(
                stderr,
                SetForegroundColor(palette::PURPLE),
                Print(&entry.profile.role_name)
            )?;
        }
        Ok(())
    }

    fn render_numbered_footer(&self, stderr: &mut Stderr) -> io::Result<()> {
        queue!(stderr, Print("  "))?;
        footer_key(stderr, "1-9")?;
        footer_word(stderr, " jump  ")?;
        footer_key(stderr, "↑↓")?;
        footer_word(stderr, " nav  ")?;
        footer_key(stderr, "/")?;
        footer_word(stderr, " filter  ")?;
        footer_key(stderr, "⏎")?;
        footer_word(stderr, " switch  ")?;
        footer_key(stderr, "r")?;
        footer_word(stderr, " re-login  ")?;
        footer_key(stderr, "q")?;
        footer_word(stderr, " quit")?;
        queue!(stderr, ResetColor, Print("\r\n"))
    }

    fn render_table_footer(&self, stderr: &mut Stderr) -> io::Result<()> {
        queue!(
            stderr,
            Print("  "),
            SetForegroundColor(palette::DIM),
            Print(format!("{} profiles · ", self.model.filtered_len()))
        )?;
        if let Some(current) = self.model.current_entry() {
            render_session_status(stderr, &current.status)?;
        } else {
            queue!(
                stderr,
                SetForegroundColor(palette::MUTED),
                Print("no active profile")
            )?;
        }
        queue!(stderr, SetForegroundColor(palette::DIM), Print("  ────  "))?;
        footer_key(stderr, "↑↓")?;
        footer_word(stderr, "·")?;
        footer_key(stderr, "⏎")?;
        footer_word(stderr, "·")?;
        footer_key(stderr, "/")?;
        footer_word(stderr, "·")?;
        footer_key(stderr, "q")?;
        queue!(stderr, ResetColor, Print("\r\n"))
    }
}

fn key_to_command(key: KeyEvent, mode: PickerMode, view: PickerView) -> PickerCommand {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('c') => PickerCommand::Cancel,
            KeyCode::Char('y') => PickerCommand::Noop,
            _ => PickerCommand::Noop,
        };
    }

    if mode == PickerMode::Filter {
        return match key.code {
            KeyCode::Esc => PickerCommand::Cancel,
            KeyCode::Enter => PickerCommand::Enter,
            KeyCode::Backspace => PickerCommand::Backspace,
            KeyCode::Up | KeyCode::Char('k') => PickerCommand::Up,
            KeyCode::Down | KeyCode::Char('j') => PickerCommand::Down,
            KeyCode::Char(value @ '1'..='9') => {
                PickerCommand::JumpVisible(value as usize - '1' as usize)
            }
            KeyCode::Char(value) if !value.is_control() => PickerCommand::Input(value),
            _ => PickerCommand::Noop,
        };
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => PickerCommand::Up,
        KeyCode::Down | KeyCode::Char('j') => PickerCommand::Down,
        KeyCode::Home | KeyCode::Char('g') => PickerCommand::First,
        KeyCode::End | KeyCode::Char('G') => PickerCommand::Last,
        KeyCode::Char('/') => PickerCommand::StartFilter,
        KeyCode::Char('r') => PickerCommand::Relogin,
        KeyCode::Esc | KeyCode::Char('q') => PickerCommand::Cancel,
        KeyCode::Enter => PickerCommand::Enter,
        KeyCode::Char(value @ '1'..='9') if view == PickerView::Numbered => {
            PickerCommand::JumpVisible(value as usize - '1' as usize)
        }
        KeyCode::Char(value) if !value.is_control() => PickerCommand::Input(value),
        _ => PickerCommand::Noop,
    }
}

fn render_session_status(stderr: &mut Stderr, status: &CacheStatus) -> io::Result<()> {
    let color = match status.expires_in_seconds() {
        Some(seconds) if seconds < 0 => palette::RED,
        Some(seconds) if seconds < 3600 => palette::YELLOW,
        Some(_) => palette::GREEN,
        None => palette::MUTED,
    };
    queue!(stderr, SetForegroundColor(color), Print(status.label()))
}

fn separator(stderr: &mut Stderr) -> io::Result<()> {
    queue!(stderr, SetForegroundColor(palette::DIM), Print("  ·  "))
}

fn footer_key(stderr: &mut Stderr, key: &str) -> io::Result<()> {
    queue!(
        stderr,
        SetForegroundColor(palette::PINK),
        SetAttribute(Attribute::Bold),
        Print(key),
        SetAttribute(Attribute::Reset)
    )
}

fn footer_word(stderr: &mut Stderr, word: &str) -> io::Result<()> {
    queue!(stderr, SetForegroundColor(palette::DIM), Print(word))
}

fn scroll_marker_for(model: &PickerModel<'_>, visible_index: usize) -> &'static str {
    if visible_index == model.offset() && model.has_rows_before() {
        "↑↑"
    } else if visible_index + 1 == model.offset() + model.visible_rows() && model.has_rows_after() {
        "↓↓"
    } else {
        ""
    }
}

#[derive(Debug, Clone, Copy)]
struct Columns {
    show_region: bool,
    show_role: bool,
}

impl Columns {
    fn for_width(width: u16) -> Self {
        Self {
            show_region: width >= 60,
            show_role: width >= 80,
        }
    }
}

fn pad_or_truncate(value: &str, width: usize) -> String {
    let mut output = value.chars().take(width).collect::<String>();
    let length = output.chars().count();
    if length < width {
        output.push_str(&" ".repeat(width - length));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pads_and_truncates_profile_names() {
        assert_eq!(pad_or_truncate("abc", 5), "abc  ");
        assert_eq!(pad_or_truncate("abcdef", 3), "abc");
    }

    #[test]
    fn maps_number_keys_only_for_numbered_normal_view() {
        let one = KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE);
        assert_eq!(
            key_to_command(one, PickerMode::Normal, PickerView::Numbered),
            PickerCommand::JumpVisible(0)
        );
        assert_eq!(
            key_to_command(one, PickerMode::Normal, PickerView::Table),
            PickerCommand::Input('1')
        );
    }

    #[test]
    fn maps_filter_letters_as_input() {
        let q = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(
            key_to_command(q, PickerMode::Normal, PickerView::Numbered),
            PickerCommand::Cancel
        );
        assert_eq!(
            key_to_command(q, PickerMode::Filter, PickerView::Numbered),
            PickerCommand::Input('q')
        );
    }
}
