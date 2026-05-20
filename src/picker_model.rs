use crate::aws_config::SsoProfile;
use crate::cache::CacheStatus;

const MAX_VISIBLE_ROWS: usize = 15;

#[derive(Debug, Clone)]
pub(crate) struct PickerEntry<'a> {
    pub(crate) profile: &'a SsoProfile,
    pub(crate) status: CacheStatus,
    pub(crate) is_current: bool,
    search_text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerMode {
    Normal,
    Filter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerCommand {
    Up,
    Down,
    First,
    Last,
    JumpVisible(usize),
    StartFilter,
    Input(char),
    Backspace,
    Enter,
    Relogin,
    Cancel,
    Noop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PickerOutcome {
    Continue,
    Selected(String),
    Relogin(String),
    Cancelled,
    NoMatch,
}

pub(crate) struct PickerModel<'a> {
    entries: Vec<PickerEntry<'a>>,
    current_profile: Option<String>,
    filtered: Vec<usize>,
    selected: usize,
    offset: usize,
    filter: String,
    mode: PickerMode,
    visible_rows: usize,
}

impl<'a> PickerModel<'a> {
    pub(crate) fn new(
        profiles: &'a [SsoProfile],
        statuses: &[CacheStatus],
        current_profile: Option<&str>,
    ) -> Self {
        let entries = build_entries(profiles, statuses, current_profile);
        let mut model = Self {
            entries,
            current_profile: current_profile.map(str::to_string),
            filtered: Vec::new(),
            selected: 0,
            offset: 0,
            filter: String::new(),
            mode: PickerMode::Normal,
            visible_rows: MAX_VISIBLE_ROWS,
        };
        model.apply_filter();
        model
    }

    pub(crate) fn handle(&mut self, command: PickerCommand) -> PickerOutcome {
        match self.mode {
            PickerMode::Normal => self.handle_normal(command),
            PickerMode::Filter => self.handle_filter(command),
        }
    }

    pub(crate) fn set_terminal_height(&mut self, height: usize) {
        let chrome_rows = if self.filter_is_visible() { 5 } else { 4 };
        self.visible_rows = height
            .saturating_sub(chrome_rows)
            .max(1)
            .clamp(1, MAX_VISIBLE_ROWS);
        self.keep_selection_visible();
    }

    pub(crate) fn current_entry(&self) -> Option<&PickerEntry<'a>> {
        let current = self.current_profile.as_deref()?;
        self.entries
            .iter()
            .find(|entry| entry.profile.name == current)
    }

    pub(crate) fn mode(&self) -> PickerMode {
        self.mode
    }

    pub(crate) fn filter(&self) -> &str {
        &self.filter
    }

    pub(crate) fn filter_is_visible(&self) -> bool {
        self.mode == PickerMode::Filter || !self.filter.is_empty()
    }

    pub(crate) fn visible_rows(&self) -> usize {
        self.visible_rows
    }

    pub(crate) fn filtered_len(&self) -> usize {
        self.filtered.len()
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset
    }

    pub(crate) fn selected(&self) -> usize {
        self.selected
    }

    pub(crate) fn has_rows_before(&self) -> bool {
        self.offset > 0
    }

    pub(crate) fn has_rows_after(&self) -> bool {
        self.offset + self.visible_rows < self.filtered.len()
    }

    pub(crate) fn visible_entries(&self) -> impl Iterator<Item = (usize, &PickerEntry<'a>)> {
        let end = usize::min(self.filtered.len(), self.offset + self.visible_rows);
        self.filtered[self.offset..end]
            .iter()
            .enumerate()
            .map(|(visible_index, entry_index)| {
                (self.offset + visible_index, &self.entries[*entry_index])
            })
    }

    fn handle_normal(&mut self, command: PickerCommand) -> PickerOutcome {
        match command {
            PickerCommand::Up => self.move_up(),
            PickerCommand::Down => self.move_down(),
            PickerCommand::First => self.jump_first(),
            PickerCommand::Last => self.jump_last(),
            PickerCommand::JumpVisible(index) => return self.select_visible(index),
            PickerCommand::StartFilter => self.mode = PickerMode::Filter,
            PickerCommand::Cancel => return PickerOutcome::Cancelled,
            PickerCommand::Relogin => return self.relogin_selected(),
            PickerCommand::Enter => return self.select_selected(),
            PickerCommand::Input(value) if !value.is_control() => {
                self.mode = PickerMode::Filter;
                self.filter.push(value);
                self.apply_filter();
            }
            _ => {}
        }

        PickerOutcome::Continue
    }

    fn handle_filter(&mut self, command: PickerCommand) -> PickerOutcome {
        match command {
            PickerCommand::Cancel => {
                self.mode = PickerMode::Normal;
                self.filter.clear();
                self.apply_filter();
            }
            PickerCommand::Enter => return self.select_selected(),
            PickerCommand::Backspace => {
                self.filter.pop();
                self.apply_filter();
            }
            PickerCommand::Up => self.move_up(),
            PickerCommand::Down => self.move_down(),
            PickerCommand::JumpVisible(index) => return self.select_visible(index),
            PickerCommand::Input(value) if !value.is_control() => {
                self.filter.push(value);
                self.apply_filter();
            }
            _ => {}
        }

        PickerOutcome::Continue
    }

    fn select_selected(&self) -> PickerOutcome {
        let Some(entry_index) = self.filtered.get(self.selected).copied() else {
            return PickerOutcome::NoMatch;
        };
        PickerOutcome::Selected(self.entries[entry_index].profile.name.clone())
    }

    fn select_visible(&self, index: usize) -> PickerOutcome {
        let visible_index = self.offset + index;
        let Some(entry_index) = self.filtered.get(visible_index).copied() else {
            return PickerOutcome::NoMatch;
        };
        PickerOutcome::Selected(self.entries[entry_index].profile.name.clone())
    }

    fn relogin_selected(&self) -> PickerOutcome {
        let Some(entry_index) = self.filtered.get(self.selected).copied() else {
            return PickerOutcome::NoMatch;
        };
        PickerOutcome::Relogin(self.entries[entry_index].profile.name.clone())
    }

    fn move_up(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.filtered.len() - 1
        } else {
            self.selected - 1
        };
        self.keep_selection_visible();
    }

    fn move_down(&mut self) {
        if self.filtered.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.filtered.len();
        self.keep_selection_visible();
    }

    fn jump_first(&mut self) {
        self.selected = 0;
        self.keep_selection_visible();
    }

    fn jump_last(&mut self) {
        if !self.filtered.is_empty() {
            self.selected = self.filtered.len() - 1;
            self.keep_selection_visible();
        }
    }

    fn apply_filter(&mut self) {
        let needle = self.filter.trim().to_ascii_lowercase();
        let mut matches = self
            .entries
            .iter()
            .enumerate()
            .filter_map(|(index, entry)| {
                if needle.is_empty() {
                    return Some((index, 0));
                }
                entry
                    .search_text
                    .find(&needle)
                    .map(|position| (index, position))
            })
            .collect::<Vec<_>>();

        matches.sort_by(|left, right| {
            left.1.cmp(&right.1).then_with(|| {
                self.entries[left.0]
                    .profile
                    .name
                    .cmp(&self.entries[right.0].profile.name)
            })
        });
        self.filtered = matches.into_iter().map(|(index, _)| index).collect();

        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        self.keep_selection_visible();
    }

    fn keep_selection_visible(&mut self) {
        let lower_buffer = 1;
        let upper_buffer = self.visible_rows.saturating_sub(2);

        if self.selected < self.offset + lower_buffer {
            self.offset = self.selected.saturating_sub(lower_buffer);
        } else if self.selected >= self.offset + upper_buffer {
            self.offset = self.selected.saturating_sub(upper_buffer);
        }

        let max_offset = self.filtered.len().saturating_sub(self.visible_rows);
        if self.offset > max_offset {
            self.offset = max_offset;
        }
    }
}

fn build_entries<'a>(
    profiles: &'a [SsoProfile],
    statuses: &[CacheStatus],
    current_profile: Option<&str>,
) -> Vec<PickerEntry<'a>> {
    profiles
        .iter()
        .enumerate()
        .map(|(original_index, profile)| PickerEntry {
            profile,
            status: statuses
                .get(original_index)
                .cloned()
                .unwrap_or_else(CacheStatus::unknown),
            is_current: Some(profile.name.as_str()) == current_profile,
            search_text: format!(
                "{} {} {} {}",
                profile.name,
                profile.role_name,
                profile.region.label(),
                profile.account_id
            )
            .to_ascii_lowercase(),
        })
        .collect()
}

pub fn is_prod_profile(profile_name: &str) -> bool {
    profile_name.to_ascii_lowercase().contains("prod")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aws_config::RegionDisplay;

    fn profile(name: &str) -> SsoProfile {
        SsoProfile {
            name: name.to_string(),
            sso_session: Some("corp".to_string()),
            sso_start_url: "https://example.awsapps.com/start".to_string(),
            sso_region: "us-east-1".to_string(),
            account_id: "123456789012".to_string(),
            role_name: "Admin".to_string(),
            region: RegionDisplay::Profile("us-east-1".to_string()),
        }
    }

    #[test]
    fn preserves_profile_order_and_marks_current() {
        let profiles = vec![profile("dev"), profile("prod"), profile("staging")];
        let statuses = vec![CacheStatus::unknown(); 3];
        let model = PickerModel::new(&profiles, &statuses, Some("prod"));

        let entries = model.visible_entries().collect::<Vec<_>>();
        assert_eq!(entries[0].1.profile.name, "dev");
        assert_eq!(entries[1].1.profile.name, "prod");
        assert!(entries[1].1.is_current);
    }

    #[test]
    fn filters_rows_and_selects_top_match_on_enter() {
        let profiles = vec![profile("dev"), profile("prod"), profile("staging")];
        let statuses = vec![CacheStatus::unknown(); 3];
        let mut model = PickerModel::new(&profiles, &statuses, None);

        for value in "stag".chars() {
            assert_eq!(
                model.handle(PickerCommand::Input(value)),
                PickerOutcome::Continue
            );
        }
        assert_eq!(model.filtered_len(), 1);
        assert_eq!(
            model.handle(PickerCommand::Enter),
            PickerOutcome::Selected("staging".to_string())
        );
    }

    #[test]
    fn number_hotkeys_select_visible_rows() {
        let profiles = vec![profile("dev"), profile("prod")];
        let statuses = vec![CacheStatus::unknown(); 2];
        let mut model = PickerModel::new(&profiles, &statuses, None);

        assert_eq!(
            model.handle(PickerCommand::JumpVisible(1)),
            PickerOutcome::Selected("prod".to_string())
        );
    }

    #[test]
    fn detects_prod_prompt_indicator() {
        assert!(is_prod_profile("acme-prod-readonly"));
        assert!(!is_prod_profile("acme-staging-dev"));
    }
}
