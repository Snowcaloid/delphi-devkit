use crate::{files::groupproj::parse_groupproj, lexorank::LexoRank, projects::{changes::WorkspaceUpdateData, compilers::{CompilerConfiguration, compiler_exists}}};
use serde::{Serialize, Deserialize};
use super::compilers::load_compilers;
use anyhow::Result;
use std::path::PathBuf;
use std::collections::HashSet;

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ProjectsData {
    pub(super) id_counter: usize,
    pub(super) active_project_id: Option<usize>,
    pub workspaces: Vec<Workspace>,
    pub projects: Vec<Project>,
    pub group_project: Option<GroupProject>,
}

impl Default for ProjectsData {
    fn default() -> Self {
        ProjectsData {
            id_counter: 0,
            active_project_id: None,
            workspaces: Vec::new(),
            projects: Vec::new(),
            group_project: None,
        }
    }
}

impl ProjectsData {
    pub fn new() -> Self {
        if let Ok(path) = Self::projects_data_file_path() {
            if path.exists() {
                let content = std::fs::read_to_string(&path).unwrap_or_default();
                let projects: ProjectsData = ron::from_str(&content).unwrap_or_default();
                return projects
            } else {
                return Self::default();
            }
        } else {
            return Self::default();
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::projects_data_file_path()?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| anyhow::anyhow!("Failed to create config directory: {}", e))?;
        }

        let content = ron::to_string(self)
            .map_err(|e| anyhow::anyhow!("Failed to serialize projects data: {}", e))?;
        std::fs::write(&path, content)
            .map_err(|e| anyhow::anyhow!("Failed to write projects data file: {}", e))?;
        Ok(())
    }

    fn projects_data_file_path() -> Result<std::path::PathBuf> {
        let path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
            .join("ddk")
            .join("compilers.ron");
        return Ok(path)
    }

    pub fn next_id(&mut self) -> usize {
        self.id_counter += 1;
        return self.id_counter;
    }

    pub fn can_find_any_links(&self, project_id: usize) -> bool {
        for workspace in &self.workspaces {
            for project_link in &workspace.project_links {
                if project_link.project_id == project_id {
                    return true;
                }
            }
        }
        if let Some(group_project) = &self.group_project {
            for project_link in &group_project.project_links {
                if project_link.project_id == project_id {
                    return true;
                }
            }
        }
        return false;
    }

    pub fn new_project(&mut self, file_path: &String, workspace_id: usize) -> Result<()> {
        let (project_id, link_id) = (self.id_counter + 1, self.id_counter + 2);
        let workspace = match self.workspaces.iter_mut().find(|ws| ws.id == workspace_id) {
            Some(ws) => ws,
            _ => return Err(anyhow::anyhow!("Workspace with id {} not found", workspace_id)),
        };
        let file = PathBuf::from(file_path);
        let project = match file.extension().and_then(|ext| ext.to_str()).map(|s| s.to_lowercase()) {
            Some(ext) if ext == "dproj" => {
                Project {
                    id: project_id,
                    name: file.file_stem().and_then(|s| s.to_str()).unwrap_or("<name error>").to_string(),
                    directory: file.parent().and_then(|p| p.to_str()).unwrap_or("<directory error>").to_string(),
                    dproj: Some(file_path.clone()),
                    dpr: None,
                    dpk: None,
                    exe: None,
                    ini: None,
                }
            },
            Some(ext) if ext == "dpr" => {
                Project {
                    id: project_id,
                    name: file.file_stem().and_then(|s| s.to_str()).unwrap_or("<name error>").to_string(),
                    directory: file.parent().and_then(|p| p.to_str()).unwrap_or("<directory error>").to_string(),
                    dproj: None,
                    dpr: Some(file_path.clone()),
                    dpk: None,
                    exe: None,
                    ini: None,
                }
            },
            Some(ext) if ext == "dpk" => {
                Project {
                    id: project_id,
                    name: file.file_stem().and_then(|s| s.to_str()).unwrap_or("<name error>").to_string(),
                    directory: file.parent().and_then(|p| p.to_str()).unwrap_or("<directory error>").to_string(),
                    dproj: None,
                    dpr: None,
                    dpk: Some(file_path.clone()),
                    exe: None,
                    ini: None,
                }
            },
            _ => {
                return Err(anyhow::anyhow!("Unsupported project file type: {}", file_path));
            }
        };
        workspace.project_links.push(ProjectLink {
            id: link_id,
            project_id: project.id,
            sort_rank: LexoRank::default(),
        });
        self.projects.push(project);
        self.next_id(); // for project_id
        self.next_id(); // for link_id

        return Ok(());
    }

    pub fn remove_project(&mut self, project_id: usize, remove_links: bool) {
        self.projects.retain(|proj| proj.id != project_id);

        if Some(project_id) == self.active_project_id {
            self.active_project_id = None;
        }

        if remove_links {
            for workspace in &mut self.workspaces {
                workspace.project_links.retain(|link| link.project_id != project_id);
            }
            if let Some(group_project) = &mut self.group_project {
                group_project.project_links.retain(|link| link.project_id != project_id);
            }
        }
    }

    pub fn remove_project_link(&mut self, project_link_id: usize) {
        let mut project_id: Option<usize> = None;
        for workspace in &mut self.workspaces {
            if let Some(pos) = workspace.project_links.iter().position(|link| link.id == project_link_id) {
                project_id = Some(workspace.project_links[pos].project_id);
                workspace.project_links.remove(pos);
                break;
            }
        }
        if project_id.is_none() {
            if let Some(group_project) = &mut self.group_project &&
               let Some(pos) = group_project.project_links.iter().position(|link| link.id == project_link_id) {
                project_id = Some(group_project.project_links[pos].project_id);
                group_project.project_links.remove(pos);
            }
        }
        if project_id.is_some() && !self.can_find_any_links(project_id.unwrap()) {
            self.remove_project(project_id.unwrap(), false);
        }
    }

    pub fn move_project_link(&mut self, project_link_id: usize, previous: &LexoRank, next: &LexoRank) -> Option<()> {
        for workspace in &mut self.workspaces {
            if let Some(project_link) = workspace.project_links.iter_mut().find(|link| link.id == project_link_id) {
                project_link.sort_rank = previous.between(next)?;
                return Some(());
            }
        }
        if let Some(group_project) = &mut self.group_project {
            if let Some(project_link) = group_project.project_links.iter_mut().find(|link| link.id == project_link_id) {
                project_link.sort_rank = previous.between(next)?;
                return Some(());
            }
        }
        return Some(());
    }

    pub fn refresh_project_paths(&mut self, project_id: usize) -> Result<()> {
        let project = match self.get_project_mut(project_id) {
            Some(proj) => proj,
            _ => return Err(anyhow::anyhow!("Project with id {} not found", project_id)),
        };
        return project.discover_paths();
    }

    pub fn new_workspace(&mut self, name: &String, compiler: &String) -> Result<()> {
        if !compiler_exists(compiler)? {
           return Err(anyhow::anyhow!("Compiler not found: {}", compiler));
        }
        let workspace_id = self.next_id();
        let lexo_rank = if let Some(last_ws) = self.workspaces.last() {
            &last_ws.sort_rank
        } else {
            &LexoRank::default()
        };
        let workspace = Workspace::new(workspace_id, name.clone(), compiler.clone(), lexo_rank.next());
        self.workspaces.push(workspace);
        return Ok(());
    }

    pub fn remove_workspace(&mut self, workspace_id: usize) {
        let project_ids: Vec<usize> = self.workspaces
            .iter()
            .find(|ws| ws.id == workspace_id)
            .map(|ws| ws.project_links.iter().map(|link| link.project_id).collect())
            .unwrap_or_default();

        self.workspaces.retain(|ws| ws.id != workspace_id);
        for project_id in project_ids {
            if !self.can_find_any_links(project_id) {
                self.remove_project(project_id, false);
            }
        }
    }

    pub fn move_workspace(&mut self, workspace_id: usize, previous: &LexoRank, next: &LexoRank) -> Option<()> {
        if let Some(workspace) = self.workspaces.iter_mut().find(|ws| ws.id == workspace_id) {
            workspace.sort_rank = previous.between(next)?;
            return Some(());
        }
        return None;
    }

    pub fn update_workspace(&mut self, workspace_id: usize, data: &WorkspaceUpdateData) -> Result<()> {
        let workspace = match self.get_workspace_mut(workspace_id) {
            Some(ws) => ws,
            _ => return Err(anyhow::anyhow!("Workspace with id {} not found", workspace_id)),
        };
        if let Some(name) = &data.name {
            workspace.name = name.clone();
        }
        if let Some(compiler_id) = &data.compiler {
            if !compiler_exists(compiler_id)? {
                return Err(anyhow::anyhow!("Compiler not found: {}", compiler_id));
            }
            workspace.compiler_id = compiler_id.clone();
        }
        return Ok(());
    }

    pub fn set_group_project(&mut self, groupproj_path: &String, compiler: &String) -> Result<()> {
        if !compiler_exists(compiler)? {
           return Err(anyhow::anyhow!("Compiler not found: {}", compiler));
        }
        let path = PathBuf::from(groupproj_path);
        if !path.exists() {
            return Err(anyhow::anyhow!("Group project file does not exist: {}", groupproj_path));
        }
        let mut group_project = GroupProject {
            name: path.file_stem().and_then(|s| s.to_str()).unwrap_or("<name error>").to_string(),
            project_links: Vec::new(),
            path: groupproj_path.clone(),
            compiler_id: compiler.clone(),
        };
        group_project.fill(self)?;
        self.group_project = Some(group_project);
        return Ok(());
    }

    pub fn remove_group_project(&mut self) {
        self.group_project = None;

        let linked_project_ids: HashSet<usize> = self.workspaces
            .iter()
            .flat_map(|workspace| workspace.project_links.iter())
            .map(|link| link.project_id)
            .collect();

        self.projects.retain(|project| linked_project_ids.contains(&project.id));

        if let Some(active_project_id) = self.active_project_id && !self.can_find_any_links(active_project_id) {
            self.active_project_id = None;
        }
    }

    pub fn get_project(&self, project_id: usize) -> Option<&Project> {
        return self.projects.iter().find(|proj| proj.id == project_id);
    }

    pub fn get_project_mut(&mut self, project_id: usize) -> Option<&mut Project> {
        return self.projects.iter_mut().find(|proj| proj.id == project_id);
    }

    pub fn get_workspace(&self, workspace_id: usize) -> Option<&Workspace> {
        return self.workspaces.iter().find(|ws| ws.id == workspace_id);
    }

    pub fn get_workspace_mut(&mut self, workspace_id: usize) -> Option<&mut Workspace> {
        return self.workspaces.iter_mut().find(|ws| ws.id == workspace_id);
    }

    pub fn find_project_by_dproj(&self, dproj: &String) -> Option<&Project> {
        return self.projects.iter().find(|proj| proj.dproj.as_ref().map_or(false, |p| p == dproj));
    }

    pub fn sort(&mut self) {
        self.workspaces.sort_by(|a: &Workspace, b: &Workspace| a.sort_rank.cmp(&b.sort_rank));
        for workspace in &mut self.workspaces {
            workspace.project_links.sort_by(|a: &ProjectLink, b: &ProjectLink| a.sort_rank.cmp(&b.sort_rank));
        }
        if let Some(group_project) = &mut self.group_project {
            group_project.project_links.sort_by(|a: &ProjectLink, b: &ProjectLink| a.sort_rank.cmp(&b.sort_rank));
        }
    }

    pub fn active_project(&self) -> Option<&Project> {
        if let Some(active_id) = self.active_project_id {
            return self.projects.iter().find(|proj| proj.id == active_id);
        }
        return None;
    }

    pub fn projects_of_workspace(&self, workspace: &Workspace) -> Vec<&Project> {
        let mut result = Vec::new();
        for project_link in &workspace.project_links {
            if let Some(project) = self.projects.iter().find(|proj| proj.id == project_link.project_id) {
                result.push(project);
            }
        }
        return result;
    }

    pub fn projects_of_group_project(&self, group_project: &GroupProject) -> Vec<&Project> {
        let mut result = Vec::new();
        for project_link in &group_project.project_links {
            if let Some(project) = self.projects.iter().find(|proj| proj.id == project_link.project_id) {
                result.push(project);
            }
        }
        return result;
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub id: usize,
    pub name: String,
    pub compiler_id: String,
    pub project_links: Vec<ProjectLink>,
    pub sort_rank: LexoRank,
}

impl Workspace {
    pub fn new(id: usize, name: String, compiler_id: String, lexo_rank: LexoRank) -> Self {
        Workspace {
            id,
            name,
            compiler_id,
            project_links: Vec::new(),
            sort_rank: lexo_rank,
        }
    }

    pub fn compiler(&self) -> Option<CompilerConfiguration> {
        let mut compilers = load_compilers().ok()?;
        return compilers.remove(&self.compiler_id.to_string());
    }

    pub fn new_project_link(&mut self, id: usize, project_id: usize) {
        let last_rank = if let Some(last_link) = self.project_links.last() {
            last_link.sort_rank.clone()
        } else {
            LexoRank::default()
        };
        self.project_links.push(ProjectLink {
            id,
            project_id,
            sort_rank: last_rank.next(),
        });
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ProjectLink {
    pub id: usize,
    pub project_id: usize,
    pub sort_rank: LexoRank,
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: usize,
    pub name: String,
    pub directory: String,
    pub dproj: Option<String>,
    pub dpr: Option<String>,
    pub dpk: Option<String>,
    pub exe: Option<String>,
    pub ini: Option<String>,
}

impl Project {
    pub fn discover_paths(&mut self) -> Result<()> {
        todo!("Discovery logic must still be implemented");
    }
}

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct GroupProject {
    pub name: String,
    pub path: String,
    pub compiler_id: String,
    pub project_links: Vec<ProjectLink>,
}


impl GroupProject {
    pub fn compiler(&self) -> Option<CompilerConfiguration> {
        let mut compilers = load_compilers().ok()?;
        return compilers.remove(&self.compiler_id.to_string());
    }

    pub fn new_project_link(&mut self, id: usize, project_id: usize) {
        let last_rank = if let Some(last_link) = self.project_links.last() {
            last_link.sort_rank.clone()
        } else {
            LexoRank::default()
        };
        self.project_links.push(ProjectLink {
            id,
            project_id,
            sort_rank: last_rank.next(),
        });
    }

    pub fn fill(&mut self, projects_data: &mut ProjectsData) -> Result<()> {
        let project_paths = parse_groupproj(PathBuf::from(&self.path))?;
        for project_path in project_paths {
            let dproj = project_path.to_string_lossy().to_string();
            let existing_project_id = projects_data.find_project_by_dproj(&dproj).map(|p| p.id);
            if let Some(existing_id) = existing_project_id {
                self.new_project_link(projects_data.next_id(), existing_id);
                continue;
            } else {
                let project_id = projects_data.next_id();
                let mut project = Project {
                    id: project_id,
                    name: project_path.file_stem().and_then(|s| s.to_str()).unwrap_or("<name error>").to_string(),
                    directory: project_path.parent().and_then(|p| p.to_str()).unwrap_or("<directory error>").to_string(),
                    dproj: Some(dproj.clone()),
                    dpr: None,
                    dpk: None,
                    exe: None,
                    ini: None,
                };
                project.discover_paths()?;
                projects_data.projects.push(project);
                self.new_project_link(projects_data.next_id(), project_id);
            }
        }
        return Ok(());
    }
}