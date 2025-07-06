use ratatui::{prelude::*, widgets::*};
use ratatui::text::{Span, Line};
use houston_api::{Action, Tag};
use houston_api::github::WorkflowInputField;

// UI State Types
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum FocusedPanel {
    Repos,
    Actions,
    Tags,
}

#[derive(Debug, Clone)]
pub enum InputType {
    Text,
    Dropdown { options: Vec<String>, selected: usize },
    Environment,
    Choice { options: Vec<String>, selected: usize },
}

#[derive(Debug, Clone)]
pub struct UIWorkflowInputField {
    pub name: String,
    pub value: String,
    pub required: bool,
    pub description: Option<String>,
    pub input_type: InputType,
}

#[derive(Debug, Clone)]
pub enum DialogFocus {
    Field(usize),
    ConfirmButton,
    CancelButton,
}

#[derive(Debug, Clone)]
pub enum DialogType {
    Input {
        fields: Vec<UIWorkflowInputField>,
        focus: DialogFocus,
        repo_name: String,
        action_name: String,
        is_confirmation: bool,
    },
    DropdownSelection {
        field_name: String,
        options: Vec<String>,
        filtered_options: Vec<String>,
        selected: usize,
        search_query: String,
        target_field_index: usize,
        scroll_offset: usize,
    },
}

#[derive(Debug, Clone)]
pub struct DialogState {
    pub dialog_type: DialogType,
    pub open: bool,
}

// Panel Components
pub struct RepoListPanel<'a> {
    pub repos: &'a [String],
    pub selected: usize,
}

impl<'a> RepoListPanel<'a> {
    pub fn new(repos: &'a [String], selected: usize) -> Self {
        Self { repos, selected }
    }
}

pub struct ActionsPanel<'a> {
    pub actions: &'a [Action],
    pub selected: usize,
}

impl<'a> ActionsPanel<'a> {
    pub fn new(actions: &'a [Action], selected: usize) -> Self {
        Self { actions, selected }
    }
}

pub struct TagsPanel<'a> {
    pub tags: &'a [Tag],
    pub selected: usize,
}

impl<'a> TagsPanel<'a> {
    pub fn new(tags: &'a [Tag], selected: usize) -> Self {
        Self { tags, selected }
    }
}

// Rendering Functions
pub fn render_repo_list_panel(
    f: &mut Frame,
    area: Rect,
    panel: &RepoListPanel,
    state: &mut ListState,
) {
    let items: Vec<ListItem> = panel
        .repos
        .iter()
        .map(|r| ListItem::new(r.as_str()))
        .collect();
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Repositories"))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    state.select(Some(panel.selected));
    f.render_stateful_widget(list, area, state);
}

pub fn render_repo_list_panel_with_title_and_style(
    f: &mut Frame,
    area: Rect,
    panel: &RepoListPanel,
    state: &mut ListState,
    title: &str,
    style: Style,
    loading: bool,
) {
    let items: Vec<ListItem> = if loading {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else {
        panel
            .repos
            .iter()
            .map(|r| ListItem::new(r.as_str()))
            .collect()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title).border_style(style))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));
    if !panel.repos.is_empty() {
        state.select(Some(panel.selected));
    } else {
        state.select(None);
    }
    f.render_stateful_widget(list, area, state);
}

pub fn render_actions_panel(
    f: &mut Frame,
    area: Rect,
    panel: &ActionsPanel,
    state: &mut ListState,
    loading: bool,
    border_style: Style,
) {
    let items: Vec<ListItem> = if loading && panel.actions.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else {
        panel
            .actions
            .iter()
            .map(|a| {
                let text = if let Some(desc) = &a.description {
                    format!("{} - {}", a.name, desc)
                } else {
                    a.name.clone()
                };
                ListItem::new(text)
            })
            .collect()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Actions").border_style(border_style))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));
    if !panel.actions.is_empty() {
        state.select(Some(panel.selected));
    } else {
        state.select(None);
    }
    f.render_stateful_widget(list, area, state);
}

pub fn render_tags_panel(
    f: &mut Frame,
    area: Rect,
    panel: &TagsPanel,
    state: &mut ListState,
    loading: bool,
    border_style: Style,
) {
    let items: Vec<ListItem> = if loading && panel.tags.is_empty() {
        vec![ListItem::new("Loading...").style(Style::default().fg(Color::DarkGray))]
    } else {
        panel
            .tags
            .iter()
            .map(|t| ListItem::new(t.name.as_str()))
            .collect()
    };
    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title("Tags").border_style(border_style))
        .highlight_symbol("▶ ")
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    if !panel.tags.is_empty() {
        state.select(Some(panel.selected));
    } else {
        state.select(None);
    }
    f.render_stateful_widget(list, area, state);
}

// Dialog Rendering
pub fn render_dialog(f: &mut Frame, dialog: &DialogState) {
    match &dialog.dialog_type {
        DialogType::Input { fields, focus, repo_name, action_name, is_confirmation } => {
            let area = centered_rect(70, 50, f.area());
            let mut lines = vec![];
            
            if *is_confirmation {
                // Confirmation dialog
                lines.push(Line::from(Span::styled(
                    format!("Are you sure you want to run '{}' on '{}'?", action_name, repo_name),
                    Style::default().fg(Color::White)
                )));
                lines.push(Line::from(""));
            } else {
                // Input dialog - show fields
                for (i, field) in fields.iter().enumerate() {
                    let is_focused = matches!(focus, DialogFocus::Field(idx) if *idx == i);
                    
                    // Field name and description
                    let mut field_name = field.name.clone();
                    if field.required {
                        field_name.push_str(" *");
                    }
                    if let Some(desc) = &field.description {
                        field_name.push_str(&format!(" ({})", desc));
                    }
                    
                    let name_style = if is_focused {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };
                    lines.push(Line::from(Span::styled(field_name, name_style)));
                    
                    // Field value display
                    let value_display = match &field.input_type {
                        InputType::Text | InputType::Environment => {
                            if is_focused {
                                format!("▶ {}", field.value)
                            } else {
                                format!("  {}", field.value)
                            }
                        }
                        InputType::Dropdown { options, .. } | InputType::Choice { options, .. } => {
                            if is_focused {
                                format!("▶ {} (Space to open selection, {} options)", field.value, options.len())
                            } else {
                                format!("  {} (dropdown)", field.value)
                            }
                        }
                    };
                    
                    let value_style = if is_focused {
                        Style::default().fg(Color::White).bg(Color::Blue)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    lines.push(Line::from(Span::styled(value_display, value_style)));
                    lines.push(Line::from(""));
                }
            }
            
            // Add buttons
            lines.push(Line::from(""));
            
            let confirm_style = if matches!(focus, DialogFocus::ConfirmButton) {
                Style::default().fg(Color::Black).bg(Color::Green).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
            };
            
            let cancel_style = if matches!(focus, DialogFocus::CancelButton) {
                Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
            };
            
            let button_line = Line::from(vec![
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("Confirm", confirm_style),
                Span::styled("]", Style::default().fg(Color::Gray)),
                Span::styled("  [", Style::default().fg(Color::Gray)),
                Span::styled("Cancel", cancel_style),
                Span::styled("]", Style::default().fg(Color::Gray)),
            ]);
            lines.push(button_line);
            
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Tab/Shift+Tab: Navigate • Enter: Select • Escape: Cancel",
                Style::default().fg(Color::Gray)
            )));
            
            let title = if *is_confirmation {
                "Confirm Action"
            } else {
                "Workflow Inputs"
            };
            
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow));
            let para = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(Clear, area); // Clear the area beneath the modal
            f.render_widget(para, area);
        }
        DialogType::DropdownSelection { field_name, filtered_options, selected, search_query, scroll_offset, .. } => {
            let area = centered_rect(60, 70, f.area());
            let mut lines = vec![];
            
            // Title and search info
            lines.push(Line::from(Span::styled(
                format!("Select {}", field_name),
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            )));
            lines.push(Line::from(""));
            
            // Search query
            if !search_query.is_empty() {
                lines.push(Line::from(Span::styled(
                    format!("Search: {}", search_query),
                    Style::default().fg(Color::Cyan)
                )));
            } else {
                lines.push(Line::from(Span::styled(
                    "Search: (type to filter)",
                    Style::default().fg(Color::Gray)
                )));
            }
            lines.push(Line::from(""));
            
            // Calculate available space for options (total area minus header and footer)
            let available_height = area.height.saturating_sub(8); // Leave space for title, search, help, borders
            let max_visible_items = available_height as usize;
            
            // Show scroll indicators if needed
            let total_items = filtered_options.len();
            let has_more_above = *scroll_offset > 0;
            let has_more_below = *scroll_offset + max_visible_items < total_items;
            
            if has_more_above {
                lines.push(Line::from(Span::styled(
                    "  ↑ More options above ↑",
                    Style::default().fg(Color::Cyan)
                )));
            }
            
            // Options list (only show visible portion)
            for (i, option) in filtered_options.iter().enumerate().skip(*scroll_offset).take(max_visible_items) {
                let style = if i == *selected {
                    Style::default().fg(Color::Black).bg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                
                let prefix = if i == *selected { "▶ " } else { "  " };
                lines.push(Line::from(Span::styled(
                    format!("{}{}", prefix, option),
                    style
                )));
            }
            
            if has_more_below {
                lines.push(Line::from(Span::styled(
                    "  ↓ More options below ↓",
                    Style::default().fg(Color::Cyan)
                )));
            }
            
            if filtered_options.is_empty() {
                lines.push(Line::from(Span::styled(
                    "No matches found",
                    Style::default().fg(Color::Red)
                )));
            }
            
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "↑↓ or j/k: Navigate • Enter: Select • Escape: Cancel • Type: Search",
                Style::default().fg(Color::Gray)
            )));
            
            let block = Block::default()
                .title("Select Option")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan));
            let para = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(Clear, area); // Clear the area beneath the modal
            f.render_widget(para, area);
        }
    }
}

// UI Helper Functions
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ].as_ref())
        .split(r);
    let vertical = popup_layout[1];
    let popup_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ].as_ref())
        .split(vertical);
    popup_layout[1]
}

pub fn determine_input_type(field_name: &str, available_tags: &[String]) -> InputType {
    let field_lower = field_name.to_lowercase();
    
    // Check for version-related fields
    if field_lower.contains("version") || field_lower.contains("tag") || field_lower.contains("ref") {
        if !available_tags.is_empty() {
            return InputType::Dropdown {
                options: available_tags.to_vec(),
                selected: 0,
            };
        }
    }
    
    // Check for environment fields
    if field_lower.contains("environment") || field_lower.contains("env") {
        return InputType::Choice {
            options: vec![
                "production".to_string(),
                "staging".to_string(),
                "development".to_string(),
                "test".to_string(),
            ],
            selected: 0,
        };
    }
    
    // Check for boolean-like fields
    if field_lower.contains("enable") || field_lower.contains("disable") || field_lower.contains("debug") {
        return InputType::Choice {
            options: vec!["true".to_string(), "false".to_string()],
            selected: 0,
        };
    }
    
    // Default to text input
    InputType::Text
}

// Convert API WorkflowInputField to UI WorkflowInputField
pub fn convert_workflow_input_field(input: WorkflowInputField, available_tags: &[String]) -> UIWorkflowInputField {
    let input_type = determine_input_type(&input.name, available_tags);
    let default_value = input.default.unwrap_or_default();
    
    let (final_input_type, final_value) = match input_type {
        InputType::Dropdown { options, .. } | InputType::Choice { options, .. } => {
            let selected = if !default_value.is_empty() {
                options.iter().position(|opt| opt == &default_value).unwrap_or(0)
            } else {
                0
            };
            let value = if !options.is_empty() {
                options[selected].clone()
            } else {
                default_value
            };
            (InputType::Dropdown { options, selected }, value)
        }
        _ => (input_type, default_value)
    };
    
    UIWorkflowInputField {
        name: input.name,
        value: final_value,
        required: input.required,
        description: input.description,
        input_type: final_input_type,
    }
}