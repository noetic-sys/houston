use async_trait::async_trait;
use std::collections::HashMap;

pub mod github;
pub use github::GitHubProvider;

#[derive(Debug, Clone)]
pub struct Repo {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub description: Option<String>,
    pub workflow_id: String,
}

#[derive(Debug, Clone)]
pub struct ActionRun {
    pub id: String,
    pub workflow_name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub branch: Option<String>,
}

#[derive(thiserror::Error, Debug)]
pub enum ProviderError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("API error: {0}")]
    Api(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Unknown error: {0}")]
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct JobStep {
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub number: u64,
}

#[derive(Debug, Clone)]
pub struct JobInfo {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub steps: Vec<JobStep>,
}

#[derive(Debug, Clone)]
pub struct Environment {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct DeploymentInfo {
    pub id: u64,
    pub environment: String,
    pub sha: String,
    pub ref_name: String,
    pub status: String,
    pub created_at: String,
    pub creator: String,
}

#[async_trait]
pub trait VcsProvider: Send + Sync {
    async fn list_repos(&self) -> Result<Vec<Repo>, ProviderError>;
    async fn list_tags(&self, repo: &str) -> Result<Vec<Tag>, ProviderError>;
    async fn list_branches(&self, repo: &str) -> Result<Vec<Branch>, ProviderError>;
    async fn list_actions(&self, repo: &str) -> Result<Vec<Action>, ProviderError>;
    async fn execute_action(&self, repo: &str, action: &str, git_ref: &str) -> Result<ActionRun, ProviderError>;
    async fn execute_action_with_inputs(&self, repo: &str, action: &str, git_ref: &str, inputs: &HashMap<String, String>) -> Result<ActionRun, ProviderError>;
    async fn list_action_runs(&self, repo: &str) -> Result<Vec<ActionRun>, ProviderError>;
    async fn get_run_jobs(&self, repo: &str, run_id: &str) -> Result<Vec<JobInfo>, ProviderError>;
    async fn list_deployments(&self, repo: &str) -> Result<Vec<DeploymentInfo>, ProviderError>;
}
