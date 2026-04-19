use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use houston_core::AppState;
use houston_ui::{DialogFocus, DialogType, PanelId, PresetLayout};
use std::io;

/// Return value: (should_quit, should_load_logs)
pub async fn handle_key_event(
    key: KeyEvent,
    app: &mut AppState,
    search_mode: &mut bool,
) -> io::Result<(bool, bool)> {
    // Ctrl+C or Ctrl+Q to quit (always works)
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        if let KeyCode::Char('c') | KeyCode::Char('q') = key.code {
            return Ok((true, false));
        }
    }

    // Handle zoomed panel mode - z to unzoom, navigation within panel
    if app.window_manager.is_zoomed() {
        // If zoomed on Logs panel, handle log viewer navigation
        if app.window_manager.zoomed() == Some(PanelId::Logs) && app.log_viewer.is_some() {
            match key.code {
                KeyCode::Esc | KeyCode::Char('z') => {
                    app.window_manager.unzoom();
                }
                KeyCode::Char('j') | KeyCode::Down => app.log_scroll_down(),
                KeyCode::Char('k') | KeyCode::Up => app.log_scroll_up(),
                KeyCode::Char('g') => app.log_scroll_top(),
                KeyCode::Char('G') => app.log_scroll_bottom(),
                _ => {}
            }
            return Ok((false, false));
        }

        // For other zoomed panels, z or Esc unzooms
        match key.code {
            KeyCode::Esc | KeyCode::Char('z') => {
                app.window_manager.unzoom();
                return Ok((false, false));
            }
            // Fall through to normal navigation for the zoomed panel
            _ => {}
        }
    }

    // Handle preset switching with number keys (1-5) when not in search mode or dialog
    if !*search_mode && app.dialog.is_none() {
        if let KeyCode::Char(c) = key.code {
            if let Some(preset) = PresetLayout::from_key(c) {
                app.window_manager.apply_preset(preset);
                return Ok((false, false));
            }
        }
    }

    // Handle panel toggles and zoom when not in search mode or dialog
    if !*search_mode && app.dialog.is_none() {
        match key.code {
            // Panel visibility toggles
            KeyCode::Char('l') => {
                app.window_manager.toggle_panel(PanelId::Logs);
                return Ok((false, false));
            }
            KeyCode::Char('d') => {
                app.window_manager.toggle_panel(PanelId::Deployments);
                return Ok((false, false));
            }
            KeyCode::Char('t') if app.window_manager.current_preset() != PresetLayout::Tags => {
                // Only toggle if not in Tags preset (where tags is the main panel)
                app.window_manager.toggle_panel(PanelId::Tags);
                return Ok((false, false));
            }
            // Zoom toggle
            KeyCode::Char('z') => {
                app.window_manager.toggle_zoom();
                return Ok((false, false));
            }
            // Focus navigation with h (left)
            KeyCode::Char('h') => {
                app.window_manager.focus_left();
                return Ok((false, false));
            }
            _ => {}
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
        // Enter when Runs panel is focused opens log viewer
        KeyCode::Enter
            if app.window_manager.focused() == PanelId::Runs
                && app.dialog.is_none()
                && !*search_mode =>
        {
            app.open_log_viewer();
            // Show the logs panel if not visible
            if !app.window_manager.is_visible(PanelId::Logs) {
                app.window_manager.show_panel(PanelId::Logs);
            }
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
        // Open dropdown or toggle boolean with space when focused on field
        KeyCode::Char(' ') if app.dialog.is_some() => {
            if let Some(dialog) = &app.dialog {
                match &dialog.dialog_type {
                    DialogType::Input { fields, focus, .. } => {
                        if let DialogFocus::Field(idx) = focus {
                            if let Some(field) = fields.get(*idx) {
                                if matches!(field.input_type, houston_ui::InputType::Boolean { .. })
                                {
                                    app.dialog_toggle_boolean();
                                } else {
                                    app.dialog_open_dropdown();
                                }
                            }
                        }
                    }
                    DialogType::DropdownSelection { .. } => {}
                }
            }
        }
        // Dialog navigation with j/k
        KeyCode::Up | KeyCode::Char('k') if app.dialog.is_some() => {
            app.dialog_navigate(-1);
        }
        KeyCode::Down | KeyCode::Char('j') if app.dialog.is_some() => {
            app.dialog_navigate(1);
        }
        KeyCode::Char(c) if app.dialog.is_some() => {
            if let Some(dialog) = &app.dialog {
                match &dialog.dialog_type {
                    DialogType::DropdownSelection { .. } => {
                        app.dialog_search_char(c);
                    }
                    DialogType::Input { .. } => {
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
        // Navigation keys - route based on focused panel
        KeyCode::Up | KeyCode::Char('k') if !*search_mode && app.dialog.is_none() => {
            handle_panel_navigation(app, -1);
        }
        KeyCode::Down | KeyCode::Char('j') if !*search_mode && app.dialog.is_none() => {
            handle_panel_navigation(app, 1);
        }
        // Execute action when Workflows panel is focused
        KeyCode::Char(' ')
            if !*search_mode
                && app.window_manager.focused() == PanelId::Workflows
                && app.dialog.is_none() =>
        {
            app.execute_selected_action().await;
        }
        // Tab cycles focus between visible panels
        KeyCode::Tab if app.dialog.is_none() => {
            app.window_manager.focus_next();
        }
        KeyCode::BackTab if app.dialog.is_none() => {
            app.window_manager.focus_prev();
        }
        // Refresh runs
        KeyCode::Char('r')
            if !*search_mode
                && app.dialog.is_none()
                && app.window_manager.is_visible(PanelId::Runs) =>
        {
            app.start_loading_runs();
        }
        _ => {}
    }
    Ok((false, false))
}

/// Handle navigation within the currently focused panel
fn handle_panel_navigation(app: &mut AppState, direction: i32) {
    match app.window_manager.focused() {
        PanelId::Repos => {
            if direction > 0 {
                app.next_repo();
            } else {
                app.previous_repo();
            }
        }
        PanelId::Workflows => {
            if direction > 0 {
                app.next_action();
            } else {
                app.previous_action();
            }
        }
        PanelId::Runs => {
            if direction > 0 {
                app.next_run();
            } else {
                app.previous_run();
            }
        }
        PanelId::Logs => {
            if direction > 0 {
                app.log_scroll_down();
            } else {
                app.log_scroll_up();
            }
        }
        PanelId::Deployments => {
            if direction > 0 {
                app.next_deployment();
            } else {
                app.previous_deployment();
            }
        }
        PanelId::Tags => {
            if direction > 0 {
                app.next_tag();
            } else {
                app.previous_tag();
            }
        }
    }
}
