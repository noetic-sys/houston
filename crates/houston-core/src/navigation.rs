use crate::AppState;
use houston_ui::{FocusedPanel, DialogType, DialogFocus, InputType};

impl AppState {
    // Search functionality
    pub fn filter_repos(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_repos = self.repos.clone();
        } else {
            self.filtered_repos = self.repos
                .iter()
                .filter(|repo| repo.to_lowercase().contains(&self.search_query.to_lowercase()))
                .cloned()
                .collect();
        }
        
        // Adjust selection if current selection is out of bounds
        if self.selected_repo >= self.filtered_repos.len() {
            self.selected_repo = if self.filtered_repos.is_empty() { 0 } else { self.filtered_repos.len() - 1 };
        }
        
        if !self.filtered_repos.is_empty() {
            self.repo_list_state.select(Some(self.selected_repo));
        } else {
            self.repo_list_state.select(None);
        }
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

    // Repository navigation
    pub fn next_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = (self.selected_repo + 1) % self.filtered_repos.len();
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    pub fn previous_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = if self.selected_repo == 0 {
                self.filtered_repos.len() - 1
            } else {
                self.selected_repo - 1
            };
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    // Action navigation
    pub fn next_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = (self.selected_action + 1) % self.actions.len();
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    pub fn previous_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = if self.selected_action == 0 {
                self.actions.len() - 1
            } else {
                self.selected_action - 1
            };
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    // Tag navigation
    pub fn next_tag(&mut self) {
        if !self.tags.is_empty() {
            self.selected_tag = (self.selected_tag + 1) % self.tags.len();
            self.tags_list_state.select(Some(self.selected_tag));
        }
    }

    pub fn previous_tag(&mut self) {
        if !self.tags.is_empty() {
            self.selected_tag = if self.selected_tag == 0 {
                self.tags.len() - 1
            } else {
                self.selected_tag - 1
            };
            self.tags_list_state.select(Some(self.selected_tag));
        }
    }

    // Panel focus management
    pub fn focus_next_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Repos => FocusedPanel::Actions,
            FocusedPanel::Actions => FocusedPanel::Tags,
            FocusedPanel::Tags => FocusedPanel::Repos,
        };
    }

    pub fn focus_prev_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Repos => FocusedPanel::Tags,
            FocusedPanel::Actions => FocusedPanel::Repos,
            FocusedPanel::Tags => FocusedPanel::Actions,
        };
    }

    // Dialog navigation
    pub fn dialog_next_field(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    *focus = match *focus {
                        DialogFocus::Field(idx) => {
                            if idx + 1 < fields.len() {
                                DialogFocus::Field(idx + 1)
                            } else {
                                DialogFocus::ConfirmButton
                            }
                        }
                        DialogFocus::ConfirmButton => DialogFocus::CancelButton,
                        DialogFocus::CancelButton => {
                            if !fields.is_empty() {
                                DialogFocus::Field(0)
                            } else {
                                DialogFocus::ConfirmButton
                            }
                        }
                    };
                }
                DialogType::DropdownSelection { .. } => {
                    // Tab does nothing in dropdown selection mode
                }
            }
        }
    }

    pub fn dialog_prev_field(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    *focus = match *focus {
                        DialogFocus::Field(idx) => {
                            if idx > 0 {
                                DialogFocus::Field(idx - 1)
                            } else {
                                DialogFocus::CancelButton
                            }
                        }
                        DialogFocus::ConfirmButton => {
                            if !fields.is_empty() {
                                DialogFocus::Field(fields.len() - 1)
                            } else {
                                DialogFocus::CancelButton
                            }
                        }
                        DialogFocus::CancelButton => DialogFocus::ConfirmButton,
                    };
                }
                DialogType::DropdownSelection { .. } => {
                    // Shift+Tab does nothing in dropdown selection mode
                }
            }
        }
    }

    pub fn dialog_input_char(&mut self, c: char) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    if let DialogFocus::Field(idx) = *focus {
                        if let Some(field) = fields.get_mut(idx) {
                            match &field.input_type {
                                InputType::Text | InputType::Environment => {
                                    field.value.push(c);
                                }
                                InputType::Dropdown { .. } | InputType::Choice { .. } => {
                                    // For dropdown/choice, open selection dialog instead
                                }
                            }
                        }
                    }
                }
                DialogType::DropdownSelection { .. } => {
                    // Character input is handled separately for dropdown search
                }
            }
        }
    }

    pub fn dialog_search_char(&mut self, c: char) {
        if let Some(dialog) = &mut self.dialog {
            if let DialogType::DropdownSelection { search_query, options, filtered_options, selected, scroll_offset, .. } = &mut dialog.dialog_type {
                // Add character to search query
                search_query.push(c);
                
                // Filter options based on search query
                *filtered_options = options.iter()
                    .filter(|opt| opt.to_lowercase().contains(&search_query.to_lowercase()))
                    .cloned()
                    .collect();
                
                // Reset selection and scroll to first filtered option
                *selected = 0;
                *scroll_offset = 0;
            }
        }
    }

    pub fn dialog_backspace(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    if let DialogFocus::Field(idx) = *focus {
                        if let Some(field) = fields.get_mut(idx) {
                            match &field.input_type {
                                InputType::Text | InputType::Environment => {
                                    field.value.pop();
                                }
                                InputType::Dropdown { .. } | InputType::Choice { .. } => {
                                    // For dropdown/choice, backspace doesn't do anything
                                }
                            }
                        }
                    }
                }
                DialogType::DropdownSelection { search_query, options, filtered_options, selected, scroll_offset, .. } => {
                    // Remove character from search query
                    search_query.pop();
                    
                    // Re-filter options based on updated search query
                    *filtered_options = if search_query.is_empty() {
                        options.clone()
                    } else {
                        options.iter()
                            .filter(|opt| opt.to_lowercase().contains(&search_query.to_lowercase()))
                            .cloned()
                            .collect()
                    };
                    
                    // Reset selection and scroll to first filtered option
                    *selected = 0;
                    *scroll_offset = 0;
                }
            }
        }
    }

    pub fn dialog_navigate(&mut self, direction: i32) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { .. } => {
                    // Navigate through fields and buttons in input dialog
                    if direction > 0 {
                        self.dialog_next_field();
                    } else {
                        self.dialog_prev_field();
                    }
                }
                DialogType::DropdownSelection { filtered_options, selected, scroll_offset, .. } => {
                    // Navigate through options in dropdown selection with scrolling
                    if !filtered_options.is_empty() {
                        if direction > 0 {
                            *selected = (*selected + 1) % filtered_options.len();
                        } else {
                            *selected = if *selected == 0 { 
                                filtered_options.len() - 1 
                            } else { 
                                *selected - 1 
                            };
                        }
                        
                        // Update scroll offset to keep selected item visible
                        // Use a reasonable default that matches the UI calculation
                        let visible_items = 15; // This should match the UI calculation
                        
                        if *selected < *scroll_offset {
                            // Selected item is above visible area, scroll up
                            *scroll_offset = *selected;
                        } else if *selected >= *scroll_offset + visible_items {
                            // Selected item is below visible area, scroll down
                            *scroll_offset = (*selected + 1).saturating_sub(visible_items);
                        }
                    }
                }
            }
        }
    }

    pub fn dialog_open_dropdown(&mut self) {
        let should_open_dropdown = if let Some(dialog) = &self.dialog {
            match &dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    if let DialogFocus::Field(idx) = *focus {
                        if let Some(field) = fields.get(idx) {
                            match &field.input_type {
                                InputType::Dropdown { options, .. } | InputType::Choice { options, .. } => {
                                    Some((field.name.clone(), options.clone(), idx))
                                }
                                _ => None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                DialogType::DropdownSelection { .. } => None,
            }
        } else {
            None
        };
        
        if let Some((field_name, options, idx)) = should_open_dropdown {
            self.open_dropdown_selection(field_name, options, idx);
        }
    }

    pub fn dialog_navigate_dropdown(&mut self, direction: i32) {
        let should_open_dropdown = if let Some(dialog) = &self.dialog {
            match &dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
                    if let DialogFocus::Field(idx) = *focus {
                        if let Some(field) = fields.get(idx) {
                            match &field.input_type {
                                InputType::Dropdown { options, .. } | InputType::Choice { options, .. } => {
                                    Some((field.name.clone(), options.clone(), idx))
                                }
                                _ => None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                DialogType::DropdownSelection { .. } => None,
            }
        } else {
            None
        };
        
        if let Some((field_name, options, idx)) = should_open_dropdown {
            self.open_dropdown_selection(field_name, options, idx);
            return;
        }
        
        // Handle navigation within dropdown selection
        if let Some(dialog) = &mut self.dialog {
            if let DialogType::DropdownSelection { filtered_options, selected, .. } = &mut dialog.dialog_type {
                if !filtered_options.is_empty() {
                    if direction > 0 {
                        *selected = (*selected + 1) % filtered_options.len();
                    } else {
                        *selected = if *selected == 0 { 
                            filtered_options.len() - 1 
                        } else { 
                            *selected - 1 
                        };
                    }
                }
            }
        }
    }
} 