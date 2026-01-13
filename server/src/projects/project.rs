use serde::{Serialize, Deserialize};
use anyhow::Result;
use std::path::PathBuf;
use crate::lexorank::{LexoRank, HasLexoRank};
use crate::projects::*;
use crate::files::dproj::{find_dproj_file, get_main_source, get_exe_path};

#[derive(Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ProjectLink {
    pub id: usize,
    pub project_id: usize,
    pub sort_rank: LexoRank,
}

impl ProjectLink {
    pub fn get_project<'a>(&self, projects_data: &'a ProjectsData) -> Option<&'a Project> {
        return projects_data.projects.iter().find(|proj| proj.id == self.project_id);
    }
    pub fn get_project_mut<'a>(&self, projects_data: &'a mut ProjectsData) -> Option<&'a mut Project> {
        return projects_data.projects.iter_mut().find(|proj| proj.id == self.project_id);
    }
    pub fn get_workspace<'a>(&self, projects_data: &'a ProjectsData) -> Option<&'a Workspace> {
        for workspace in &projects_data.workspaces {
            if workspace.project_links.iter().any(|link| link.id == self.id) {
                return Some(workspace);
            }
        }
        return None;
    }
    pub fn get_workspace_mut<'a>(&self, projects_data: &'a mut ProjectsData) -> Option<&'a mut Workspace> {
        for workspace in &mut projects_data.workspaces {
            if workspace.project_links.iter().any(|link| link.id == self.id) {
                return Some(workspace);
            }
        }
        return None;
    }
}

impl HasLexoRank for ProjectLink {
    fn get_lexorank(&self) -> &LexoRank {
        &self.sort_rank
    }
    fn set_lexorank(&mut self, lexorank: LexoRank) {
        self.sort_rank = lexorank;
    }
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
        if self.dproj.is_none() {
            if let Some(dpr_path) = &self.dpr {
                let dproj_path = find_dproj_file(&PathBuf::from(dpr_path))?;
                self.dproj = Some(dproj_path.to_string_lossy().to_string());
            } else if let Some(dpk_path) = &self.dpk {
                let dproj_path = find_dproj_file(&PathBuf::from(dpk_path))?;
                self.dproj = Some(dproj_path.to_string_lossy().to_string());
            }
        }
        if self.dproj.is_none() {
            anyhow::bail!("Cannot discover paths - no dproj, dpr or dpk available for project id: {}", self.id);
        }
        let dproj_path = PathBuf::from(self.dproj.as_ref().unwrap());

        let main_source = get_main_source(&dproj_path)?;
        match main_source.extension().and_then(|ext| ext.to_str()).map(|s| s.to_lowercase()) {
            Some(ext) if ext == "dpr" => {
                self.dpr = Some(main_source.to_string_lossy().to_string());
                self.dpk = None;
                if let Ok(exe_path) = get_exe_path(&dproj_path) {
                    let exe_file_name = exe_path.join(self.name.clone()).with_extension("exe");
                    self.exe = Some(exe_file_name.to_string_lossy().to_string());
                    self.ini = Some(exe_file_name.with_extension("ini").to_string_lossy().to_string());
                } else {
                    if self.exe.is_some() {
                        let exe_path = PathBuf::from(self.exe.as_ref().unwrap());
                        if exe_path.exists() {
                            self.ini = Some(exe_path.with_extension("ini").to_string_lossy().to_string());
                        } else if self.ini.is_some() {
                            let ini_path = PathBuf::from(self.ini.as_ref().unwrap());
                            self.exe = Some(ini_path.with_extension("exe").to_string_lossy().to_string());
                        } else {
                            self.exe = None;
                            self.ini = None;
                        }
                    } else if self.ini.is_some() {
                        let ini_path = PathBuf::from(self.ini.as_ref().unwrap());
                        self.exe = Some(ini_path.with_extension("exe").to_string_lossy().to_string());
                    } else {
                        self.exe = None;
                        self.ini = None;
                    }
                }
            },
            Some(ext) if ext == "dpk" => {
                self.dpk = Some(main_source.to_string_lossy().to_string());
                self.dpr = None;
                self.exe = None;
                self.ini = None;
            },
            _ => {
                anyhow::bail!("Cannot discover paths - main source file is not a DPR or DPK for project id: {}", self.id);
            }
        }

        return Ok(());
    }
}