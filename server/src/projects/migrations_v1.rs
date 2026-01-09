use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct JSON {
    pub(super) configuration: Configuration,
    pub(super) compilers: Vec<Compiler>,
    pub(super) version: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Compiler {
    pub(super) name: String,
    pub(super) rs_vers_path: String,
    pub(super) ms_build_path: String,
    pub(super) build_arguments: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Configuration {
    pub(super) id: usize,
    pub(super) workspaces: Vec<Workspace>,
    pub(super) group_projects_compiler: Option<String>,
    pub(super) selected_project: Option<Project>,
    pub(super) selected_group_project: Option<GroupProject>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Workspace {
    pub(super) id: usize,
    pub(super) name: String,
    pub(super) compiler: String,
    pub(super) projects: Vec<ProjectLink>,
    pub(super) sort_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct GroupProject {
    pub(super) id: usize,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) projects: Vec<ProjectLink>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ProjectLink {
    pub(super) id: usize,
    pub(super) project: Project,
    pub(super) sort_value: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Project {
    pub(super) id: usize,
    pub(super) name: String,
    pub(super) path: String,
    pub(super) dproj: Option<String>,
    pub(super) dpr: Option<String>,
    pub(super) dpk: Option<String>,
    pub(super) exe: Option<String>,
    pub(super) ini: Option<String>,
}
