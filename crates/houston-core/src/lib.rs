use houston_api::{
    Action, ActionRun, Branch, DeploymentInfo, GitHubProvider, JobInfo, Tag, VcsProvider,
};
use houston_ui::{
    DialogFocus, DialogState, DialogType, InputType, LogViewerState, PanelId, UIWorkflowInputField,
    WindowManager, convert_workflow_input_field,
};
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
    BranchesLoaded(Vec<Branch>),
    BranchesFailed(String),
    RunsLoaded(Vec<ActionRun>),
    RunsFailed(String),
    JobsLoaded(Vec<JobInfo>),
    JobsFailed(String),
    DeploymentsLoaded(Vec<DeploymentInfo>),
    DeploymentsFailed(String),
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
    pub branches: Vec<Branch>,
    pub branches_loading: bool,
    pub default_branch: String,
    pub dialog: Option<DialogState>,
    pub previous_dialog: Option<DialogState>,
    // Runs
    pub runs: Vec<ActionRun>,
    pub selected_run: usize,
    pub runs_loading: bool,
    // Log viewer
    pub log_viewer: Option<LogViewerState>,
    // Auto-refresh
    pub last_runs_refresh: Option<Instant>,
    pub refresh_interval: Duration,
    // Deployments
    pub deployments: Vec<DeploymentInfo>,
    pub selected_deployment: usize,
    pub deployments_loading: bool,
    // Window manager for multi-panel UI
    pub window_manager: WindowManager,
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
            branches: Vec::new(),
            branches_loading: false,
            default_branch: "main".to_string(),
            dialog: None,
            previous_dialog: None,
            // Runs
            runs: Vec::new(),
            selected_run: 0,
            runs_loading: false,
            // Log viewer
            log_viewer: None,
            // Auto-refresh (5 seconds)
            last_runs_refresh: None,
            refresh_interval: Duration::from_secs(5),
            // Deployments
            deployments: Vec::new(),
            selected_deployment: 0,
            deployments_loading: false,
            // Window manager for multi-panel UI
            window_manager: WindowManager::new(),
        }
    }

    // Centralized message handler
    pub fn handle_message(&mut self, msg: AppMsg) {
        match msg {
            AppMsg::TagsLoaded(data) => self.finish_loading_tags(data),
            AppMsg::TagsFailed(e) => self.fail_loading_tags(e),
            AppMsg::ActionsLoaded(data) => self.finish_loading_actions(data),
            AppMsg::ActionsFailed(e) => self.fail_loading_actions(e),
            AppMsg::BranchesLoaded(data) => self.finish_loading_branches(data),
            AppMsg::BranchesFailed(e) => self.fail_loading_branches(e),
            AppMsg::RunsLoaded(data) => self.finish_loading_runs(data),
            AppMsg::RunsFailed(e) => self.fail_loading_runs(e),
            AppMsg::JobsLoaded(data) => self.finish_loading_jobs(data),
            AppMsg::JobsFailed(e) => self.fail_loading_jobs(e),
            AppMsg::DeploymentsLoaded(data) => self.finish_loading_deployments(data),
            AppMsg::DeploymentsFailed(e) => self.fail_loading_deployments(e),
        }
    }

    // Start loading all data for a repo
    pub fn start_all_loading(&mut self) {
        self.start_loading_tags();
        self.start_loading_actions();
        self.start_loading_branches();
        self.start_loading_runs();
        self.start_loading_deployments();
    }

    // Runs management
    pub fn start_loading_runs(&mut self) {
        self.runs_loading = true;
    }

    pub fn finish_loading_runs(&mut self, runs: Vec<ActionRun>) {
        self.runs_loading = false;
        self.runs = runs;
        self.selected_run = 0;
    }

    pub fn fail_loading_runs(&mut self, msg: String) {
        self.runs_loading = false;
        self.set_notification(msg);
    }

    pub fn should_refresh_runs(&self) -> bool {
        // Only auto-refresh when Runs panel is visible and not currently loading
        if !self.window_manager.is_visible(PanelId::Runs) || self.runs_loading {
            return false;
        }
        match self.last_runs_refresh {
            None => true,
            Some(last) => last.elapsed() >= self.refresh_interval,
        }
    }

    pub fn mark_runs_refreshed(&mut self) {
        self.last_runs_refresh = Some(Instant::now());
    }

    pub fn next_run(&mut self) {
        if !self.runs.is_empty() {
            self.selected_run = (self.selected_run + 1) % self.runs.len();
        }
    }

    pub fn previous_run(&mut self) {
        if !self.runs.is_empty() {
            self.selected_run = if self.selected_run == 0 {
                self.runs.len() - 1
            } else {
                self.selected_run - 1
            };
        }
    }

    // Log viewer (shows jobs and steps)
    pub fn open_log_viewer(&mut self) {
        if let Some(run) = self.runs.get(self.selected_run) {
            self.log_viewer = Some(LogViewerState::new(
                run.id.clone(),
                run.workflow_name.clone(),
            ));
        }
    }

    pub fn close_log_viewer(&mut self) {
        self.log_viewer = None;
    }

    pub fn finish_loading_jobs(&mut self, jobs: Vec<JobInfo>) {
        if let Some(viewer) = &mut self.log_viewer {
            viewer.jobs = jobs;
            viewer.loading = false;
        }
    }

    pub fn fail_loading_jobs(&mut self, msg: String) {
        if let Some(viewer) = &mut self.log_viewer {
            viewer.loading = false;
            self.set_notification(msg);
        }
    }

    pub fn log_scroll_down(&mut self) {
        if let Some(viewer) = &mut self.log_viewer {
            let max_scroll = viewer.total_lines().saturating_sub(1);
            viewer.scroll_offset = (viewer.scroll_offset + 1).min(max_scroll);
        }
    }

    pub fn log_scroll_up(&mut self) {
        if let Some(viewer) = &mut self.log_viewer {
            viewer.scroll_offset = viewer.scroll_offset.saturating_sub(1);
        }
    }

    pub fn log_scroll_top(&mut self) {
        if let Some(viewer) = &mut self.log_viewer {
            viewer.scroll_offset = 0;
        }
    }

    pub fn log_scroll_bottom(&mut self) {
        if let Some(viewer) = &mut self.log_viewer {
            viewer.scroll_offset = viewer.total_lines().saturating_sub(20);
        }
    }

    // Deployments
    pub fn start_loading_deployments(&mut self) {
        self.deployments_loading = true;
        self.deployments.clear();
    }

    pub fn finish_loading_deployments(&mut self, deployments: Vec<DeploymentInfo>) {
        self.deployments_loading = false;
        self.deployments = deployments;
        self.selected_deployment = 0;
    }

    pub fn fail_loading_deployments(&mut self, msg: String) {
        self.deployments_loading = false;
        self.set_notification(msg);
    }

    pub fn next_deployment(&mut self) {
        if !self.deployments.is_empty() {
            self.selected_deployment = (self.selected_deployment + 1) % self.deployments.len();
        }
    }

    pub fn previous_deployment(&mut self) {
        if !self.deployments.is_empty() {
            self.selected_deployment = if self.selected_deployment == 0 {
                self.deployments.len() - 1
            } else {
                self.selected_deployment - 1
            };
        }
    }

    pub fn set_notification(&mut self, msg: impl Into<String>) {
        self.notification = Some(msg.into());
        self.notification_time = Some(Instant::now());
    }

    pub fn maybe_clear_notification(&mut self) {
        if let Some(time) = self.notification_time
            && Instant::now().duration_since(time) > Duration::from_secs(3)
        {
            self.notification = None;
            self.notification_time = None;
        }
    }

    pub fn should_load_actions(&self) -> bool {
        if !self.needs_action_reload {
            return false;
        }

        if let Some(last_load) = self.last_action_load
            && Instant::now().duration_since(last_load) < self.action_load_debounce
        {
            return false;
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

    pub fn start_loading_branches(&mut self) {
        self.branches_loading = true;
        self.branches.clear();
    }

    pub fn finish_loading_branches(&mut self, branches: Vec<Branch>) {
        self.branches_loading = false;
        // Find and store the default branch
        if let Some(default) = branches.iter().find(|b| b.is_default) {
            self.default_branch = default.name.clone();
        } else if !branches.is_empty() {
            self.default_branch = branches[0].name.clone();
        }
        self.branches = branches;
    }

    pub fn fail_loading_branches(&mut self, msg: String) {
        self.branches_loading = false;
        self.branches.clear();
        self.set_notification(msg);
    }

    pub async fn execute_selected_action(&mut self) {
        if !self.filtered_repos.is_empty() && !self.actions.is_empty() {
            let repo_name = self.filtered_repos[self.selected_repo].clone();
            let action = self.actions[self.selected_action].clone();

            // Build ref options: branches first, then tags
            let mut ref_options: Vec<String> =
                self.branches.iter().map(|b| b.name.clone()).collect();
            for tag in &self.tags {
                ref_options.push(format!("tags/{}", tag.name));
            }

            // Find default selection index
            let default_idx = ref_options
                .iter()
                .position(|r| r == &self.default_branch)
                .unwrap_or(0);

            // First, try to fetch workflow inputs
            match self
                .provider
                .fetch_workflow_inputs(&repo_name, &action.workflow_id)
                .await
            {
                Ok(inputs) => {
                    // Create ref field as first input
                    let ref_field = UIWorkflowInputField {
                        name: "ref".to_string(),
                        value: ref_options
                            .get(default_idx)
                            .cloned()
                            .unwrap_or_else(|| self.default_branch.clone()),
                        required: true,
                        description: Some("Branch or tag to run workflow on".to_string()),
                        input_type: InputType::Dropdown {
                            options: ref_options,
                            selected: default_idx,
                        },
                    };

                    // Convert API inputs to UI inputs
                    let tag_names: Vec<String> =
                        self.tags.iter().map(|tag| tag.name.clone()).collect();
                    let mut ui_inputs: Vec<UIWorkflowInputField> = vec![ref_field];
                    ui_inputs.extend(
                        inputs
                            .into_iter()
                            .map(|input| convert_workflow_input_field(input, &tag_names)),
                    );

                    self.open_dialog(ui_inputs);
                }
                Err(e) => {
                    self.set_notification(format!(
                        "Failed to fetch workflow inputs for {}: {}",
                        action.name, e
                    ));
                }
            }
        }
    }

    pub async fn submit_dialog(&mut self) {
        if let Some(dialog) = &mut self.dialog {
            match &mut dialog.dialog_type {
                DialogType::Input {
                    focus,
                    repo_name,
                    action_name,
                    is_confirmation,
                    fields,
                } => {
                    match focus {
                        DialogFocus::ConfirmButton => {
                            let repo_name = repo_name.clone();
                            let action_name = action_name.clone();

                            // Extract git ref (first field named "ref") and other inputs
                            let git_ref = fields
                                .iter()
                                .find(|f| f.name == "ref")
                                .map(|f| f.value.clone())
                                .unwrap_or_else(|| self.default_branch.clone());

                            // Filter out the ref field from inputs
                            let inputs: std::collections::HashMap<String, String> = fields
                                .iter()
                                .filter(|field| field.name != "ref")
                                .map(|field| (field.name.clone(), field.value.clone()))
                                .collect();

                            if *is_confirmation || inputs.is_empty() {
                                // Execute action without inputs (but with ref)
                                match self
                                    .provider
                                    .execute_action(&repo_name, &action_name, &git_ref)
                                    .await
                                {
                                    Ok(_) => {
                                        self.set_notification(format!(
                                            "Triggered {} on {} @ {}",
                                            action_name, repo_name, git_ref
                                        ));
                                    }
                                    Err(e) => {
                                        self.set_notification(format!("Failed: {}", e));
                                    }
                                }
                            } else {
                                // Execute action with inputs
                                match self
                                    .provider
                                    .execute_action_with_inputs(
                                        &repo_name,
                                        &action_name,
                                        &git_ref,
                                        &inputs,
                                    )
                                    .await
                                {
                                    Ok(_) => {
                                        self.set_notification(format!(
                                            "Triggered {} on {} @ {}",
                                            action_name, repo_name, git_ref
                                        ));
                                    }
                                    Err(e) => {
                                        self.set_notification(format!("Failed: {}", e));
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
        let repo_name = self
            .filtered_repos
            .get(self.selected_repo)
            .cloned()
            .unwrap_or_default();
        let action_name = self
            .actions
            .get(self.selected_action)
            .map(|a| a.name.clone())
            .unwrap_or_default();
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

    pub fn open_dropdown_selection(
        &mut self,
        field_name: String,
        options: Vec<String>,
        target_field_index: usize,
    ) {
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
        if let Some(dialog) = self.dialog.take()
            && let DialogType::DropdownSelection {
                filtered_options,
                selected,
                target_field_index,
                ..
            } = dialog.dialog_type
            && !filtered_options.is_empty()
            && selected < filtered_options.len()
        {
            let selected_value = filtered_options[selected].clone();

            if let Some(mut restored_dialog) = self.previous_dialog.take() {
                if let DialogType::Input { ref mut fields, .. } = restored_dialog.dialog_type
                    && let Some(field) = fields.get_mut(target_field_index)
                {
                    field.value = selected_value;
                    if let InputType::Dropdown { options, selected }
                    | InputType::Choice { options, selected } = &mut field.input_type
                    {
                        *selected = options
                            .iter()
                            .position(|opt| opt == &field.value)
                            .unwrap_or(0);
                    }
                }
                self.dialog = Some(restored_dialog);
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
