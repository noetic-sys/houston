// This module contains additional state management utilities and helpers
// that don't fit directly in the AppState struct

use houston_api::VcsProvider;
use std::time::Instant;
use crate::AppState;

impl AppState {
    pub async fn load_actions_for_selected_repo(&mut self) {
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

    pub async fn load_tags_for_selected_repo(&mut self) {
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
} 