use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::{projects::{compilers::{CompilerConfiguration, PartialCompilerConfiguration, compiler_exists, load_compilers, save_compilers}, project_data::ProjectsData}, lexorank::LexoRank};

#[derive(Serialize, Deserialize)]
pub struct ChangeSet {
    pub changes: Vec<Change>,
}

impl ChangeSet {
    pub fn execute(&self) -> Result<()> {
        for change in &self.changes {
            change.execute()?;
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub struct WorkspaceUpdateData {
    pub name: Option<String>,
    pub compiler: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Change {
    AddProject { file_path: String, workspace_id: usize },
    RemoveProject { project_link_id: usize },
    MoveProject { project_link_id: usize, previous: LexoRank, next: LexoRank },
    RefreshProject { project_id: usize },
    AddWorkspace { name: String, compiler: String },
    RemoveWorkspace { workspace_id: usize },
    MoveWorkspace { workspace_id: usize, previous: LexoRank, next: LexoRank },
    UpdateWorkspace { workspace_id: usize, data: WorkspaceUpdateData },
    AddCompiler { key: String, config: CompilerConfiguration },
    RemoveCompiler { compiler: String },
    UpdateCompiler { key: String, data: PartialCompilerConfiguration },
    SetGroupProject { groupproj_path: String, compiler: String },
    RemoveGroupProject,
    SetGroupProjectCompiler { compiler: String },
}

impl Change {
    pub fn execute(&self) -> Result<()> {
        match self {
            Change::AddProject { file_path, workspace_id } => {
                return self.add_project(file_path, *workspace_id);
            }
            Change::RemoveProject { project_link_id } => {
                return self.remove_project_link(*project_link_id);
            }
            Change::MoveProject { project_link_id, previous, next } => {
                return self.move_project(*project_link_id, previous, next);
            }
            Change::RefreshProject { project_id } => {
                return self.refresh_project(*project_id);
            }
            Change::AddWorkspace { name, compiler } => {
                return self.add_workspace(name, compiler);
            }
            Change::RemoveWorkspace { workspace_id } => {
                return self.remove_workspace(*workspace_id);
            }
            Change::MoveWorkspace { workspace_id, previous, next } => {
                return self.move_workspace(*workspace_id, previous, next);
            }
            Change::UpdateWorkspace { workspace_id, data } => {
                return self.update_workspace(*workspace_id, data);
            }
            Change::AddCompiler { key, config } => {
                return self.add_compiler(key, config);
            }
            Change::RemoveCompiler { compiler } => {
                return self.remove_compiler(compiler);
            }
            Change::UpdateCompiler { key, data } => {
                return self.update_compiler(key, data);
            }
            Change::SetGroupProject { groupproj_path, compiler } => {
                return self.set_group_project(groupproj_path, compiler);
            }
            Change::RemoveGroupProject => {
                return self.remove_group_project();
            }
            Change::SetGroupProjectCompiler { compiler } => {
                return self.set_group_project_compiler(compiler);
            }
        }
    }

    fn add_project(&self, file_path: &String, workspace_id: usize) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.new_project(file_path, workspace_id)?;
        return projects_data.save();
    }

    fn remove_project_link(&self, project_link_id: usize) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.remove_project_link(project_link_id);
        return projects_data.save();
    }

    fn move_project(&self, project_link_id: usize, previous: &LexoRank, next: &LexoRank) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        if projects_data.move_project_link(project_link_id, previous, next).is_none() {
            anyhow::bail!("Unable to move project link - lexorank error at link id: {}", project_link_id);
        }
        return projects_data.save();
    }

    fn refresh_project(&self, project_id: usize) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.refresh_project_paths(project_id)?;
        return projects_data.save();
    }

    fn add_workspace(&self, name: &String, compiler: &String) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.new_workspace(name, compiler)?;
        return projects_data.save();
    }

    fn remove_workspace(&self, workspace_id: usize) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.remove_workspace(workspace_id);
        return projects_data.save();
    }

    fn move_workspace(&self, workspace_id: usize, previous: &LexoRank, next: &LexoRank) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        if projects_data.move_workspace(workspace_id, previous, next).is_none() {
            anyhow::bail!("Unable to move workspace - lexorank error at workspace id: {}", workspace_id);
        }
        return projects_data.save();
    }

    fn update_workspace(&self, workspace_id: usize, data: &WorkspaceUpdateData) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.update_workspace(workspace_id, data)?;
        return projects_data.save();
    }

    fn add_compiler(&self, key: &String, config: &CompilerConfiguration) -> Result<()> {
        let mut compilers = load_compilers().map_err(|e| anyhow::anyhow!("Unable to add compiler - unable to load compilers: {}", e))?;
        if compilers.contains_key(key) {
            anyhow::bail!("Unable to add compiler - compiler already exists: {}", key);
        }
        compilers.insert(key.clone(), config.clone());
        return save_compilers(&compilers).map_err(|e| anyhow::anyhow!("Unable to add compiler - unable to save compilers: {}", e));
    }

    fn remove_compiler(&self, compiler: &String) -> Result<()> {
        let mut compilers = load_compilers().map_err(|e| anyhow::anyhow!("Unable to remove compiler - unable to load compilers: {}", e))?;
        if compilers.remove(compiler).is_none() {
            anyhow::bail!("Unable to remove compiler - compiler not found: {}", compiler);
        }
        return save_compilers(&compilers).map_err(|e| anyhow::anyhow!("Unable to remove compiler - unable to save compilers: {}", e));
    }

    fn update_compiler(&self, key: &String, data: &PartialCompilerConfiguration) -> Result<()> {
        let mut compilers = load_compilers().map_err(|e| anyhow::anyhow!("Unable to update compiler - unable to load compilers: {}", e))?;
        if let Some(compiler) = compilers.get_mut(key) {
            compiler.update(data);
            return save_compilers(&compilers).map_err(|e| anyhow::anyhow!("Unable to update compiler - unable to save compilers: {}", e));
        } else {
            anyhow::bail!("Unable to update compiler - compiler not found: {}", key);
        }
    }

    fn set_group_project(&self, groupproj_path: &String, compiler: &String) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.set_group_project(groupproj_path, compiler)?;
        return projects_data.save();
    }

    fn remove_group_project(&self) -> Result<()> {
        let mut projects_data = ProjectsData::new();
        projects_data.remove_group_project();
        return projects_data.save();
    }

    fn set_group_project_compiler(&self, compiler: &String) -> Result<()> {
        if !compiler_exists(compiler)? {
            anyhow::bail!(
                "Unable to set group project compiler - compiler not found: {}",
                compiler
            );
        }
        let mut projects_data = ProjectsData::new();
        if let Some(group_project) = &mut projects_data.group_project {
            group_project.compiler_id = compiler.clone();
            return projects_data.save();
        } else {
            anyhow::bail!("Unable to set group project compiler - no group project is set");
        }
    }
}
