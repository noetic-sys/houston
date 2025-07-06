use crate::{VcsProvider, Repo, Tag, Action, ActionRun, ProviderError};
use async_trait::async_trait;
use octocrab::Octocrab;
use octocrab::models::Repository;
use serde::{Deserialize, Serialize};
use serde_json;
use base64::Engine;

#[derive(Debug, Clone)]
pub struct WorkflowInputField {
    pub name: String,
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<String>,
}

#[derive(Clone)]
pub struct GitHubProvider {
    client: Octocrab,
    org: Option<String>,
    user: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct RepoListParams {
    per_page: u8,
    page: u32,
}

impl GitHubProvider {
    pub fn new(token: Option<String>, org: Option<String>, user: Option<String>) -> Self {
        let client = if let Some(token) = token {
            Octocrab::builder().personal_token(token).build().unwrap()
        } else {
            Octocrab::builder().build().unwrap()
        };
        Self { client, org, user }
    }

    pub async fn fetch_workflow_inputs(&self, repo: &str, workflow_id: &str) -> Result<Vec<WorkflowInputField>, ProviderError> {
        let (owner, repo_name): (String, String) = if let Some(org) = &self.org {
            (org.clone(), repo.to_string())
        } else if let Some(user) = &self.user {
            (user.clone(), repo.to_string())
        } else {
            let user = self.client.current().user().await.map_err(|e| ProviderError::Api(e.to_string()))?;
            (user.login, repo.to_string())
        };
        // Get workflow metadata to find the path
        let workflow: serde_json::Value = self.client
            .get(format!("/repos/{}/{}/actions/workflows/{}", owner, repo_name, workflow_id), None::<&()>)
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;
        let path = workflow.get("path").and_then(|v| v.as_str()).ok_or_else(|| ProviderError::Api("No workflow path found".to_string()))?;
        // Get the file content (YAML)
        let file: serde_json::Value = self.client
            .get(format!("/repos/{}/{}/contents/{}", owner, repo_name, path), None::<&()>)
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;
        let content_b64 = file.get("content").and_then(|v| v.as_str()).ok_or_else(|| ProviderError::Api("No workflow content found".to_string()))?;
        let content = base64::engine::general_purpose::STANDARD
            .decode(content_b64.replace('\n', ""))
            .map_err(|e| ProviderError::Api(format!("Base64 decode error: {}", e)))?;
        let yaml_str = String::from_utf8(content).map_err(|e| ProviderError::Api(format!("UTF-8 decode error: {}", e)))?;
        // Parse YAML for inputs
        let yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_str).map_err(|e| ProviderError::Api(format!("YAML parse error: {}", e)))?;
        let mut fields = Vec::new();
        if let Some(inputs) = yaml.get("on").and_then(|on| on.get("workflow_dispatch")).and_then(|wd| wd.get("inputs")) {
            if let Some(map) = inputs.as_mapping() {
                for (k, v) in map {
                    let name = k.as_str().unwrap_or("").to_string();
                    let required = v.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
                    let description = v.get("description").and_then(|d| d.as_str()).map(|s| s.to_string());
                    let default = v.get("default").and_then(|d| d.as_str()).map(|s| s.to_string());
                    fields.push(WorkflowInputField { name, required, description, default });
                }
            }
        }
        Ok(fields)
    }
}

#[async_trait]
impl VcsProvider for GitHubProvider {
    async fn list_repos(&self) -> Result<Vec<Repo>, ProviderError> {
        let mut repos = Vec::new();
        let mut page = 1u32;
        loop {
            let page_repos: Vec<Repository> = if let Some(org) = &self.org {
                let resp = self.client.orgs(org).list_repos().per_page(100).page(page).send().await
                    .map_err(|e| ProviderError::Api(e.to_string()))?;
                resp.items
            } else if let Some(user) = &self.user {
                let params = RepoListParams { per_page: 100, page };
                self.client.get(format!("/users/{}/repos", user), Some(&params)).await
                    .map_err(|e| ProviderError::Api(e.to_string()))?
            } else {
                let params = RepoListParams { per_page: 100, page };
                self.client.get("/user/repos", Some(&params)).await
                    .map_err(|e| ProviderError::Api(e.to_string()))?
            };
            if page_repos.is_empty() {
                break;
            }
            for r in page_repos {
                repos.push(Repo {
                    name: r.name,
                    description: r.description,
                });
            }
            page += 1;
        }
        Ok(repos)
    }

    async fn list_tags(&self, repo: &str) -> Result<Vec<Tag>, ProviderError> {
        if let Some(org) = &self.org {
            let tags = self.client.repos(org, repo).list_tags().per_page(100).send().await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            return Ok(tags.items.into_iter().map(|t| Tag { name: t.name }).collect());
        } else if let Some(user) = &self.user {
            let tags = self.client.repos(user, repo).list_tags().per_page(100).send().await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            return Ok(tags.items.into_iter().map(|t| Tag { name: t.name }).collect());
        } else {
            let user = self.client.current().user().await.map_err(|e| ProviderError::Api(e.to_string()))?;
            let login = user.login.clone();
            let tags = self.client.repos(&login, repo).list_tags().per_page(100).send().await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            return Ok(tags.items.into_iter().map(|t| Tag { name: t.name }).collect());
        }
    }

    async fn list_actions(&self, repo: &str) -> Result<Vec<Action>, ProviderError> {
        let response: Result<serde_json::Value, _> = if let Some(org) = &self.org {
            // Call the GitHub Actions API to get workflows for org repo
            self.client
                .get(format!("/repos/{}/{}/actions/workflows", org, repo), None::<&()>)
                .await
        } else if let Some(user) = &self.user {
            // Call the GitHub Actions API to get workflows for user repo
            self.client
                .get(format!("/repos/{}/{}/actions/workflows", user, repo), None::<&()>)
                .await
        } else {
            // Get authenticated user and call the GitHub Actions API
            let user = self.client.current().user().await.map_err(|e| ProviderError::Api(e.to_string()))?;
            let login = user.login.clone();
            self.client
                .get(format!("/repos/{}/{}/actions/workflows", login, repo), None::<&()>)
                .await
        };

        match response {
            Ok(workflows_json) => {
                // Parse the workflows from the response
                if let Some(workflows) = workflows_json.get("workflows") {
                    if let Some(workflows_array) = workflows.as_array() {
                        let actions: Vec<Action> = workflows_array
                            .iter()
                            .filter_map(|w| {
                                let name = w.get("name")?.as_str()?;
                                let path = w.get("path")?.as_str();
                                let id = w.get("id")?.as_u64()?.to_string();
                                Some(Action {
                                    name: name.to_string(),
                                    description: path.map(|p| format!("Workflow: {}", p)),
                                    workflow_id: id,
                                })
                            })
                            .collect();
                        return Ok(actions);
                    }
                }
                Ok(vec![])
            }
            Err(e) => {
                // If it's a 404, the repo has no workflows - return empty list
                if e.to_string().contains("Not Found") {
                    Ok(vec![])
                } else {
                    Err(ProviderError::Api(e.to_string()))
                }
            }
        }
    }

    async fn execute_action(&self, repo: &str, action: &str) -> Result<ActionRun, ProviderError> {
        // Placeholder: In a real implementation, this would trigger a workflow run
        Ok(ActionRun {
            id: format!("run_{}", chrono::Utc::now().timestamp()),
            status: "queued".to_string(),
            started_at: Some(chrono::Utc::now().to_rfc3339()),
            finished_at: None,
        })
    }

    async fn list_action_runs(&self, repo: &str) -> Result<Vec<ActionRun>, ProviderError> {
        // Placeholder: Return mock action runs
        let runs = vec![
            ActionRun {
                id: "123456789".to_string(),
                status: "completed".to_string(),
                started_at: Some("2024-01-15T10:30:00Z".to_string()),
                finished_at: Some("2024-01-15T10:35:00Z".to_string()),
            },
            ActionRun {
                id: "123456788".to_string(),
                status: "failed".to_string(),
                started_at: Some("2024-01-14T15:20:00Z".to_string()),
                finished_at: Some("2024-01-14T15:25:00Z".to_string()),
            },
        ];
        Ok(runs)
    }
} 