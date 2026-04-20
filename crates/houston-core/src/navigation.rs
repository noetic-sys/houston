use crate::AppState;
use houston_ui::{DialogFocus, DialogType, InputType};

impl AppState {
    // =========================================================================
    // Search
    // =========================================================================

    pub fn filter_repos(&mut self) {
        let base: Vec<String> = if self.search_query.is_empty() {
            self.repos.clone()
        } else {
            let query = self.search_query.to_lowercase();
            self.repos
                .iter()
                .filter(|r| r.to_lowercase().contains(&query))
                .cloned()
                .collect()
        };
        self.filtered_repos = crate::sorted_with_pins(&base, &self.pinned_repos);

        // Clamp selection
        if self.selected_repo >= self.filtered_repos.len() {
            self.selected_repo = self.filtered_repos.len().saturating_sub(1);
        }
        self.repo_list_state
            .select((!self.filtered_repos.is_empty()).then_some(self.selected_repo));
    }

    pub fn add_to_search(&mut self, c: char) {
        self.search_query.push(c);
        self.filter_repos();
        self.needs_action_reload = true;
    }

    pub fn remove_from_search(&mut self) {
        self.search_query.pop();
        self.filter_repos();
        self.needs_action_reload = true;
    }

    pub fn clear_search(&mut self) {
        self.search_query.clear();
        self.filter_repos();
        self.needs_action_reload = true;
    }

    // =========================================================================
    // List Navigation (repos, actions, tags)
    // =========================================================================

    pub fn next_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = (self.selected_repo + 1) % self.filtered_repos.len();
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    pub fn previous_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = self
                .selected_repo
                .checked_sub(1)
                .unwrap_or(self.filtered_repos.len() - 1);
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    pub fn next_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = (self.selected_action + 1) % self.actions.len();
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    pub fn previous_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = self
                .selected_action
                .checked_sub(1)
                .unwrap_or(self.actions.len() - 1);
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    pub fn next_tag(&mut self) {
        if !self.tags.is_empty() {
            self.selected_tag = (self.selected_tag + 1) % self.tags.len();
            self.tags_list_state.select(Some(self.selected_tag));
        }
    }

    pub fn previous_tag(&mut self) {
        if !self.tags.is_empty() {
            self.selected_tag = self
                .selected_tag
                .checked_sub(1)
                .unwrap_or(self.tags.len() - 1);
            self.tags_list_state.select(Some(self.selected_tag));
        }
    }

    // =========================================================================
    // Dialog Navigation
    // =========================================================================

    pub fn dialog_next_field(&mut self) {
        if let Some(dialog) = &mut self.dialog
            && let DialogType::Input { fields, focus, .. } = &mut dialog.dialog_type
        {
            *focus = match *focus {
                DialogFocus::Field(i) if i + 1 < fields.len() => DialogFocus::Field(i + 1),
                DialogFocus::Field(_) => DialogFocus::ConfirmButton,
                DialogFocus::ConfirmButton => DialogFocus::CancelButton,
                DialogFocus::CancelButton if !fields.is_empty() => DialogFocus::Field(0),
                DialogFocus::CancelButton => DialogFocus::ConfirmButton,
            };
        }
    }

    pub fn dialog_prev_field(&mut self) {
        if let Some(dialog) = &mut self.dialog
            && let DialogType::Input { fields, focus, .. } = &mut dialog.dialog_type
        {
            *focus = match *focus {
                DialogFocus::Field(0) => DialogFocus::CancelButton,
                DialogFocus::Field(i) => DialogFocus::Field(i - 1),
                DialogFocus::ConfirmButton if !fields.is_empty() => {
                    DialogFocus::Field(fields.len() - 1)
                }
                DialogFocus::ConfirmButton => DialogFocus::CancelButton,
                DialogFocus::CancelButton => DialogFocus::ConfirmButton,
            };
        }
    }

    pub fn dialog_navigate(&mut self, direction: i32) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { .. } => {
                    if direction > 0 {
                        self.dialog_next_field();
                    } else {
                        self.dialog_prev_field();
                    }
                }
                DialogType::DropdownSelection {
                    filtered_options,
                    selected,
                    scroll_offset,
                    ..
                } => {
                    if filtered_options.is_empty() {
                        return;
                    }

                    *selected = if direction > 0 {
                        (*selected + 1) % filtered_options.len()
                    } else if *selected == 0 {
                        filtered_options.len() - 1
                    } else {
                        *selected - 1
                    };

                    // Keep selection visible (15 items max visible)
                    const VISIBLE: usize = 15;
                    if *selected < *scroll_offset {
                        *scroll_offset = *selected;
                    } else if *selected >= *scroll_offset + VISIBLE {
                        *scroll_offset = *selected + 1 - VISIBLE;
                    }
                }
            }
        }
    }

    // =========================================================================
    // Dialog Input
    // =========================================================================

    pub fn dialog_input_char(&mut self, c: char) {
        if let Some(dialog) = &mut self.dialog
            && let DialogType::Input { fields, focus, .. } = &mut dialog.dialog_type
            && let DialogFocus::Field(idx) = *focus
            && let Some(field) = fields.get_mut(idx)
            && let InputType::Text = field.input_type
        {
            field.value.push(c);
        }
    }

    pub fn dialog_search_char(&mut self, c: char) {
        if let Some(dialog) = &mut self.dialog
            && let DialogType::DropdownSelection {
                search_query,
                options,
                filtered_options,
                selected,
                scroll_offset,
                ..
            } = &mut dialog.dialog_type
        {
            search_query.push(c);
            let query = search_query.to_lowercase();
            *filtered_options = options
                .iter()
                .filter(|o| o.to_lowercase().contains(&query))
                .cloned()
                .collect();
            *selected = 0;
            *scroll_offset = 0;
        }
    }

    pub fn dialog_backspace(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    if let DialogFocus::Field(idx) = *focus
                        && let Some(field) = fields.get_mut(idx)
                        && let InputType::Text = field.input_type
                    {
                        field.value.pop();
                    }
                }
                DialogType::DropdownSelection {
                    search_query,
                    options,
                    filtered_options,
                    selected,
                    scroll_offset,
                    ..
                } => {
                    search_query.pop();
                    let query = search_query.to_lowercase();
                    *filtered_options = if query.is_empty() {
                        options.clone()
                    } else {
                        options
                            .iter()
                            .filter(|o| o.to_lowercase().contains(&query))
                            .cloned()
                            .collect()
                    };
                    *selected = 0;
                    *scroll_offset = 0;
                }
            }
        }
    }

    pub fn dialog_open_dropdown(&mut self) {
        let info = if let Some(dialog) = &self.dialog
            && let DialogType::Input { fields, focus, .. } = &dialog.dialog_type
            && let DialogFocus::Field(idx) = *focus
        {
            fields.get(idx).and_then(|f| match &f.input_type {
                InputType::Dropdown { options, .. } | InputType::Choice { options, .. } => {
                    Some((f.name.clone(), options.clone(), idx))
                }
                _ => None,
            })
        } else {
            None
        };

        if let Some((name, options, idx)) = info {
            self.open_dropdown_selection(name, options, idx);
        }
    }

    pub fn dialog_toggle_boolean(&mut self) {
        if let Some(dialog) = &mut self.dialog
            && let DialogType::Input { fields, focus, .. } = &mut dialog.dialog_type
            && let DialogFocus::Field(idx) = *focus
            && let Some(field) = fields.get_mut(idx)
            && let InputType::Boolean { value } = &mut field.input_type
        {
            *value = !*value;
            field.value = value.to_string();
        }
    }
}
