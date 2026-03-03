//! Shared command implementations for DDK.
//!
//! Both the MCP server and the CLI binary delegate to these functions.
//! Each function returns a typed Rust struct; the caller decides how to
//! present it (JSON for MCP, human-readable table for CLI, etc.).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::lsp_types::{CompileProjectParams, CompilerProgress, CompilerProgressParams};
use crate::projects::*;
use crate::state::*;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Summary of a single project entry within a workspace or group project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSummary {
    pub id: usize,
    pub name: String,
    pub directory: String,
    pub dproj: Option<String>,
    pub active: bool,
}

/// Summary of a user-defined workspace and its projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub id: usize,
    pub name: String,
    pub compiler_id: String,
    pub projects: Vec<ProjectSummary>,
}

/// Summary of the loaded group project and its projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupProjectSummary {
    pub name: String,
    pub path: String,
    pub compiler_id: String,
    pub projects: Vec<ProjectSummary>,
}

/// Hierarchical project listing preserving workspace / group-project structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectListResult {
    pub workspaces: Vec<WorkspaceSummary>,
    pub group_project: Option<GroupProjectSummary>,
    pub active_project_id: Option<usize>,
}

impl fmt::Display for ProjectListResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.workspaces.is_empty() && self.group_project.is_none() {
            return write!(f, "No projects found.");
        }

        for ws in &self.workspaces {
            writeln!(f, "Workspace: {} (compiler: {})", ws.name, ws.compiler_id)?;
            if ws.projects.is_empty() {
                writeln!(f, "  (empty)")?;
            } else {
                for p in &ws.projects {
                    let marker = if p.active { " *" } else { "" };
                    writeln!(f, "  [{}]{} {} ({})", p.id, marker, p.name, p.directory)?;
                }
            }
        }

        if let Some(gp) = &self.group_project {
            writeln!(f, "Group Project: {} (compiler: {})", gp.name, gp.compiler_id)?;
            if gp.projects.is_empty() {
                writeln!(f, "  (empty)")?;
            } else {
                for p in &gp.projects {
                    let marker = if p.active { " *" } else { "" };
                    writeln!(f, "  [{}]{} {} ({})", p.id, marker, p.name, p.directory)?;
                }
            }
        }

        Ok(())
    }
}

/// Environment info for the currently active project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentInfo {
    pub project: Option<EnvironmentProject>,
    pub group_project_compiler: Option<CompilerSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentProject {
    pub id: usize,
    pub name: String,
    pub directory: String,
    pub dproj: Option<String>,
    pub compilers: Vec<EnvironmentCompilerEntry>,
}

/// A compiler associated with a specific context (workspace name or
/// "group_project").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentCompilerEntry {
    pub context: String,
    pub key: String,
    pub product_name: String,
    pub product_version: usize,
    pub compiler_version: usize,
    pub installation_path: String,
}

impl fmt::Display for EnvironmentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.project {
            Some(proj) => {
                writeln!(f, "Active project: {} (ID {})", proj.name, proj.id)?;
                writeln!(f, "  Directory: {}", proj.directory)?;
                if let Some(dproj) = &proj.dproj {
                    writeln!(f, "  Dproj:     {dproj}")?;
                }
                if !proj.compilers.is_empty() {
                    writeln!(f, "  Compilers:")?;
                    for entry in &proj.compilers {
                        writeln!(
                            f,
                            "    [{context}] {name} v{ver} ({key})",
                            context = entry.context,
                            name = entry.product_name,
                            ver = entry.product_version,
                            key = entry.key,
                        )?;
                    }
                }
            }
            None => {
                writeln!(f, "No active project.")?;
            }
        }
        if let Some(gc) = &self.group_project_compiler {
            writeln!(
                f,
                "Group project compiler: {} ({}) at {}",
                gc.product_name, gc.key, gc.installation_path
            )?;
        }
        Ok(())
    }
}

/// Summary of a compiler configuration (returned by `list_compilers`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerSummary {
    pub key: String,
    pub product_name: String,
    pub product_version: usize,
    pub compiler_version: usize,
    pub installation_path: String,
}

impl fmt::Display for CompilerSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{key}  {name} v{ver}  ({path})",
            key = self.key,
            name = self.product_name,
            ver = self.product_version,
            path = self.installation_path,
        )
    }
}

/// Confirmation after selecting a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectProjectResult {
    pub project_id: usize,
    pub project_name: String,
}

impl fmt::Display for SelectProjectResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Selected project: {} (ID {}).",
            self.project_name, self.project_id
        )
    }
}

/// Confirmation after setting the group project compiler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetCompilerResult {
    pub key: String,
    pub product_name: String,
}

impl fmt::Display for SetCompilerResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Group project compiler set to: {} ({}).",
            self.product_name, self.key
        )
    }
}

/// Full compilation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompileOutput {
    pub project_name: String,
    pub success: bool,
    pub cancelled: bool,
    pub code: i32,
    pub lines: Vec<String>,
}

impl fmt::Display for CompileOutput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let summary = if self.cancelled {
            format!("Compilation of \"{}\" was cancelled.", self.project_name)
        } else if self.success {
            format!(
                "Project \"{}\" compiled successfully.",
                self.project_name
            )
        } else {
            format!(
                "Compilation of \"{}\" finished with errors (exit code {}).",
                self.project_name, self.code
            )
        };
        write!(f, "{summary}")?;
        if !self.lines.is_empty() {
            write!(f, "\n\nCompiler output:\n{}", self.lines.join("\n"))?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the first `ProjectLink.id` for a given project, searching workspaces
/// first, then the group project.
pub fn find_project_link_id(data: &ProjectsData, project_id: usize) -> Option<usize> {
    for ws in &data.workspaces {
        if let Some(link) = ws.project_links.iter().find(|l| l.project_id == project_id) {
            return Some(link.id);
        }
    }
    if let Some(gp) = &data.group_project {
        if let Some(link) = gp.project_links.iter().find(|l| l.project_id == project_id) {
            return Some(link.id);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Returns environment information for the currently active project.
pub async fn cmd_get_environment_info() -> Result<EnvironmentInfo> {
    let projects_data = PROJECTS_DATA.read().await;
    let compilers = COMPILER_CONFIGURATIONS.read().await;

    let project = match projects_data.active_project_id {
        Some(id) => projects_data.get_project(id),
        None => None,
    };

    let env_project = project.map(|p| {
        let mut entries = Vec::new();

        for workspace in &projects_data.workspaces {
            for link in &workspace.project_links {
                if link.project_id == p.id {
                    if let Some(compiler) = compilers.get(&workspace.compiler_id) {
                        entries.push(EnvironmentCompilerEntry {
                            context: workspace.name.clone(),
                            key: workspace.compiler_id.clone(),
                            product_name: compiler.product_name.clone(),
                            product_version: compiler.product_version,
                            compiler_version: compiler.compiler_version,
                            installation_path: compiler.installation_path.clone(),
                        });
                    }
                }
            }
        }

        if let Some(group_project) = &projects_data.group_project {
            for link in &group_project.project_links {
                if link.project_id == p.id {
                    if let Some(compiler) =
                        compilers.get(&projects_data.group_project_compiler_id)
                    {
                        entries.push(EnvironmentCompilerEntry {
                            context: "group_project".to_string(),
                            key: projects_data.group_project_compiler_id.clone(),
                            product_name: compiler.product_name.clone(),
                            product_version: compiler.product_version,
                            compiler_version: compiler.compiler_version,
                            installation_path: compiler.installation_path.clone(),
                        });
                    }
                }
            }
        }

        EnvironmentProject {
            id: p.id,
            name: p.name.clone(),
            directory: p.directory.clone(),
            dproj: p.dproj.clone(),
            compilers: entries,
        }
    });

    let group_project_compiler = compilers
        .get(&projects_data.group_project_compiler_id)
        .map(|c| CompilerSummary {
            key: projects_data.group_project_compiler_id.clone(),
            product_name: c.product_name.clone(),
            product_version: c.product_version,
            compiler_version: c.compiler_version,
            installation_path: c.installation_path.clone(),
        });

    Ok(EnvironmentInfo {
        project: env_project,
        group_project_compiler,
    })
}

/// Lists all known projects, preserving workspace / group-project hierarchy.
pub async fn cmd_list_projects() -> Result<ProjectListResult> {
    let projects_data = PROJECTS_DATA.read().await;
    let active_id = projects_data.active_project_id;

    let make_summary = |p: &crate::projects::Project| ProjectSummary {
        id: p.id,
        name: p.name.clone(),
        directory: p.directory.clone(),
        dproj: p.dproj.clone(),
        active: Some(p.id) == active_id,
    };

    let workspaces = projects_data
        .workspaces
        .iter()
        .map(|ws| {
            let projects = ws
                .project_links
                .iter()
                .filter_map(|link| {
                    projects_data
                        .get_project(link.project_id)
                        .map(&make_summary)
                })
                .collect();
            WorkspaceSummary {
                id: ws.id,
                name: ws.name.clone(),
                compiler_id: ws.compiler_id.clone(),
                projects,
            }
        })
        .collect();

    let group_project = projects_data.group_project.as_ref().map(|gp| {
        let projects = gp
            .project_links
            .iter()
            .filter_map(|link| {
                projects_data
                    .get_project(link.project_id)
                    .map(&make_summary)
            })
            .collect();
        GroupProjectSummary {
            name: gp.name.clone(),
            path: gp.path.clone(),
            compiler_id: projects_data.group_project_compiler_id.clone(),
            projects,
        }
    });

    Ok(ProjectListResult {
        workspaces,
        group_project,
        active_project_id: active_id,
    })
}

/// Selects a project by ID.
pub async fn cmd_select_project(project_id: usize) -> Result<SelectProjectResult> {
    {
        let data = PROJECTS_DATA.read().await;
        if data.get_project(project_id).is_none() {
            bail!("No project found with ID {project_id}.");
        }
    }

    let change = Change::SelectProject { project_id };
    change.execute().await?;

    let data = PROJECTS_DATA.read().await;
    let name = data
        .get_project(project_id)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("ID {project_id}"));

    Ok(SelectProjectResult {
        project_id,
        project_name: name,
    })
}

/// Lists all available compiler configurations.
pub async fn cmd_list_compilers() -> Result<Vec<CompilerSummary>> {
    let configs = COMPILER_CONFIGURATIONS.read().await;
    Ok(configs
        .iter()
        .map(|(key, cfg)| CompilerSummary {
            key: key.clone(),
            product_name: cfg.product_name.clone(),
            product_version: cfg.product_version,
            compiler_version: cfg.compiler_version,
            installation_path: cfg.installation_path.clone(),
        })
        .collect())
}

/// Sets the group project compiler by key.
pub async fn cmd_set_group_compiler(compiler_key: String) -> Result<SetCompilerResult> {
    {
        let configs = COMPILER_CONFIGURATIONS.read().await;
        if !configs.contains_key(&compiler_key) {
            let available: Vec<String> = configs.keys().cloned().collect();
            bail!(
                "Unknown compiler key: \"{compiler_key}\". Available keys: {}",
                available.join(", ")
            );
        }
    }

    let change = Change::SetGroupProjectCompiler {
        compiler: compiler_key.clone(),
    };
    change.execute().await?;

    let configs = COMPILER_CONFIGURATIONS.read().await;
    let name = configs
        .get(&compiler_key)
        .map(|c| c.product_name.clone())
        .unwrap_or_default();

    Ok(SetCompilerResult {
        key: compiler_key,
        product_name: name,
    })
}

/// Result of formatting a file in-place.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatFileResult {
    pub file_path: String,
}

impl fmt::Display for FormatFileResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Formatted: {}", self.file_path)
    }
}

/// Compiles a project. If `project_id` is `Some`, that project is
/// selected first; otherwise the currently active project is compiled.
/// Collects compiler broadcast output and returns it as a `CompileOutput`.
pub async fn cmd_compile(rebuild: bool, project_id: Option<usize>) -> Result<CompileOutput> {
    // If an explicit project was requested, select it first.
    if let Some(pid) = project_id {
        cmd_select_project(pid).await?;
    }

    let (project_name, resolved_id, link_id) = {
        let data = PROJECTS_DATA.read().await;
        let active_id = match data.active_project_id {
            Some(id) => id,
            None => bail!("No active project selected."),
        };
        let project = match data.get_project(active_id) {
            Some(p) => p,
            None => bail!("Active project not found."),
        };
        let name = project.name.clone();
        let lid = find_project_link_id(&data, active_id);
        match lid {
            Some(lid) => (name, active_id, lid),
            None => bail!("Project \"{name}\" has no compiled links."),
        }
    };

    let params = CompileProjectParams::Project {
        project_id: resolved_id,
        project_link_id: Some(link_id),
        rebuild,
        event_id: "cmd-compile".to_string(),
    };

    // Collect broadcast messages concurrently with compilation.
    let collected: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected_clone = collected.clone();
    let mut receiver = CompilerProgress::subscribe();

    let collect_handle = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let mut lines = collected_clone.lock().unwrap();
                    match event {
                        CompilerProgressParams::Start { lines: ls }
                        | CompilerProgressParams::SingleProjectStarted { lines: ls, .. }
                        | CompilerProgressParams::Completed { lines: ls, .. }
                        | CompilerProgressParams::SingleProjectCompleted { lines: ls, .. } => {
                            lines.extend(ls);
                        }
                        CompilerProgressParams::Stdout { line }
                        | CompilerProgressParams::Stderr { line } => {
                            lines.push(line);
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    });

    let compiler = Compiler::new_standalone(&params).await;
    let compile_result = compiler.compile().await;

    // Brief settling window for in-flight broadcasts, then stop collector.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    collect_handle.abort();
    let _ = collect_handle.await;

    let output_lines = match std::sync::Arc::try_unwrap(collected) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc) => arc.lock().unwrap().clone(),
    };

    match compile_result {
        Ok(result) => Ok(CompileOutput {
            project_name,
            success: result.success,
            cancelled: result.cancelled,
            code: result.code,
            lines: output_lines,
        }),
        Err(e) => {
            // Still return collected output on failure.
            bail!(
                "Compilation failed: {e}{}",
                if output_lines.is_empty() {
                    String::new()
                } else {
                    format!("\n\nCompiler output:\n{}", output_lines.join("\n"))
                }
            );
        }
    }
}

/// Formats a Delphi source file in-place.
///
/// Reads the file at `file_path`, decodes it with `encoding` (e.g. `"utf-8"`,
/// `"windows-1252"`, `"oem"`), runs it through the DDK formatter, then
/// encodes the result back to the same encoding before writing.
/// Defaults to `"utf-8"` when `encoding` is `None`.
pub async fn cmd_format_file(file_path: String, encoding: Option<String>) -> Result<FormatFileResult> {
    use crate::format::Formatter;
    use crate::encoding::{decode_bytes, encode_string};

    let encoding_label = encoding.as_deref().unwrap_or("utf-8");

    let raw = std::fs::read(&file_path)
        .with_context(|| format!("Failed to read file: {file_path}"))?;
    let content = decode_bytes(&raw, encoding_label);

    let formatted = Formatter::new(content)?.execute().await?;

    let out_bytes = encode_string(&formatted, encoding_label);
    std::fs::write(&file_path, &out_bytes)
        .with_context(|| format!("Failed to write file: {file_path}"))?;
    Ok(FormatFileResult { file_path })
}
