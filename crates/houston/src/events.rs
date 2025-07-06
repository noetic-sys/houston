use houston_core::AppState;
use houston_ui::{FocusedPanel, DialogType};
use crossterm::event::{KeyEvent, KeyCode};
use std::io;

pub async fn handle_key_event(key: KeyEvent, app: &mut AppState, search_mode: &mut bool) -> io::Result<bool> {
    match key.code {
        KeyCode::Char('q') => return Ok(true),
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
        // Open dropdown with space when focused on dropdown field (MUST come before general char handler)
        KeyCode::Char(' ') if app.dialog.is_some() => {
            if let Some(dialog) = &app.dialog {
                match &dialog.dialog_type {
                    DialogType::Input { .. } => {
                        app.dialog_open_dropdown();
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
        // Navigation keys routed to focused panel (only when no dialog is open)
        KeyCode::Up | KeyCode::Char('k') if !*search_mode && app.dialog.is_none() => {
            match app.focused_panel {
                FocusedPanel::Repos => app.previous_repo(),
                FocusedPanel::Actions => app.previous_action(),
                FocusedPanel::Tags => app.previous_tag(),
            }
        }
        KeyCode::Down | KeyCode::Char('j') if !*search_mode && app.dialog.is_none() => {
            match app.focused_panel {
                FocusedPanel::Repos => app.next_repo(),
                FocusedPanel::Actions => app.next_action(),
                FocusedPanel::Tags => app.next_tag(),
            }
        }
        KeyCode::Char(' ') if !*search_mode && app.focused_panel == FocusedPanel::Actions && app.dialog.is_none() => {
            app.execute_selected_action().await;
        }
        KeyCode::Tab if app.dialog.is_none() => app.focus_next_panel(),
        KeyCode::BackTab if app.dialog.is_none() => app.focus_prev_panel(),
        _ => {}
    }
    Ok(false)
} 