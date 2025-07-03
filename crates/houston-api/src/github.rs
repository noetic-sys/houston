use crate::{VcsProvider, Repo, Tag, Action, ActionRun, ProviderError};
use async_trait::async_trait;
use octocrab::Octocrab;
use octocrab::models::Repository;
use serde::{Deserialize, Serialize};

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
            let login = user.login;
            let tags = self.client.repos(&login, repo).list_tags().per_page(100).send().await
                .map_err(|e| ProviderError::Api(e.to_string()))?;
            return Ok(tags.items.into_iter().map(|t| Tag { name: t.name }).collect());
        }
    }

    async fn list_actions(&self, _repo: &str) -> Result<Vec<Action>, ProviderError> {
        // Placeholder: GitHub Actions listing not implemented
        Ok(vec![])
    }

    async fn execute_action(&self, _repo: &str, _action: &str) -> Result<ActionRun, ProviderError> {
        // Placeholder: Action execution not implemented
        Err(ProviderError::Unknown("Action execution not implemented".into()))
    }

    async fn list_action_runs(&self, _repo: &str) -> Result<Vec<ActionRun>, ProviderError> {
        // Placeholder: Action runs listing not implemented
        Ok(vec![])
    }
} 