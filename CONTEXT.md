# awsp Context

## Domain Terms

### SSO Profile Inventory

The normalized view of complete usable AWS SSO profiles, SSO sessions, and malformed profile diagnostics loaded from the AWS config file.

### Profile Activation

The workflow that selects a profile, checks local SSO cache status, optionally runs AWS SSO login, records the selected profile in awsp state, and emits shell code when running through shell integration.

### Shell Integration

The zsh/bash function and startup-file hook that allow awsp to update the current shell by evaluating shell-safe code printed by the hidden `awsp __shell` command.

### Terminal Picker

The built-in interactive profile chooser. Its decision model owns filtering, visible-row number hotkeys, selection, re-login requests, and navigation; the terminal adapter owns crossterm rendering and input translation.

### Output Contract

The rule that all human UI goes to stderr while stdout is reserved for shell code in shell-mode commands and machine-readable command output elsewhere.

### Profile Query

A direct profile fragment, such as `prod` in `awsp prod`. Exact profile names win first, then case-insensitive substring matches. One match switches, multiple matches render disambiguation, and zero matches render typo suggestions.

### Prompt Indicator

The shell-facing `AWSP_PROD` export. awsp sets it only when the selected profile name contains `prod`, allowing the user’s prompt to render a production marker without awsp reintroducing env-colored UI pills.
