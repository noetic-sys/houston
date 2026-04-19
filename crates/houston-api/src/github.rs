use crate::{
    Action, ActionRun, Branch, DeploymentInfo, JobInfo, JobStep, ProviderError, PullRequest, Repo,
    Tag, VcsProvider,
};
use async_trait::async_trait;
use base64::Engine;
use octocrab::Octocrab;
use octocrab::models::Repository;
use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct WorkflowInputField {
    pub name: String,
    pub required: bool,
    pub description: Option<String>,
    pub default: Option<String>,
    pub input_type: Option<String>, // "string", "number", "boolean", "choice", "environment"
    pub options: Option<Vec<String>>, // For choice type inputs
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

    /// Parse a repo string that might be "owner/repo" or just "repo"
    /// Returns (owner, repo) tuple
    async fn parse_repo(&self, repo: &str) -> Result<(String, String), ProviderError> {
        if let Some((owner, name)) = repo.split_once('/') {
            Ok((owner.to_string(), name.to_string()))
        } else if let Some(org) = &self.org {
            Ok((org.clone(), repo.to_string()))
        } else if let Some(user) = &self.user {
            Ok((user.clone(), repo.to_string()))
        } else {
            let user = self
                .client
                .current()
                .user()
                .await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            Ok((user.login, repo.to_string()))
        }
    }

    pub async fn fetch_workflow_inputs(
        &self,
        repo: &str,
        workflow_id: &str,
    ) -> Result<Vec<WorkflowInputField>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        // Get workflow metadata to find the path
        let workflow: serde_json::Value = self
            .client
            .get(
                format!(
                    "/repos/{}/{}/actions/workflows/{}",
                    owner, repo_name, workflow_id
                ),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let path = workflow
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Api("No workflow path found".to_string()))?;

        // Get the file content (YAML)
        let file: serde_json::Value = self
            .client
            .get(
                format!("/repos/{}/{}/contents/{}", owner, repo_name, path),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let content_b64 = file
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProviderError::Api("No workflow content found".to_string()))?;
        let content = base64::engine::general_purpose::STANDARD
            .decode(content_b64.replace('\n', ""))
            .map_err(|e| ProviderError::Api(format!("Base64 decode error: {}", e)))?;
        let yaml_str = String::from_utf8(content)
            .map_err(|e| ProviderError::Api(format!("UTF-8 decode error: {}", e)))?;

        // Parse YAML for inputs
        let yaml: serde_yaml::Value = serde_yaml::from_str(&yaml_str)
            .map_err(|e| ProviderError::Api(format!("YAML parse error: {}", e)))?;
        let mut fields = Vec::new();

        if let Some(inputs) = yaml
            .get("on")
            .and_then(|on| on.get("workflow_dispatch"))
            .and_then(|wd| wd.get("inputs"))
            && let Some(map) = inputs.as_mapping()
        {
            for (k, v) in map {
                let name = k.as_str().unwrap_or("").to_string();
                let required = v.get("required").and_then(|r| r.as_bool()).unwrap_or(false);
                let description = v
                    .get("description")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string());
                let default = v
                    .get("default")
                    .and_then(|d| d.as_str())
                    .map(|s| s.to_string());
                let input_type = v
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(|s| s.to_string());
                let options = v.get("options").and_then(|o| {
                    if let serde_yaml::Value::Sequence(seq) = o {
                        Some(
                            seq.iter()
                                .map(|s| s.as_str().unwrap_or("").to_string())
                                .collect(),
                        )
                    } else {
                        None
                    }
                });
                fields.push(WorkflowInputField {
                    name,
                    required,
                    description,
                    default,
                    input_type,
                    options,
                });
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
                let resp = self
                    .client
                    .orgs(org)
                    .list_repos()
                    .per_page(100)
                    .page(page)
                    .send()
                    .await
                    .map_err(|e| ProviderError::Api(e.to_string()))?;
                resp.items
            } else if let Some(user) = &self.user {
                let params = RepoListParams {
                    per_page: 100,
                    page,
                };
                self.client
                    .get(format!("/users/{}/repos", user), Some(&params))
                    .await
                    .map_err(|e| ProviderError::Api(e.to_string()))?
            } else {
                let params = RepoListParams {
                    per_page: 100,
                    page,
                };
                self.client
                    .get("/user/repos", Some(&params))
                    .await
                    .map_err(|e| ProviderError::Api(e.to_string()))?
            };

            if page_repos.is_empty() {
                break;
            }

            for r in page_repos {
                // Use full_name (owner/repo) to properly identify repos across different owners
                let full_name = r.full_name.unwrap_or_else(|| r.name.clone());
                repos.push(Repo {
                    name: full_name,
                    description: r.description,
                });
            }
            page += 1;
        }

        Ok(repos)
    }

    async fn list_tags(&self, repo: &str) -> Result<Vec<Tag>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;
        let tags = self
            .client
            .repos(&owner, &repo_name)
            .list_tags()
            .per_page(100)
            .send()
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;
        Ok(tags
            .items
            .into_iter()
            .map(|t| Tag { name: t.name })
            .collect())
    }

    async fn list_branches(&self, repo: &str) -> Result<Vec<Branch>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        // Get repo info to find default branch
        let repo_info: serde_json::Value = self
            .client
            .get(format!("/repos/{}/{}", owner, repo_name), None::<&()>)
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let default_branch = repo_info
            .get("default_branch")
            .and_then(|v| v.as_str())
            .unwrap_or("main")
            .to_string();

        // Get branches
        let branches: Vec<serde_json::Value> = self
            .client
            .get(
                format!("/repos/{}/{}/branches?per_page=100", owner, repo_name),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        Ok(branches
            .iter()
            .filter_map(|b| {
                let name = b.get("name")?.as_str()?.to_string();
                let is_default = name == default_branch;
                Some(Branch { name, is_default })
            })
            .collect())
    }

    async fn list_actions(&self, repo: &str) -> Result<Vec<Action>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        let response: Result<serde_json::Value, _> = self
            .client
            .get(
                format!("/repos/{}/{}/actions/workflows", owner, repo_name),
                None::<&()>,
            )
            .await;

        match response {
            Ok(workflows_json) => {
                // Parse the workflows from the response
                if let Some(workflows) = workflows_json.get("workflows")
                    && let Some(workflows_array) = workflows.as_array()
                {
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

    async fn execute_action(
        &self,
        repo: &str,
        action: &str,
        git_ref: &str,
    ) -> Result<ActionRun, ProviderError> {
        // Execute workflow without inputs
        self.execute_action_with_inputs(repo, action, git_ref, &HashMap::new())
            .await
    }

    async fn execute_action_with_inputs(
        &self,
        repo: &str,
        action: &str,
        git_ref: &str,
        inputs: &HashMap<String, String>,
    ) -> Result<ActionRun, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        // Prepare the payload for workflow dispatch
        let mut payload = serde_json::json!({
            "ref": git_ref,
        });

        // Add inputs if provided
        if !inputs.is_empty() {
            payload["inputs"] = serde_json::Value::Object(
                inputs
                    .iter()
                    .map(|(k, v)| (k.clone(), serde_json::Value::String(v.clone())))
                    .collect(),
            );
        }

        // Trigger workflow dispatch
        let response: Result<serde_json::Value, _> = self
            .client
            .post(
                format!(
                    "/repos/{}/{}/actions/workflows/{}/dispatches",
                    owner, repo_name, action
                ),
                Some(&payload),
            )
            .await;

        match response {
            Ok(_) => {
                // GitHub API returns 204 No Content on success, so we create a mock ActionRun
                Ok(ActionRun {
                    id: format!("run_{}", chrono::Utc::now().timestamp()),
                    workflow_name: action.to_string(),
                    status: "queued".to_string(),
                    conclusion: None,
                    started_at: Some(chrono::Utc::now().to_rfc3339()),
                    finished_at: None,
                    branch: Some(git_ref.to_string()),
                })
            }
            Err(e) => Err(ProviderError::Api(e.to_string())),
        }
    }

    async fn list_action_runs(&self, repo: &str) -> Result<Vec<ActionRun>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        let runs_response: serde_json::Value = self
            .client
            .get(
                format!("/repos/{}/{}/actions/runs?per_page=20", owner, repo_name),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let runs = runs_response
            .get("workflow_runs")
            .and_then(|r| r.as_array())
            .map(|runs_array| {
                runs_array
                    .iter()
                    .filter_map(|run| {
                        let id = run.get("id")?.as_u64()?.to_string();
                        let workflow_name = run
                            .get("name")
                            .and_then(|n| n.as_str())
                            .unwrap_or("Unknown")
                            .to_string();
                        let status = run.get("status")?.as_str()?.to_string();
                        let conclusion = run
                            .get("conclusion")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());
                        let started_at = run
                            .get("created_at")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string());
                        let finished_at = run
                            .get("updated_at")
                            .and_then(|t| t.as_str())
                            .map(|s| s.to_string());
                        let branch = run
                            .get("head_branch")
                            .and_then(|b| b.as_str())
                            .map(|s| s.to_string());

                        Some(ActionRun {
                            id,
                            workflow_name,
                            status,
                            conclusion,
                            started_at,
                            finished_at,
                            branch,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(runs)
    }

    async fn get_run_jobs(&self, repo: &str, run_id: &str) -> Result<Vec<JobInfo>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        let jobs_response: serde_json::Value = self
            .client
            .get(
                format!(
                    "/repos/{}/{}/actions/runs/{}/jobs",
                    owner, repo_name, run_id
                ),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let jobs = jobs_response
            .get("jobs")
            .and_then(|j| j.as_array())
            .map(|jobs_array| {
                jobs_array
                    .iter()
                    .filter_map(|job| {
                        let id = job.get("id")?.as_u64()?;
                        let name = job.get("name")?.as_str()?.to_string();
                        let status = job.get("status")?.as_str()?.to_string();
                        let conclusion = job
                            .get("conclusion")
                            .and_then(|c| c.as_str())
                            .map(|s| s.to_string());

                        // Parse steps
                        let steps = job
                            .get("steps")
                            .and_then(|s| s.as_array())
                            .map(|steps_array| {
                                steps_array
                                    .iter()
                                    .filter_map(|step| {
                                        let step_name = step.get("name")?.as_str()?.to_string();
                                        let step_status = step.get("status")?.as_str()?.to_string();
                                        let step_conclusion = step
                                            .get("conclusion")
                                            .and_then(|c| c.as_str())
                                            .map(|s| s.to_string());
                                        let step_number = step.get("number")?.as_u64()?;
                                        Some(JobStep {
                                            name: step_name,
                                            status: step_status,
                                            conclusion: step_conclusion,
                                            number: step_number,
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        Some(JobInfo {
                            id,
                            name,
                            status,
                            conclusion,
                            steps,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(jobs)
    }

    async fn list_deployments(&self, repo: &str) -> Result<Vec<DeploymentInfo>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        let deployments_response: serde_json::Value = self
            .client
            .get(
                format!("/repos/{}/{}/deployments?per_page=50", owner, repo_name),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let deployments_array = deployments_response.as_array().cloned().unwrap_or_default();

        let mut deployments = Vec::new();

        for deployment in deployments_array {
            let id = match deployment.get("id").and_then(|i| i.as_u64()) {
                Some(id) => id,
                None => continue,
            };
            let environment = deployment
                .get("environment")
                .and_then(|e| e.as_str())
                .unwrap_or("unknown")
                .to_string();
            let sha = deployment
                .get("sha")
                .and_then(|s| s.as_str())
                .unwrap_or("")
                .to_string();
            let ref_name = deployment
                .get("ref")
                .and_then(|r| r.as_str())
                .unwrap_or("")
                .to_string();
            let created_at = deployment
                .get("created_at")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let creator = deployment
                .get("creator")
                .and_then(|c| c.get("login"))
                .and_then(|l| l.as_str())
                .unwrap_or("unknown")
                .to_string();

            // Get deployment status
            let status_response: Result<serde_json::Value, _> = self
                .client
                .get(
                    format!("/repos/{}/{}/deployments/{}/statuses", owner, repo_name, id),
                    None::<&()>,
                )
                .await;

            let status = match status_response {
                Ok(statuses) => statuses
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|s| s.get("state"))
                    .and_then(|state| state.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                Err(_) => "unknown".to_string(),
            };

            deployments.push(DeploymentInfo {
                id,
                environment,
                sha: sha.chars().take(7).collect(), // Short SHA
                ref_name,
                status,
                created_at,
                creator,
            });
        }

        Ok(deployments)
    }

    async fn list_pull_requests(&self, repo: &str) -> Result<Vec<PullRequest>, ProviderError> {
        let (owner, repo_name) = self.parse_repo(repo).await?;

        let prs: serde_json::Value = self
            .client
            .get(
                format!(
                    "/repos/{}/{}/pulls?state=open&per_page=50&sort=updated&direction=desc",
                    owner, repo_name
                ),
                None::<&()>,
            )
            .await
            .map_err(|e| ProviderError::Api(e.to_string()))?;

        let result = prs
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|pr| {
                        let number = pr.get("number")?.as_u64()?;
                        let title = pr.get("title")?.as_str()?.to_string();
                        let state = pr.get("state")?.as_str()?.to_string();
                        let author = pr
                            .get("user")
                            .and_then(|u| u.get("login"))
                            .and_then(|l| l.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let branch = pr
                            .get("head")
                            .and_then(|h| h.get("ref"))
                            .and_then(|r| r.as_str())
                            .unwrap_or("")
                            .to_string();
                        let updated_at = pr
                            .get("updated_at")
                            .and_then(|t| t.as_str())
                            .unwrap_or("")
                            .to_string();
                        let draft = pr.get("draft").and_then(|d| d.as_bool()).unwrap_or(false);

                        Some(PullRequest {
                            number,
                            title,
                            state,
                            author,
                            branch,
                            updated_at,
                            draft,
                            reviews_approved: 0,
                            checks_status: "none".to_string(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(result)
    }
}
