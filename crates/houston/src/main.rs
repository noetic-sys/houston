use houston_api::{GitHubProvider, VcsProvider, Action};
use houston_ui::{RepoListPanel, render_repo_list_panel, ActionsPanel, render_actions_panel, TagsPanel, render_tags_panel};
use ratatui::{prelude::*, widgets::{ListState, Paragraph, Block, Borders, ListItem, List, Clear, Wrap}};
use ratatui::text::{Span, Line};
// Remove: use ratatui::widgets::{Span, Spans};
// Span and Spans should be available via ratatui::prelude::*
use std::io;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[derive(Copy, Clone, PartialEq, Eq)]
enum FocusedPanel {
    Repos,
    Actions,
    Tags,
}

#[derive(Debug, Clone)]
enum InputType {
    Text,
    Dropdown { options: Vec<String>, selected: usize },
    Environment,
    Choice { options: Vec<String>, selected: usize },
}

#[derive(Debug, Clone)]
struct WorkflowInputField {
    name: String,
    value: String,
    required: bool,
    description: Option<String>,
    input_type: InputType,
}

#[derive(Debug, Clone)]
enum DialogFocus {
    Field(usize),
    ConfirmButton,
    CancelButton,
}

#[derive(Debug, Clone)]
enum DialogType {
    Input {
        fields: Vec<WorkflowInputField>,
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
struct DialogState {
    dialog_type: DialogType,
    open: bool,
}

struct AppState {
    repos: Vec<String>,
    filtered_repos: Vec<String>,
    search_query: String,
    selected_repo: usize,
    repo_list_state: ListState,
    actions: Vec<Action>,
    selected_action: usize,
    actions_list_state: ListState,
    provider: GitHubProvider,
    needs_action_reload: bool,
    last_action_load: Option<Instant>,
    action_load_debounce: Duration,
    notification: Option<String>,
    notification_time: Option<Instant>,
    tags: Vec<houston_api::Tag>,
    selected_tag: usize,
    tags_list_state: ListState,
    tags_loading: bool,
    actions_loading: bool,
    focused_panel: FocusedPanel,
    dialog: Option<DialogState>,
    previous_dialog: Option<DialogState>,
}

impl AppState {
    fn new(repos: Vec<String>, provider: GitHubProvider) -> Self {
        let mut repo_list_state = ListState::default();
        let selected_repo = 0;
        let filtered_repos = repos.clone();
        if !filtered_repos.is_empty() {
            repo_list_state.select(Some(selected_repo));
        }
        Self { 
            repos, 
            filtered_repos,
            search_query: String::new(),
            selected_repo, 
            repo_list_state,
            actions: Vec::new(),
            selected_action: 0,
            actions_list_state: ListState::default(),
            provider,
            needs_action_reload: true,
            last_action_load: None,
            action_load_debounce: Duration::from_millis(500),
            notification: None,
            notification_time: None,
            tags: Vec::new(),
            selected_tag: 0,
            tags_list_state: ListState::default(),
            tags_loading: false,
            actions_loading: false,
            focused_panel: FocusedPanel::Repos,
            dialog: None,
            previous_dialog: None,
        }
    }

    fn filter_repos(&mut self) {
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

    fn add_to_search(&mut self, c: char) {
        self.search_query.push(c);
        self.filter_repos();
        self.needs_action_reload = true;
    }

    fn remove_from_search(&mut self) {
        self.search_query.pop();
        self.filter_repos();
        self.needs_action_reload = true;
    }

    fn clear_search(&mut self) {
        self.search_query.clear();
        self.filter_repos();
        self.needs_action_reload = true;
    }

    fn next_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            self.selected_repo = (self.selected_repo + 1) % self.filtered_repos.len();
            self.repo_list_state.select(Some(self.selected_repo));
            self.needs_action_reload = true;
        }
    }

    fn previous_repo(&mut self) {
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

    fn next_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = (self.selected_action + 1) % self.actions.len();
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    fn previous_action(&mut self) {
        if !self.actions.is_empty() {
            self.selected_action = if self.selected_action == 0 {
                self.actions.len() - 1
            } else {
                self.selected_action - 1
            };
            self.actions_list_state.select(Some(self.selected_action));
        }
    }

    async fn load_actions_for_selected_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            let repo_name = &self.filtered_repos[self.selected_repo];
            match self.provider.list_actions(repo_name).await {
                Ok(actions) => {
                    self.actions = actions;
                    self.selected_action = 0;
                    if !self.actions.is_empty() {
                        self.actions_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load actions for {}: {}", repo_name, e);
                    self.actions = Vec::new();
                }
            }
        }
        self.needs_action_reload = false;
        self.last_action_load = Some(Instant::now());
    }

    async fn load_tags_for_selected_repo(&mut self) {
        if !self.filtered_repos.is_empty() {
            let repo_name = &self.filtered_repos[self.selected_repo];
            match self.provider.list_tags(repo_name).await {
                Ok(tags) => {
                    self.tags = tags;
                    self.selected_tag = 0;
                    if !self.tags.is_empty() {
                        self.tags_list_state.select(Some(0));
                    }
                }
                Err(e) => {
                    self.set_notification(format!("Failed to load tags for {}: {}", repo_name, e));
                    self.tags = Vec::new();
                }
            }
        }
    }

    fn should_load_actions(&self) -> bool {
        if !self.needs_action_reload {
            return false;
        }
        
        if let Some(last_load) = self.last_action_load {
            if Instant::now().duration_since(last_load) < self.action_load_debounce {
                return false;
            }
        }
        
        true
    }

    fn set_notification(&mut self, msg: impl Into<String>) {
        self.notification = Some(msg.into());
        self.notification_time = Some(Instant::now());
    }

    fn maybe_clear_notification(&mut self) {
        if let Some(time) = self.notification_time {
            if Instant::now().duration_since(time) > Duration::from_secs(3) {
                self.notification = None;
                self.notification_time = None;
            }
        }
    }

    async fn execute_selected_action(&mut self) {
        if !self.filtered_repos.is_empty() && !self.actions.is_empty() {
            let repo_name = self.filtered_repos[self.selected_repo].clone();
            let action = self.actions[self.selected_action].clone();
            
            // First, try to fetch workflow inputs
            match self.provider.fetch_workflow_inputs(&repo_name, &action.workflow_id).await {
                Ok(inputs) => {
                    if inputs.is_empty() {
                        // No inputs required, show confirmation dialog
                        self.open_dialog(vec![]);
                    } else {
                        // Convert API inputs to UI inputs and open dialog
                        let ui_inputs: Vec<WorkflowInputField> = inputs.into_iter().map(|input| {
                            let tag_names: Vec<String> = self.tags.iter().map(|tag| tag.name.clone()).collect();
                            let input_type = determine_input_type(&input.name, &tag_names);
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
                            
                            WorkflowInputField {
                                name: input.name,
                                value: final_value,
                                required: input.required,
                                description: input.description,
                                input_type: final_input_type,
                            }
                        }).collect();
                        self.open_dialog(ui_inputs);
                    }
                }
                Err(e) => {
                    self.set_notification(format!("Failed to fetch workflow inputs for {}: {}", action.name, e));
                }
            }
        }
    }

    fn start_loading_tags(&mut self) {
        self.tags_loading = true;
        self.tags.clear();
        self.tags_list_state.select(None);
    }
    fn finish_loading_tags(&mut self, tags: Vec<houston_api::Tag>) {
        self.tags_loading = false;
        self.tags = tags;
        self.selected_tag = 0;
        if !self.tags.is_empty() {
            self.tags_list_state.select(Some(0));
        }
    }
    fn fail_loading_tags(&mut self, msg: String) {
        self.tags_loading = false;
        self.tags.clear();
        self.tags_list_state.select(None);
        self.set_notification(msg);
    }
    fn start_loading_actions(&mut self) {
        self.actions_loading = true;
        self.actions.clear();
        self.actions_list_state.select(None);
    }
    fn finish_loading_actions(&mut self, actions: Vec<Action>) {
        self.actions_loading = false;
        self.actions = actions;
        self.selected_action = 0;
        if !self.actions.is_empty() {
            self.actions_list_state.select(Some(0));
        }
    }
    fn fail_loading_actions(&mut self, msg: String) {
        self.actions_loading = false;
        self.actions.clear();
        self.actions_list_state.select(None);
        self.set_notification(msg);
    }

    fn focus_next_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Repos => FocusedPanel::Actions,
            FocusedPanel::Actions => FocusedPanel::Tags,
            FocusedPanel::Tags => FocusedPanel::Repos,
        };
    }
    fn focus_prev_panel(&mut self) {
        self.focused_panel = match self.focused_panel {
            FocusedPanel::Repos => FocusedPanel::Tags,
            FocusedPanel::Actions => FocusedPanel::Repos,
            FocusedPanel::Tags => FocusedPanel::Actions,
        };
    }

    fn open_dialog(&mut self, fields: Vec<WorkflowInputField>) {
        let repo_name = self.filtered_repos.get(self.selected_repo).cloned().unwrap_or_default();
        let action_name = self.actions.get(self.selected_action).map(|a| a.name.clone()).unwrap_or_default();
        let is_confirmation = fields.is_empty();
        
        let initial_focus = if is_confirmation {
            DialogFocus::ConfirmButton
        } else if !fields.is_empty() {
            DialogFocus::Field(0)
        } else {
            DialogFocus::ConfirmButton
        };
        
        self.dialog = Some(DialogState {
            dialog_type: DialogType::Input {
                fields,
                focus: initial_focus,
                repo_name,
                action_name,
                is_confirmation,
            },
            open: true,
        });
    }
    
    fn open_confirmation_dialog(&mut self, repo_name: String, action_name: String) {
        self.dialog = Some(DialogState {
            dialog_type: DialogType::Input {
                fields: vec![],
                focus: DialogFocus::ConfirmButton,
                repo_name,
                action_name,
                is_confirmation: true,
            },
            open: true,
        });
    }
    
    fn open_dropdown_selection(&mut self, field_name: String, options: Vec<String>, target_field_index: usize) {
        let filtered_options = options.clone();
        
        // Store the current dialog as previous
        self.previous_dialog = self.dialog.take();
        
        self.dialog = Some(DialogState {
            dialog_type: DialogType::DropdownSelection {
                field_name,
                options,
                filtered_options,
                selected: 0,
                search_query: String::new(),
                target_field_index,
                scroll_offset: 0,
            },
            open: true,
        });
    }
    fn close_dialog(&mut self) {
        self.dialog = None;
        self.previous_dialog = None;
    }
    
    fn cancel_dropdown_selection(&mut self) {
        if let Some(previous) = self.previous_dialog.take() {
            self.dialog = Some(previous);
        } else {
            self.close_dialog();
        }
    }
    fn dialog_next_field(&mut self) {
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
    fn dialog_prev_field(&mut self) {
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
    fn dialog_input_char(&mut self, c: char) {
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
    
    fn dialog_search_char(&mut self, c: char) {
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
    fn dialog_backspace(&mut self) {
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
    
    fn dialog_navigate(&mut self, direction: i32) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { fields, focus, .. } => {
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
    
    fn dialog_open_dropdown(&mut self) {
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
    
    fn dialog_navigate_dropdown(&mut self, direction: i32) {
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
    
    fn dialog_select_dropdown_option(&mut self) {
        if let Some(dialog) = self.dialog.take() {
            if let DialogType::DropdownSelection { filtered_options, selected, target_field_index, .. } = dialog.dialog_type {
                if !filtered_options.is_empty() && selected < filtered_options.len() {
                    let selected_value = filtered_options[selected].clone();
                    
                    // Restore the input dialog and update the field value
                    if let Some(mut restored_dialog) = self.previous_dialog.take() {
                        if let DialogType::Input { ref mut fields, .. } = restored_dialog.dialog_type {
                            if let Some(field) = fields.get_mut(target_field_index) {
                                field.value = selected_value;
                                // Update the selected index in the dropdown
                                if let InputType::Dropdown { options, selected } | InputType::Choice { options, selected } = &mut field.input_type {
                                    *selected = options.iter().position(|opt| opt == &field.value).unwrap_or(0);
                                }
                            }
                        }
                        self.dialog = Some(restored_dialog);
                    }
                }
            }
        }
    }
    
    fn dialog_handle_button_press(&mut self) -> bool {
        if let Some(dialog) = &self.dialog {
            match &dialog.dialog_type {
                DialogType::Input { focus, .. } => {
                    matches!(focus, DialogFocus::ConfirmButton)
                }
                DialogType::DropdownSelection { .. } => false,
            }
        } else {
            false
        }
    }
    
    async fn submit_dialog(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { focus, repo_name, action_name, is_confirmation, .. } => {
                    match focus {
                        DialogFocus::ConfirmButton => {
                            let repo_name = repo_name.clone();
                            let action_name = action_name.clone();
                            
                            if *is_confirmation {
                                // Execute action without inputs
                                match self.provider.execute_action(&repo_name, &action_name).await {
                                    Ok(run) => {
                                        self.set_notification(format!("Triggered action {} on {}: {}", action_name, repo_name, run.id));
                                    }
                                    Err(e) => {
                                        self.set_notification(format!("Failed to execute action {} on {}: {}", action_name, repo_name, e));
                                    }
                                }
                            } else {
                                // Execute action with inputs
                                // TODO: Implement input submission and action execution with inputs
                                self.set_notification(format!("Executing {} on {} with inputs", action_name, repo_name));
                            }
                            self.close_dialog();
                        }
                        DialogFocus::CancelButton => {
                            self.close_dialog();
                        }
                        DialogFocus::Field(_) => {
                            // If Enter is pressed on a field, move to confirm button
                            *focus = DialogFocus::ConfirmButton;
                        }
                    }
                }
                DialogType::DropdownSelection { .. } => {
                    // Enter selects the current option in dropdown
                    self.dialog_select_dropdown_option();
                }
            }
        }
    }
}

fn get_github_token() -> Option<String> {
    std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|token| token.trim().to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Try to get GitHub token from gh CLI, fall back to None
    let token = get_github_token();
    let provider = GitHubProvider::new(token, None, None);
    
    // Try to fetch repos, fall back to mock data if authentication fails
    let repos = match provider.list_repos().await {
        Ok(repos) => repos.into_iter().map(|r| r.name).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("GitHub API error: {}. Using mock data for testing.", e);
            vec![
                "houston".to_string(),
                "test-repo-1".to_string(),
                "test-repo-2".to_string(),
                "awesome-project".to_string(),
                "backend-api".to_string(),
            ]
        }
    };
    
    let mut app = AppState::new(repos, provider);
    app.load_actions_for_selected_repo().await;

    // Terminal setup
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Enable raw mode for input handling
    crossterm::terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), crossterm::terminal::EnterAlternateScreen)?;

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    crossterm::execute!(io::stdout(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut AppState) -> io::Result<()> {
    let mut search_mode = false;
    let mut last_repo_name: Option<String> = app.filtered_repos.get(app.selected_repo).cloned();
    let (tx, mut rx) = mpsc::unbounded_channel::<AppMsg>();

    loop {
        app.maybe_clear_notification();

        // Check for background results
        while let Ok(msg) = rx.try_recv() {
            match msg {
                AppMsg::TagsLoaded(tags) => app.finish_loading_tags(tags),
                AppMsg::TagsFailed(e) => app.fail_loading_tags(e),
                AppMsg::ActionsLoaded(actions) => app.finish_loading_actions(actions),
                AppMsg::ActionsFailed(e) => app.fail_loading_actions(e),
            }
        }

        // Always load actions/tags when repo changes
        let current_repo_name = app.filtered_repos.get(app.selected_repo).cloned();
        if current_repo_name != last_repo_name {
            if let Some(repo_name) = &current_repo_name {
                app.start_loading_tags();
                app.start_loading_actions();
                let tx_tags = tx.clone();
                let tx_actions = tx.clone();
                let provider_tags = app.provider.clone();
                let provider_actions = app.provider.clone();
                let repo_name_tags = repo_name.clone();
                let repo_name_actions = repo_name.clone();
                tokio::spawn(async move {
                    match provider_tags.list_tags(&repo_name_tags).await {
                        Ok(tags) => tx_tags.send(AppMsg::TagsLoaded(tags)).ok(),
                        Err(e) => tx_tags.send(AppMsg::TagsFailed(format!("Failed to load tags: {}", e))).ok(),
                    };
                });
                tokio::spawn(async move {
                    match provider_actions.list_actions(&repo_name_actions).await {
                        Ok(actions) => tx_actions.send(AppMsg::ActionsLoaded(actions)).ok(),
                        Err(e) => tx_actions.send(AppMsg::ActionsFailed(format!("Failed to load actions: {}", e))).ok(),
                    };
                });
            }
            last_repo_name = current_repo_name;
        }

        terminal.draw(|f| ui(f, app, search_mode))?;

        if crossterm::event::poll(std::time::Duration::from_millis(100))? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    crossterm::event::KeyCode::Char('q') => return Ok(()),
                    crossterm::event::KeyCode::Char('/') if app.dialog.is_none() => {
                        search_mode = true;
                        app.clear_search();
                    }
                    crossterm::event::KeyCode::Esc if app.dialog.is_some() => {
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
                    crossterm::event::KeyCode::Esc => {
                        search_mode = false;
                        app.clear_search();
                    }
                    crossterm::event::KeyCode::Enter if search_mode => {
                        search_mode = false;
                    }
                    crossterm::event::KeyCode::Char(c) if search_mode => {
                        app.add_to_search(c);
                    }
                    crossterm::event::KeyCode::Backspace if search_mode => {
                        app.remove_from_search();
                    }
                    // Dialog input handling
                    crossterm::event::KeyCode::Tab if app.dialog.is_some() => {
                        app.dialog_next_field();
                    }
                    crossterm::event::KeyCode::BackTab if app.dialog.is_some() => {
                        app.dialog_prev_field();
                    }
                    // Open dropdown with space when focused on dropdown field (MUST come before general char handler)
                    crossterm::event::KeyCode::Char(' ') if app.dialog.is_some() => {
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
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') if app.dialog.is_some() => {
                        app.dialog_navigate(-1);
                    }
                    crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') if app.dialog.is_some() => {
                        app.dialog_navigate(1);
                    }
                    crossterm::event::KeyCode::Char(c) if app.dialog.is_some() => {
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
                    crossterm::event::KeyCode::Backspace if app.dialog.is_some() => {
                        app.dialog_backspace();
                    }
                    crossterm::event::KeyCode::Enter if app.dialog.is_some() => {
                        app.submit_dialog().await;
                    }
                    // Navigation keys routed to focused panel (only when no dialog is open)
                    crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Char('k') if !search_mode && app.dialog.is_none() => {
                        match app.focused_panel {
                            FocusedPanel::Repos => app.previous_repo(),
                            FocusedPanel::Actions => app.previous_action(),
                            FocusedPanel::Tags => app.previous_tag(),
                        }
                    }
                    crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Char('j') if !search_mode && app.dialog.is_none() => {
                        match app.focused_panel {
                            FocusedPanel::Repos => app.next_repo(),
                            FocusedPanel::Actions => app.next_action(),
                            FocusedPanel::Tags => app.next_tag(),
                        }
                    }
                    crossterm::event::KeyCode::Char(' ') if !search_mode && app.focused_panel == FocusedPanel::Actions && app.dialog.is_none() => app.execute_selected_action().await,
                    crossterm::event::KeyCode::Tab if app.dialog.is_none() => app.focus_next_panel(),
                    crossterm::event::KeyCode::BackTab if app.dialog.is_none() => app.focus_prev_panel(),
                    _ => {}
                }
            }
        }
    }
}

// Add next_tag/previous_tag to AppState
impl AppState {
    fn next_tag(&mut self) {
        if !self.tags.is_empty() {
            self.selected_tag = (self.selected_tag + 1) % self.tags.len();
            self.tags_list_state.select(Some(self.selected_tag));
        }
    }
    fn previous_tag(&mut self) {
        if !self.tags.is_empty() {
            self.selected_tag = if self.selected_tag == 0 {
                self.tags.len() - 1
            } else {
                self.selected_tag - 1
            };
            self.tags_list_state.select(Some(self.selected_tag));
        }
    }
}

#[derive(Debug)]
enum AppMsg {
    TagsLoaded(Vec<houston_api::Tag>),
    TagsFailed(String),
    ActionsLoaded(Vec<Action>),
    ActionsFailed(String),
}

fn ui(f: &mut Frame, app: &mut AppState, search_mode: bool) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ].as_ref())
        .split(f.size());

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(2), // Notification bar
        ].as_ref())
        .split(f.size());

    // Repo list title with search info
    let repo_title = if search_mode {
        format!("Repositories (search: {})", app.search_query)
    } else if !app.search_query.is_empty() {
        format!("Repositories (filtered: {})", app.search_query)
    } else {
        "Repositories".to_string()
    };

    // Highlight style for focused panel
    let highlight = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
    let normal = Style::default();

    // Render repo list panel (left)
    let panel = RepoListPanel::new(&app.filtered_repos, app.selected_repo);
    let repo_loading = app.filtered_repos.is_empty() && (app.tags_loading || app.actions_loading);
    render_repo_list_panel_with_title_and_style(
        f,
        main_chunks[0],
        &panel,
        &mut app.repo_list_state,
        &repo_title,
        if app.focused_panel == FocusedPanel::Repos { highlight } else { normal },
        repo_loading,
    );

    // Render actions panel (middle)
    let actions_panel = ActionsPanel::new(&app.actions, app.selected_action);
    render_actions_panel(
        f,
        main_chunks[1],
        &actions_panel,
        &mut app.actions_list_state,
        app.actions_loading,
        if app.focused_panel == FocusedPanel::Actions { highlight } else { normal },
    );

    // Render tags panel (right)
    let tags_panel = TagsPanel::new(&app.tags, app.selected_tag);
    render_tags_panel(
        f,
        main_chunks[2],
        &tags_panel,
        &mut app.tags_list_state,
        app.tags_loading,
        if app.focused_panel == FocusedPanel::Tags { highlight } else { normal },
    );

    // Render notification bar at the bottom
    if let Some(msg) = &app.notification {
        let block = Block::default()
            .title(msg.as_str())
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White).bg(Color::Red));
        f.render_widget(block, outer_chunks[1]);
    }

    // Render dialog modal if open
    if let Some(dialog) = &app.dialog {
        match &dialog.dialog_type {
            DialogType::Input { fields, focus, repo_name, action_name, is_confirmation } => {
                let area = centered_rect(70, 50, f.size());
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
                let area = centered_rect(60, 70, f.size());
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
                let _end_idx = (*scroll_offset + max_visible_items).min(total_items);
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
}

fn render_repo_list_panel_with_title_and_style(
    f: &mut Frame,
    area: Rect,
    panel: &houston_ui::RepoListPanel,
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

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

fn determine_input_type(field_name: &str, available_tags: &[String]) -> InputType {
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
