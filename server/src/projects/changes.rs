use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::projects::*;
use crate::state::*;

#[derive(Serialize, Deserialize)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
}

impl ChangeSet {
    pub async fn execute(self) -> Result<()> {
        for change in self.changes {
            change.execute().await?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkspaceUpdateData {
    pub name: Option<String>,
    pub compiler: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectUpdateData {
    pub name: Option<String>,
    pub directory: Option<String>,
    pub dproj: Option<String>,
    pub dpr: Option<String>,
    pub dpk: Option<String>,
    pub exe: Option<String>,
    pub ini: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Change {
    NewProject { file_path: String, workspace_id: usize },
    AddProject { project_id: usize, workspace_id: usize },
    RemoveProject { project_link_id: usize },
    MoveProject { project_link_id: usize, drop_target: usize },
    RefreshProject { project_id: usize },
    UpdateProject { project_id: usize, data: ProjectUpdateData },
    SelectProject { project_id: usize },
    AddWorkspace { name: String, compiler: String },
    RemoveWorkspace { workspace_id: usize },
    MoveWorkspace { workspace_id: usize, drop_target: usize },
    UpdateWorkspace { workspace_id: usize, data: WorkspaceUpdateData },
    AddCompiler { key: String, config: CompilerConfiguration },
    RemoveCompiler { compiler: String },
    UpdateCompiler { key: String, data: PartialCompilerConfiguration },
    SetGroupProject { groupproj_path: String },
    RemoveGroupProject,
    SetGroupProjectCompiler { compiler: String },
}

impl Change {
    pub async fn execute(self) -> Result<()> {
        match self {
            Change::NewProject { file_path, workspace_id } => {
                return Self::new_project(file_path, workspace_id).await;
            }
            Change::AddProject { project_id, workspace_id } => {
                return Self::add_project_link(project_id, workspace_id).await;
            }
            Change::RemoveProject { project_link_id } => {
                return Self::remove_project_link(project_link_id).await;
            }
            Change::MoveProject { project_link_id, drop_target } => {
                return Self::move_project(project_link_id, drop_target).await;
            }
            Change::RefreshProject { project_id } => {
                return Self::refresh_project(project_id).await;
            }
            Change::UpdateProject { project_id, data } => {
                return Self::update_project(project_id, data).await;
            }
            Change::SelectProject { project_id } => {
                return Self::select_project(project_id).await;
            }
            Change::AddWorkspace { name, compiler } => {
                return Self::add_workspace(name, compiler).await;
            }
            Change::RemoveWorkspace { workspace_id } => {
                return Self::remove_workspace(workspace_id).await;
            }
            Change::MoveWorkspace { workspace_id, drop_target } => {
                return Self::move_workspace(workspace_id, drop_target).await;
            }
            Change::UpdateWorkspace { workspace_id, data } => {
                return Self::update_workspace(workspace_id, data).await;
            }
            Change::AddCompiler { key, config } => {
                return Self::add_compiler(key, config).await;
            }
            Change::RemoveCompiler { compiler } => {
                return Self::remove_compiler(compiler).await;
            }
            Change::UpdateCompiler { key, data } => {
                return Self::update_compiler(key, data).await;
            }
            Change::SetGroupProject { groupproj_path} => {
                return Self::set_group_project(groupproj_path).await;
            }
            Change::RemoveGroupProject => {
                return Self::remove_group_project().await;
            }
            Change::SetGroupProjectCompiler { compiler } => {
                return Self::set_group_project_compiler(compiler).await;
            }
        }
    }

    async fn new_project(file_path: String, workspace_id: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.new_project(&file_path, workspace_id)?;
        return projects_data.save().await;
    }

    async fn add_project_link(project_id: usize, workspace_id: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.add_project_link(project_id, workspace_id)?;
        return projects_data.save().await;
    }

    async fn remove_project_link(project_link_id: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.remove_project_link(project_link_id);
        return projects_data.save().await;
    }

    async fn move_project(project_link_id: usize, drop_target: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.move_project_link(project_link_id, drop_target)?;
        return projects_data.save().await;
    }

    async fn refresh_project(project_id: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.refresh_project_paths(project_id)?;
        return projects_data.save().await;
    }

    async fn select_project(project_id: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.select_project(project_id)?;
        return projects_data.save().await;
    }

    async fn update_project(project_id: usize, data: ProjectUpdateData) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.update_project(project_id, data)?;
        return projects_data.save().await;
    }

    async fn add_workspace(name: String, compiler: String) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.new_workspace(&name, &compiler).await?;
        return projects_data.save().await;
    }

    async fn remove_workspace(workspace_id: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.remove_workspace(workspace_id);
        return projects_data.save().await;
    }

    async fn move_workspace(workspace_id: usize, drop_target: usize) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.move_workspace(workspace_id, drop_target)?;
        return projects_data.save().await;
    }

    async fn update_workspace(workspace_id: usize, data: WorkspaceUpdateData) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.update_workspace(workspace_id, &data).await?;
        return projects_data.save().await;
    }

    async fn add_compiler(key: String, config: CompilerConfiguration) -> Result<()> {
        let mut compilers = COMPILER_CONFIGURATIONS.write().await;
        compilers.insert(key, config);
        return compilers.save().await;
    }

    async fn remove_compiler(compiler: String) -> Result<()> {
        let mut compilers = COMPILER_CONFIGURATIONS.write().await;
        if compilers.remove(&compiler).is_none() {
            anyhow::bail!("Unable to remove compiler - compiler not found: {}", compiler);
        }
        return compilers.save().await;
    }

    async fn update_compiler(key: String, data: PartialCompilerConfiguration) -> Result<()> {
        let mut compilers = COMPILER_CONFIGURATIONS.write().await;
        if let Some(compiler) = compilers.get_mut(&key) {
            compiler.update(&data);
            return compilers.save().await;
        } else {
            anyhow::bail!("Unable to update compiler - compiler not found: {}", key);
        }
    }

    async fn set_group_project(groupproj_path: String) -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.set_group_project(&groupproj_path)?;
        return projects_data.save().await;
    }

    async fn remove_group_project() -> Result<()> {
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.remove_group_project();
        return projects_data.save().await;
    }

    async fn set_group_project_compiler(compiler: String) -> Result<()> {
        if !compiler_exists(&compiler).await {
            anyhow::bail!(
                "Unable to set group project compiler - compiler not found: {}",
                compiler
            );
        }
        let mut projects_data = PROJECTS_DATA.write().await;
        projects_data.group_project_compiler_id = compiler.clone();
        return projects_data.save().await;
    }
}
