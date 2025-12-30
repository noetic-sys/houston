//! Panel implementations for the multi-panel UI.
//!
//! Each panel is a self-contained rendering unit that can display
//! a specific type of data (runs, logs, repos, etc.).

pub mod runs;
pub mod logs;
pub mod repos;
pub mod workflows;
pub mod deployments;
pub mod tags;

pub use runs::render_runs_panel;
pub use logs::render_logs_panel;
pub use repos::render_repos_panel;
pub use workflows::render_workflows_panel;
pub use deployments::render_deployments_panel;
pub use tags::render_tags_panel;
