//! Panel implementations for the multi-panel UI.
//!
//! Each panel is a self-contained rendering unit that can display
//! a specific type of data (runs, logs, repos, etc.).

pub mod deployments;
pub mod logs;
pub mod pull_requests;
pub mod repos;
pub mod runs;
pub mod tags;
pub mod workflows;

pub use deployments::render_deployments_panel;
pub use logs::render_logs_panel;
pub use pull_requests::render_pull_requests_panel;
pub use repos::render_repos_panel;
pub use runs::render_runs_panel;
pub use tags::render_tags_panel;
pub use workflows::render_workflows_panel;
