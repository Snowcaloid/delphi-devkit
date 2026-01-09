use tower_lsp::lsp_types::notification::Notification;
use serde::{Deserialize, Serialize};

use crate::projects::project_data::ProjectsData;


pub enum NotifyError {}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct NotifyErrorParams {
    /// The request id to cancel.
    pub message: String,
}

impl Notification for NotifyError {
    type Params = NotifyErrorParams;
    const METHOD: &'static str = "$/notifications/error";
}

pub enum ProjectsUpdate {}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProjectsUpdateParams {
    pub projects: ProjectsData,
}

impl Notification for ProjectsUpdate {
    type Params = ProjectsUpdateParams;
    const METHOD: &'static str = "$/notifications/projects/update";
}