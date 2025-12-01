use houston_core::AppState;
use houston_ui::{FocusedPanel, DialogType, DialogFocus, View};
use crossterm::event::{KeyEvent, KeyCode, KeyModifiers};
use std::io;

/// Return value: (should_quit, should_load_logs)
pub async fn handle_key_event(key: KeyEvent, app: &mut AppState, search_mode: &mut bool) -> io::Result<(bool, bool)> {
    // Ctrl+C or Ctrl+Q to quit (always works)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') | KeyCode::Char('q') = key.code {
            return Ok((true, false));
        }
    }

    // Handle log viewer mode (takes over all input)
    if app.log_viewer.is_some() {
        match key.code {
            KeyCode::Esc => app.close_log_viewer(),
            KeyCode::Char('j') | KeyCode::Down => app.log_scroll_down(),
            KeyCode::Char('k') | KeyCode::Up => app.log_scroll_up(),
            KeyCode::Char('g') => app.log_scroll_top(),
            KeyCode::Char('G') => app.log_scroll_bottom(),
            _ => {}
        }
        return Ok((false, false));
    }

    // Handle view switching with number keys (1-5) when not in search mode or dialog
    if !*search_mode && app.dialog.is_none() {
        if let KeyCode::Char(c) = key.code {
            if let Some(view) = View::from_key(c) {
                app.set_view(view);
                return Ok((false, false));
            }
        }
    }

    match key.code {
        KeyCode::Char('/') if app.dialog.is_none() => {
            *search_mode = true;
            app.clear_search();
        }
        KeyCode::Esc if app.dialog.is_some() => {
            if let Some(dialog) = &app.dialog {
                match dialog.dialog_type {
                    DialogType::DropdownSelection { .. } => {
                        app.cancel_dropdown_selection();
                    }
                    DialogType::Input { .. } => {
                        app.close_dialog();
                    }
                }
            }
        }
        KeyCode::Esc => {
            *search_mode = false;
            app.clear_search();
        }
        KeyCode::Enter if *search_mode => {
            *search_mode = false;
        }
        // Enter in Runs view opens log viewer
        KeyCode::Enter if app.current_view == View::Runs && app.dialog.is_none() && !*search_mode => {
            app.open_log_viewer();
            return Ok((false, true)); // Signal to load logs
        }
        KeyCode::Char(c) if *search_mode => {
            app.add_to_search(c);
        }
        KeyCode::Backspace if *search_mode => {
            app.remove_from_search();
        }
        // Dialog input handling
        KeyCode::Tab if app.dialog.is_some() => {
            app.dialog_next_field();
        }
        KeyCode::BackTab if app.dialog.is_some() => {
            app.dialog_prev_field();
        }
        // Open dropdown or toggle boolean with space when focused on field (MUST come before general char handler)
        KeyCode::Char(' ') if app.dialog.is_some() => {
            if let Some(dialog) = &app.dialog {
                match &dialog.dialog_type {
                    DialogType::Input { fields, focus, .. } => {
                        // Check if current field is a boolean
                        if let DialogFocus::Field(idx) = focus {
                            if let Some(field) = fields.get(*idx) {
                                if matches!(field.input_type, houston_ui::InputType::Boolean { .. }) {
                                    app.dialog_toggle_boolean();
                                } else {
                                    app.dialog_open_dropdown();
                                }
                            }
                        }
                    }
                    DialogType::DropdownSelection { .. } => {
                        // Space in dropdown selection does nothing
                    }
                }
            }
        }
        // Dialog navigation with j/k (MUST come before general char handler)
        KeyCode::Up | KeyCode::Char('k') if app.dialog.is_some() => {
            app.dialog_navigate(-1);
        }
        KeyCode::Down | KeyCode::Char('j') if app.dialog.is_some() => {
            app.dialog_navigate(1);
        }
        KeyCode::Char(c) if app.dialog.is_some() => {
            // Handle special keys for dropdown selection
            if let Some(dialog) = &app.dialog {
                match &dialog.dialog_type {
                    DialogType::DropdownSelection { .. } => {
                        match c {
                            '/' => {
                                // Start/continue search mode - don't interfere with main search
                            }
                            _ => {
                                // All other characters are search input
                                app.dialog_search_char(c);
                            }
                        }
                    }
                    DialogType::Input { .. } => {
                        // All characters are input for text fields
                        app.dialog_input_char(c);
                    }
                }
            } else {
                app.dialog_input_char(c);
            }
        }
        KeyCode::Backspace if app.dialog.is_some() => {
            app.dialog_backspace();
        }
        KeyCode::Enter if app.dialog.is_some() => {
            app.submit_dialog().await;
        }
        // Navigation keys routed based on current view (only when no dialog is open)
        KeyCode::Up | KeyCode::Char('k') if !*search_mode && app.dialog.is_none() => {
            match app.current_view {
                View::Runs => app.previous_run(),
                View::Workflows => app.previous_action(),
                View::Tags => app.previous_tag(),
                View::Envs => app.previous_deployment(),
                View::Repos => {
                    match app.focused_panel {
                        FocusedPanel::Repos => app.previous_repo(),
                        FocusedPanel::Actions => app.previous_action(),
                        FocusedPanel::Tags => app.previous_tag(),
                    }
                }
            }
        }
        KeyCode::Down | KeyCode::Char('j') if !*search_mode && app.dialog.is_none() => {
            match app.current_view {
                View::Runs => app.next_run(),
                View::Workflows => app.next_action(),
                View::Tags => app.next_tag(),
                View::Envs => app.next_deployment(),
                View::Repos => {
                    match app.focused_panel {
                        FocusedPanel::Repos => app.next_repo(),
                        FocusedPanel::Actions => app.next_action(),
                        FocusedPanel::Tags => app.next_tag(),
                    }
                }
            }
        }
        KeyCode::Char(' ') if !*search_mode && app.focused_panel == FocusedPanel::Actions && app.dialog.is_none() => {
            app.execute_selected_action().await;
        }
        KeyCode::Tab if app.dialog.is_none() => app.focus_next_panel(),
        KeyCode::BackTab if app.dialog.is_none() => app.focus_prev_panel(),
        _ => {}
    }
    Ok((false, false))
}
