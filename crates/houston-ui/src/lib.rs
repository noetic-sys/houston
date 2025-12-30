use ratatui::{prelude::*, widgets::*};
use ratatui::text::{Span, Line};
use houston_api::{Action, Tag};
use houston_api::github::WorkflowInputField;

// New panel system modules
pub mod panel;
pub mod layout;
pub mod window_manager;
pub mod panels;

// Re-export key types from new modules
pub use panel::{PanelId, PanelContext};
pub use layout::{LayoutNode, PresetLayout, SplitDirection};
pub use window_manager::WindowManager;

// UI State Types

/// Main views in the application (k9s-style)
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum View {
    Repos,      // 1 - Repository list with status
    Workflows,  // 2 - Workflows for selected repo
    Runs,       // 3 - Recent runs with live status
    Envs,       // 4 - Environment deployments
    Tags,       // 5 - Release tags
}

impl View {
    pub fn name(&self) -> &'static str {
        match self {
            View::Repos => "Repos",
            View::Workflows => "Workflows",
            View::Runs => "Runs",
            View::Envs => "Environments",
            View::Tags => "Tags",
        }
    }

    pub fn from_key(c: char) -> Option<View> {
        match c {
            '1' => Some(View::Repos),
            '2' => Some(View::Workflows),
            '3' => Some(View::Runs),
            '4' => Some(View::Envs),
            '5' => Some(View::Tags),
            _ => None,
        }
    }
}

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
    Choice { options: Vec<String>, selected: usize },
    Boolean { value: bool },
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
                        InputType::Text => {
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
                        InputType::Boolean { value } => {
                            let checkbox = if *value { "[x]" } else { "[ ]" };
                            if is_focused {
                                format!("▶ {} (Space to toggle)", checkbox)
                            } else {
                                format!("  {}", checkbox)
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

// ============================================================================
// Header / Footer Chrome
// ============================================================================

/// Stats to display in header
#[derive(Default)]
pub struct HeaderStats {
    pub repo_count: usize,
    pub workflow_count: usize,
    pub runs_success: usize,
    pub runs_failed: usize,
    pub runs_pending: usize,
    pub deployments_count: usize,
}

pub fn render_header(
    f: &mut Frame,
    area: Rect,
    repo: Option<&str>,
    view: View,
    is_loading: bool,
    stats: &HeaderStats,
) {
    // Animated loading indicator
    let loading = if is_loading {
        let tick = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() / 100) % 4;
        match tick {
            0 => "◐",
            1 => "◓",
            2 => "◑",
            _ => "◒",
        }
    } else { "" };

    let repo_display = repo.unwrap_or("-");

    // Build header with stats
    let mut spans = vec![
        Span::styled("⚡HOUSTON", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::styled(" │ ", Style::default().fg(Color::DarkGray)),
    ];

    // Current repo (truncated if needed)
    let repo_short = if repo_display.len() > 30 {
        format!("...{}", &repo_display[repo_display.len()-27..])
    } else {
        repo_display.to_string()
    };
    spans.push(Span::styled(repo_short, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));

    // Resource counts based on view
    match view {
        View::Repos => {
            spans.push(Span::styled(
                format!("📦{}", stats.repo_count),
                Style::default().fg(Color::White)
            ));
        }
        View::Workflows => {
            spans.push(Span::styled(
                format!("⚙️ {}", stats.workflow_count),
                Style::default().fg(Color::White)
            ));
        }
        View::Runs => {
            // Status summary: ✓ 5  ● 2  ✗ 1
            spans.push(Span::styled(
                format!("✓{}", stats.runs_success),
                Style::default().fg(Color::Green)
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("●{}", stats.runs_pending),
                Style::default().fg(Color::Yellow)
            ));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format!("✗{}", stats.runs_failed),
                Style::default().fg(Color::Red)
            ));
        }
        View::Envs => {
            spans.push(Span::styled(
                format!("🌍{}", stats.deployments_count),
                Style::default().fg(Color::White)
            ));
        }
        View::Tags => {
            spans.push(Span::styled("🏷️ Tags", Style::default().fg(Color::White)));
        }
    }

    // View indicator
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        view.name().to_uppercase(),
        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)
    ));

    // Loading indicator
    if !loading.is_empty() {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(loading, Style::default().fg(Color::Yellow)));
    }

    let header = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(30, 30, 40)));

    f.render_widget(header, area);
}

pub fn render_footer(f: &mut Frame, area: Rect, view: View, notification: Option<&str>, help_open: bool) {
    // Show notification prominently if present
    if let Some(msg) = notification {
        let is_error = msg.to_lowercase().contains("fail") || msg.to_lowercase().contains("error");
        let (bg, fg) = if is_error {
            (Color::Red, Color::White)
        } else {
            (Color::Green, Color::Black)
        };
        let content = Line::from(Span::styled(
            format!(" {} ", msg),
            Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
        ));
        f.render_widget(Paragraph::new(content), area);
        return;
    }

    let mut spans: Vec<Span> = vec![];

    // View tabs with better styling
    let views = [
        (View::Repos, "1", "Repos"),
        (View::Workflows, "2", "Workflows"),
        (View::Runs, "3", "Runs"),
        (View::Envs, "4", "Envs"),
        (View::Tags, "5", "Tags"),
    ];

    for (v, key, name) in views {
        if v == view {
            spans.push(Span::styled(
                format!(" {}", key),
                Style::default().fg(Color::Black).bg(Color::Cyan)
            ));
            spans.push(Span::styled(
                format!(":{} ", name),
                Style::default().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            ));
        } else {
            spans.push(Span::styled(
                format!(" {}:{} ", key, name),
                Style::default().fg(Color::DarkGray)
            ));
        }
    }

    spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));

    // Contextual shortcuts
    let shortcuts = match view {
        View::Repos => vec![("↵", "select"), ("/", "filter"), ("?", "help")],
        View::Workflows => vec![("␣", "run"), ("↵", "details"), ("/", "filter")],
        View::Runs => vec![("↵", "jobs"), ("r", "refresh"), ("/", "filter")],
        View::Envs => vec![("↵", "details"), ("r", "refresh")],
        View::Tags => vec![("␣", "deploy"), ("↵", "details")],
    };

    for (key, action) in shortcuts {
        spans.push(Span::styled(format!(" {}", key), Style::default().fg(Color::Cyan)));
        spans.push(Span::styled(format!(":{}", action), Style::default().fg(Color::DarkGray)));
    }

    // Help hint
    spans.push(Span::styled(" │ ", Style::default().fg(Color::DarkGray)));
    spans.push(Span::styled(
        if help_open { "?:close" } else { "?:help" },
        Style::default().fg(Color::DarkGray)
    ));
    spans.push(Span::styled(" ^C", Style::default().fg(Color::Cyan)));
    spans.push(Span::styled(":quit", Style::default().fg(Color::DarkGray)));

    let footer = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Rgb(20, 20, 30)));

    f.render_widget(footer, area);
}

// ============================================================================
// Log Viewer Components
// ============================================================================

use houston_api::JobInfo;

#[derive(Debug, Clone)]
pub struct LogViewerState {
    pub run_id: String,
    pub workflow_name: String,
    pub jobs: Vec<JobInfo>,
    pub loading: bool,
    pub scroll_offset: usize,
}

impl LogViewerState {
    pub fn new(run_id: String, workflow_name: String) -> Self {
        Self {
            run_id,
            workflow_name,
            jobs: Vec::new(),
            loading: true,
            scroll_offset: 0,
        }
    }

    pub fn total_lines(&self) -> usize {
        let mut count = 0;
        for job in &self.jobs {
            count += 2; // Job header + blank line
            count += job.steps.len();
        }
        count
    }
}

pub fn render_log_viewer(f: &mut Frame, state: &LogViewerState) {
    let area = f.area();

    // Full screen log viewer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // Header
            Constraint::Min(1),     // Content
            Constraint::Length(1),  // Footer
        ])
        .split(area);

    // Header
    let status_indicator = if state.loading { "● Loading..." } else { "✓ Loaded" };
    let header_text = vec![
        Span::styled("Run Details: ", Style::default().fg(Color::Cyan)),
        Span::styled(&state.workflow_name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw(format!(" ({})", state.run_id)),
        Span::raw(" │ "),
        Span::styled(status_indicator, Style::default().fg(if state.loading { Color::Yellow } else { Color::Green })),
    ];
    let header = Paragraph::new(Line::from(header_text))
        .style(Style::default().bg(Color::DarkGray));
    f.render_widget(header, chunks[0]);

    // Build content lines
    let mut lines: Vec<Line> = Vec::new();

    if state.loading && state.jobs.is_empty() {
        lines.push(Line::from(Span::styled("Loading...", Style::default().fg(Color::DarkGray))));
    } else if state.jobs.is_empty() {
        lines.push(Line::from(Span::styled("No jobs found", Style::default().fg(Color::DarkGray))));
    } else {
        for job in &state.jobs {
            // Job header
            let (job_icon, job_color) = match job.conclusion.as_deref() {
                Some("success") => ("✓", Color::Green),
                Some("failure") => ("✗", Color::Red),
                Some("cancelled") => ("○", Color::DarkGray),
                Some("skipped") => ("⊘", Color::DarkGray),
                _ => ("●", Color::Yellow),
            };

            lines.push(Line::from(vec![
                Span::styled(format!("{} ", job_icon), Style::default().fg(job_color)),
                Span::styled(&job.name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(
                    format!(" ({})", job.conclusion.as_deref().unwrap_or(&job.status)),
                    Style::default().fg(job_color)
                ),
            ]));

            // Steps
            for step in &job.steps {
                let (icon, color) = match step.conclusion.as_deref() {
                    Some("success") => ("  ✓", Color::Green),
                    Some("failure") => ("  ✗", Color::Red),
                    Some("cancelled") => ("  ○", Color::DarkGray),
                    Some("skipped") => ("  ⊘", Color::DarkGray),
                    _ => ("  ●", Color::Yellow),
                };

                lines.push(Line::from(vec![
                    Span::styled(icon, Style::default().fg(color)),
                    Span::raw(" "),
                    Span::styled(&step.name, Style::default().fg(Color::White)),
                ]));
            }

            lines.push(Line::from("")); // Blank line between jobs
        }
    }

    // Apply scroll offset
    let visible_lines: Vec<Line> = lines.into_iter()
        .skip(state.scroll_offset)
        .collect();

    let content = Paragraph::new(visible_lines)
        .block(Block::default().borders(Borders::ALL).title("Jobs & Steps"));
    f.render_widget(content, chunks[1]);

    // Footer
    let footer_text = vec![
        Span::styled("Esc", Style::default().fg(Color::Cyan)),
        Span::raw(":back  "),
        Span::styled("j/k", Style::default().fg(Color::Cyan)),
        Span::raw(":scroll  "),
        Span::styled("g/G", Style::default().fg(Color::Cyan)),
        Span::raw(":top/bottom"),
    ];
    let footer = Paragraph::new(Line::from(footer_text))
        .style(Style::default().bg(Color::Black));
    f.render_widget(footer, chunks[2]);
}

// ============================================================================
// Runs View Components
// ============================================================================

use houston_api::ActionRun;

pub fn render_runs_view(f: &mut Frame, area: Rect, runs: &[ActionRun], selected: usize, loading: bool) {
    // Header row
    let header = Row::new(vec![
        Cell::from("STATUS").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("WORKFLOW").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("REF").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("STARTED").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("DURATION").style(Style::default().add_modifier(Modifier::BOLD)),
    ]).height(1);

    let rows: Vec<Row> = if loading && runs.is_empty() {
        vec![Row::new(vec![Cell::from("Loading...")]).style(Style::default().fg(Color::DarkGray))]
    } else if runs.is_empty() {
        vec![Row::new(vec![Cell::from("No runs found")]).style(Style::default().fg(Color::DarkGray))]
    } else {
        runs.iter().enumerate().map(|(i, run)| {
            let (icon, status_color) = match run.status.as_str() {
                "completed" | "success" => ("✓", Color::Green),
                "failure" | "failed" => ("✗", Color::Red),
                "in_progress" | "queued" | "pending" | "waiting" => ("●", Color::Yellow),
                "cancelled" => ("○", Color::DarkGray),
                _ => match run.conclusion.as_deref() {
                    Some("success") => ("✓", Color::Green),
                    Some("failure") => ("✗", Color::Red),
                    Some("cancelled") => ("○", Color::DarkGray),
                    _ => ("?", Color::Gray),
                }
            };

            let status_cell = Cell::from(format!("{} {}", icon, run.conclusion.as_deref().unwrap_or(&run.status)))
                .style(Style::default().fg(status_color));

            let branch_display = run.branch.as_deref().unwrap_or("-");

            // Format started time (simplified - just show the time part)
            let started = run.started_at.as_deref()
                .and_then(|s| s.split('T').nth(1))
                .and_then(|t| t.split('.').next())
                .unwrap_or("-");

            let style = if i == selected {
                Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![
                status_cell,
                Cell::from(run.workflow_name.clone()),
                Cell::from(branch_display.to_string()),
                Cell::from(started.to_string()),
                Cell::from("-"), // Duration would need calculation
            ]).style(style)
        }).collect()
    };

    let widths = [
        Constraint::Length(12),
        Constraint::Percentage(30),
        Constraint::Percentage(20),
        Constraint::Length(12),
        Constraint::Length(10),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title("Recent Runs"))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");

    f.render_widget(table, area);
}

// ============================================================================
// UI Helper Functions
// ============================================================================

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

pub fn determine_input_type(field: &WorkflowInputField, available_tags: &[String]) -> InputType {
    // Use the actual type from the YAML if available
    if let Some(input_type) = &field.input_type {
        match input_type.as_str() {
            "boolean" => {
                let default_bool = field.default.as_ref()
                    .map(|s| s.to_lowercase() == "true")
                    .unwrap_or(false);
                return InputType::Boolean { value: default_bool };
            }
            "choice" => {
                if let Some(options) = &field.options {
                    return InputType::Choice {
                        options: options.clone(),
                        selected: 0,
                    };
                }
            }
            "environment" => {
                // Only use choices if explicit options are provided
                if let Some(options) = &field.options {
                    return InputType::Choice {
                        options: options.clone(),
                        selected: 0,
                    };
                }
                // Otherwise, fall through to default text handling
            }
            "number" | "string" => {
                // Fall through to check for version/tag-related fields
            }
            _ => {
                // Unknown type, fall through to heuristics
            }
        }
    }
    
    // If no explicit type, use heuristics for common patterns
    let field_lower = field.name.to_lowercase();
    
    // Check for version-related fields that might use tags
    if field_lower.contains("version") || field_lower.contains("tag") || field_lower.contains("ref") {
        if !available_tags.is_empty() {
            return InputType::Dropdown {
                options: available_tags.to_vec(),
                selected: 0,
            };
        }
    }
    
    // Default to text input
    InputType::Text
}

// Convert API WorkflowInputField to UI WorkflowInputField
pub fn convert_workflow_input_field(input: WorkflowInputField, available_tags: &[String]) -> UIWorkflowInputField {
    let input_type = determine_input_type(&input, available_tags);
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
        InputType::Boolean { value } => (InputType::Boolean { value }, default_value.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use houston_api::github::WorkflowInputField;

    #[test]
    fn test_determine_input_type_boolean() {
        let field = WorkflowInputField {
            name: "debug".to_string(),
            required: false,
            description: Some("Enable debug mode".to_string()),
            default: Some("false".to_string()),
            input_type: Some("boolean".to_string()),
            options: None,
        };
        
        let input_type = determine_input_type(&field, &[]);
        match input_type {
            InputType::Boolean { value } => {
                assert_eq!(value, false);
            }
            _ => panic!("Expected Boolean type for boolean field"),
        }
    }

    #[test]
    fn test_determine_input_type_choice() {
        let field = WorkflowInputField {
            name: "environment".to_string(),
            required: true,
            description: Some("Deploy environment".to_string()),
            default: Some("staging".to_string()),
            input_type: Some("choice".to_string()),
            options: Some(vec!["dev".to_string(), "staging".to_string(), "prod".to_string()]),
        };
        
        let input_type = determine_input_type(&field, &[]);
        match input_type {
            InputType::Choice { options, .. } => {
                assert_eq!(options, vec!["dev", "staging", "prod"]);
            }
            _ => panic!("Expected Choice type for choice field"),
        }
    }

    #[test]
    fn test_determine_input_type_environment_with_options() {
        let field = WorkflowInputField {
            name: "env".to_string(),
            required: true,
            description: Some("Environment to deploy to".to_string()),
            default: None,
            input_type: Some("environment".to_string()),
            options: Some(vec!["dev".to_string(), "staging".to_string(), "prod".to_string()]),
        };
        
        let input_type = determine_input_type(&field, &[]);
        match input_type {
            InputType::Choice { options, .. } => {
                assert_eq!(options, vec!["dev", "staging", "prod"]);
            }
            _ => panic!("Expected Choice type for environment field with options"),
        }
    }

    #[test]
    fn test_determine_input_type_environment_without_options() {
        let field = WorkflowInputField {
            name: "environment".to_string(),
            required: true,
            description: Some("Environment to deploy to".to_string()),
            default: None,
            input_type: Some("environment".to_string()),
            options: None,
        };
        
        let input_type = determine_input_type(&field, &[]);
        match input_type {
            InputType::Text => {},
            _ => panic!("Expected Text type for environment field without options"),
        }
    }

    #[test]
    fn test_determine_input_type_version_with_tags() {
        let field = WorkflowInputField {
            name: "version".to_string(),
            required: true,
            description: Some("Version to deploy".to_string()),
            default: None,
            input_type: Some("string".to_string()),
            options: None,
        };
        
        let tags = vec!["v1.0.0".to_string(), "v1.1.0".to_string(), "v2.0.0".to_string()];
        let input_type = determine_input_type(&field, &tags);
        match input_type {
            InputType::Dropdown { options, .. } => {
                assert_eq!(options, tags);
            }
            _ => panic!("Expected Dropdown type for version field with tags"),
        }
    }

    #[test]
    fn test_determine_input_type_text_fallback() {
        let field = WorkflowInputField {
            name: "message".to_string(),
            required: false,
            description: Some("Commit message".to_string()),
            default: None,
            input_type: Some("string".to_string()),
            options: None,
        };
        
        let input_type = determine_input_type(&field, &[]);
        match input_type {
            InputType::Text => {},
            _ => panic!("Expected Text type for generic string field"),
        }
    }

    #[test]
    fn test_determine_input_type_legacy_fallback() {
        let field = WorkflowInputField {
            name: "enable_feature".to_string(),
            required: false,
            description: None,
            default: None,
            input_type: None, // No type specified in YAML
            options: None,
        };
        
        let input_type = determine_input_type(&field, &[]);
        match input_type {
            InputType::Text => {},
            _ => panic!("Expected Text type for field without type specification"),
        }
    }
}