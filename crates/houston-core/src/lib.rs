use houston_api::{GitHubProvider, VcsProvider, Action, Tag};
use houston_ui::{FocusedPanel, InputType, DialogFocus, DialogType, DialogState, UIWorkflowInputField, convert_workflow_input_field};
use ratatui::widgets::ListState;
use std::time::{Duration, Instant};

pub mod navigation;
pub mod state;

#[derive(Debug)]
pub enum AppMsg {
    TagsLoaded(Vec<Tag>),
    TagsFailed(String),
    ActionsLoaded(Vec<Action>),
    ActionsFailed(String),
}

pub struct AppState {
    pub repos: Vec<String>,
    pub filtered_repos: Vec<String>,
    pub search_query: String,
    pub selected_repo: usize,
    pub repo_list_state: ListState,
    pub actions: Vec<Action>,
    pub selected_action: usize,
    pub actions_list_state: ListState,
    pub provider: GitHubProvider,
    pub needs_action_reload: bool,
    pub last_action_load: Option<Instant>,
    pub action_load_debounce: Duration,
    pub notification: Option<String>,
    pub notification_time: Option<Instant>,
    pub tags: Vec<Tag>,
    pub selected_tag: usize,
    pub tags_list_state: ListState,
    pub tags_loading: bool,
    pub actions_loading: bool,
    pub focused_panel: FocusedPanel,
    pub dialog: Option<DialogState>,
    pub previous_dialog: Option<DialogState>,
}

impl AppState {
    pub fn new(repos: Vec<String>, provider: GitHubProvider) -> Self {
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

    pub fn set_notification(&mut self, msg: impl Into<String>) {
        self.notification = Some(msg.into());
        self.notification_time = Some(Instant::now());
    }

    pub fn maybe_clear_notification(&mut self) {
        if let Some(time) = self.notification_time {
            if Instant::now().duration_since(time) > Duration::from_secs(3) {
                self.notification = None;
                self.notification_time = None;
            }
        }
    }

    pub fn should_load_actions(&self) -> bool {
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

    // Loading state management
    pub fn start_loading_tags(&mut self) {
        self.tags_loading = true;
        self.tags.clear();
        self.tags_list_state.select(None);
    }

    pub fn finish_loading_tags(&mut self, tags: Vec<Tag>) {
        self.tags_loading = false;
        self.tags = tags;
        self.selected_tag = 0;
        if !self.tags.is_empty() {
            self.tags_list_state.select(Some(0));
        }
    }

    pub fn fail_loading_tags(&mut self, msg: String) {
        self.tags_loading = false;
        self.tags.clear();
        self.tags_list_state.select(None);
        self.set_notification(msg);
    }

    pub fn start_loading_actions(&mut self) {
        self.actions_loading = true;
        self.actions.clear();
        self.actions_list_state.select(None);
    }

    pub fn finish_loading_actions(&mut self, actions: Vec<Action>) {
        self.actions_loading = false;
        self.actions = actions;
        self.selected_action = 0;
        if !self.actions.is_empty() {
            self.actions_list_state.select(Some(0));
        }
    }

    pub fn fail_loading_actions(&mut self, msg: String) {
        self.actions_loading = false;
        self.actions.clear();
        self.actions_list_state.select(None);
        self.set_notification(msg);
    }

    pub async fn execute_selected_action(&mut self) {
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
                        let tag_names: Vec<String> = self.tags.iter().map(|tag| tag.name.clone()).collect();
                        let ui_inputs: Vec<UIWorkflowInputField> = inputs.into_iter().map(|input| {
                            convert_workflow_input_field(input, &tag_names)
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

    pub async fn submit_dialog(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input { focus, repo_name, action_name, is_confirmation, fields } => {
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
                                let inputs: std::collections::HashMap<String, String> = fields.iter()
                                    .map(|field| (field.name.clone(), field.value.clone()))
                                    .collect();
                                
                                match self.provider.execute_action_with_inputs(&repo_name, &action_name, &inputs).await {
                                    Ok(run) => {
                                        self.set_notification(format!("Triggered action {} on {} with inputs: {}", action_name, repo_name, run.id));
                                    }
                                    Err(e) => {
                                        self.set_notification(format!("Failed to execute action {} on {} with inputs: {}", action_name, repo_name, e));
                                    }
                                }
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

    pub fn open_dialog(&mut self, fields: Vec<UIWorkflowInputField>) {
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

    pub fn close_dialog(&mut self) {
        self.dialog = None;
        self.previous_dialog = None;
    }

    pub fn open_dropdown_selection(&mut self, field_name: String, options: Vec<String>, target_field_index: usize) {
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

    pub fn cancel_dropdown_selection(&mut self) {
        if let Some(previous) = self.previous_dialog.take() {
            self.dialog = Some(previous);
        } else {
            self.close_dialog();
        }
    }

    pub fn dialog_select_dropdown_option(&mut self) {
        if let Some(dialog) = self.dialog.take() {
            if let DialogType::DropdownSelection { filtered_options, selected, target_field_index, .. } = dialog.dialog_type {
                if !filtered_options.is_empty() && selected < filtered_options.len() {
                    let selected_value = filtered_options[selected].clone();
                    
                    // Restore the input dialog and update the field value
                    if let Some(mut restored_dialog) = self.previous_dialog.take() {
                        if let DialogType::Input { ref mut fields, .. } = restored_dialog.dialog_type {
                            if let Some(field) = fields.get_mut(target_field_index) {
                                field.value = selected_value;
                                // Update the selected index in the dropdown/choice (not applicable to Boolean)
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

    pub fn dialog_handle_button_press(&mut self) -> bool {
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
} 