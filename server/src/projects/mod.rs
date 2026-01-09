pub mod compilers;
pub mod project_data;
pub mod changes;
mod migrations_v1;

use project_data::*;
use compilers::*;
use anyhow::{Ok, Result};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::projects::changes::ChangeSet;
use crate::lexorank::LexoRank;

pub fn update(json: Value) -> Result<()> {
    let change_set: ChangeSet = serde_json::from_value(json)?;
    change_set.execute()?;
    return Ok(());
}

pub fn migrate_from_v1(migration_object: &Value) -> Result<()> {
    let json: migrations_v1::JSON = serde_json::from_value(migration_object.clone())?;

    let mut compilers_map = load_compilers().or_else(|_| Ok(HashMap::new()))?;

    for compiler_v1 in json.compilers {
        let v1_rs_vers_path = PathBuf::from(compiler_v1.rs_vers_path);
        for part in v1_rs_vers_path.iter() {
            let product_version_str = part.to_string_lossy().to_string();
            if let std::result::Result::Ok(product_version) = product_version_str.parse::<f32>() {
                if compilers_map.contains_key(&product_version_str) {
                    if let Some(compiler_config) = &mut compilers_map.get_mut(&product_version_str) {
                        compiler_config.build_arguments = compiler_v1.build_arguments;
                    }
                    break;
                } else {
                    let compiler_config = CompilerConfiguration {
                        condition: "unknown <migration_v1>".to_string(),
                        product_name: "unknown <migration_v1>".to_string(),
                        product_version,
                        package_version: 0,
                        compiler_version: 0.0,
                        installation_path: v1_rs_vers_path
                            .parent() // bin
                            .and_then(|p| p.parent()) // lib
                            .unwrap_or(PathBuf::new().as_path())
                            .to_string_lossy()
                            .to_string(),
                        build_arguments: compiler_v1.build_arguments,
                    };
                    compilers_map.insert(product_version_str, compiler_config);
                    break;
                }
            }
        }
    }

    let _ =save_compilers(&compilers_map);

    let mut projects_data = ProjectsData {
        id_counter: 0,
        active_project_id: None,
        group_project: None,
        workspaces: Vec::new(),
        projects: Vec::new(),
    };

    let mut old_project_id_to_new_id: HashMap<usize, usize> = HashMap::new();

    if let Some(selected_project) = json.configuration.selected_project {
        let id = projects_data.next_id();
        old_project_id_to_new_id.insert(selected_project.id, id);
        let project = Project {
            id,
            name: selected_project.name,
            directory: selected_project.path,
            dproj: selected_project.dproj,
            dpr: selected_project.dpr,
            dpk: selected_project.dpk,
            exe: selected_project.exe,
            ini: selected_project.ini,
        };
        projects_data.projects.push(project);
        projects_data.active_project_id = Some(id);
    }

    let mut compiler = "23.0".to_string();

    if let Some(group_project_compiler_name) = json.configuration.group_projects_compiler {
        for (key, value) in &compilers_map {
            if value.product_name.contains(&group_project_compiler_name) {
                compiler = key.clone();
                break;
            }
        }
    }

    if let Some(selected_group_project) = json.configuration.selected_group_project {
        let mut group_project = GroupProject {
            name: selected_group_project.name,
            path: selected_group_project.path,
            project_links: Vec::new(),
            compiler_id: compiler,
        };
        for project_link_v1 in selected_group_project.projects {
            if let Some(new_id) = old_project_id_to_new_id.get(&project_link_v1.project.id) {
                group_project.project_links.push(ProjectLink {
                    id: projects_data.next_id(),
                    project_id: *new_id,
                    sort_rank: LexoRank::from_string_or_default(&project_link_v1.sort_value),
                });
                continue;
            }

            let id = projects_data.next_id();
            let project = Project {
                id,
                name: project_link_v1.project.name,
                directory: project_link_v1.project.path,
                dproj: project_link_v1.project.dproj,
                dpr: project_link_v1.project.dpr,
                dpk: project_link_v1.project.dpk,
                exe: project_link_v1.project.exe,
                ini: project_link_v1.project.ini,
            };
            projects_data.projects.push(project);
            group_project.project_links.push(ProjectLink {
                id: projects_data.next_id(),
                project_id: id,
                sort_rank: LexoRank::from_string_or_default(&project_link_v1.sort_value),
            });
        }
    }

    for workspace_v1 in json.configuration.workspaces {
        let mut compiler_id = "23.0".to_string();
        for (key, value) in &compilers_map {
            if value.product_name.contains(&workspace_v1.compiler) {
                compiler_id = key.clone();
                break;
            }
        }
        let mut workspace = Workspace {
            id: projects_data.next_id(),
            name: workspace_v1.name,
            compiler_id,
            project_links: Vec::new(),
            sort_rank: LexoRank::from_string_or_default(&workspace_v1.sort_value),
        };

        for project_link_v1 in workspace_v1.projects {
            if let Some(new_id) = old_project_id_to_new_id.get(&project_link_v1.project.id) {
                workspace.project_links.push(ProjectLink {
                    id: projects_data.next_id(),
                    project_id: *new_id,
                    sort_rank: LexoRank::from_string_or_default(&project_link_v1.sort_value),
                });
                continue;
            }

            let id = projects_data.next_id();
            let project = Project {
                id,
                name: project_link_v1.project.name,
                directory: project_link_v1.project.path,
                dproj: project_link_v1.project.dproj,
                dpr: project_link_v1.project.dpr,
                dpk: project_link_v1.project.dpk,
                exe: project_link_v1.project.exe,
                ini: project_link_v1.project.ini,
            };
            projects_data.projects.push(project);
            workspace.project_links.push(ProjectLink {
                id: projects_data.next_id(),
                project_id: id,
                sort_rank: LexoRank::from_string_or_default(&project_link_v1.sort_value),
            });
        }

        projects_data.workspaces.push(workspace);
    }

    return projects_data.save();
}