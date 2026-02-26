pub mod compiler_state;

use super::*;
use crate::state::PROJECTS_DATA;
use crate::{CompileProjectParams, CompilerProgress};
use anyhow::Result;
use rust_search::SearchBuilder;
use scopeguard::defer;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncRead, BufReader};
use tokio::process::Command;
use tower_lsp::lsp_types::{Diagnostic, Url};
use std::collections::HashMap;
use tokio::fs::File;

pub struct Compiler {
    client: tower_lsp::Client,
    params: CompileProjectParams,
    projects_data: ProjectsData,
}

impl Compiler {
    pub async fn new(client: tower_lsp::Client, params: &CompileProjectParams) -> Self {
        Compiler {
            client,
            params: params.clone(),
            projects_data: PROJECTS_DATA.read().await.clone(),
        }
    }

    async fn get_project_parameters<'a>(
        &'a self,
        project_id: usize,
        project_link_id: Option<usize>,
        rebuild: bool,
    ) -> Result<CompilationParameters<'a>> {
        let configuration;
        let project = self
            .projects_data
            .get_project(project_id)
            .ok_or_else(|| anyhow::anyhow!("Project with id {} not found", project_id))?;
        if let Some(link_id) = project_link_id {
            if self.projects_data.is_project_link_in_group_project(link_id) {
                configuration = self.projects_data.group_projects_compiler().await;
            } else if let Some(workspace_id) = self
                .projects_data
                .get_workspace_id_containing_project_link(link_id)
            {
                let workspace =
                    self.projects_data
                        .get_workspace(workspace_id)
                        .ok_or_else(|| {
                            anyhow::anyhow!("Workspace with id {} not found", workspace_id)
                        })?;
                configuration = workspace.compiler().await;
            } else {
                anyhow::bail!(
                    "No workspace or group project contains project link with id {}",
                    link_id
                );
            }
        } else {
            let workspace_id = self
                .projects_data
                .workspaces
                .iter()
                .find_map(|ws| {
                    if ws
                        .project_links
                        .iter()
                        .any(|pl| pl.project_id == project_id)
                    {
                        Some(ws.id)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "No workspace contains project link with project id {}",
                        project_id
                    )
                })?;
            configuration = self
                .projects_data
                .get_workspace(workspace_id)
                .ok_or_else(|| anyhow::anyhow!("Workspace with id {} not found", workspace_id))?
                .compiler().await;
        }
        let target = project.get_project_file()?;
        let compiler_name = configuration.product_name.clone();
        return Ok(CompilationParameters {
            projects: vec![project],
            configuration,
            rebuild,
            only_one_project: true,
            banner: CompBanner::new(
                format!("Compiling Project {}", project.name),
                target.to_string_lossy().to_string(),
                compiler_name,
                rebuild,
            ),
        });
    }

    async fn get_all_workspace_parameters<'a>(
        &'a self,
        workspace_id: usize,
        rebuild: bool,
    ) -> Result<CompilationParameters<'a>> {
        let workspace = match self.projects_data.get_workspace(workspace_id) {
            Some(ws) => ws,
            _ => anyhow::bail!("Workspace with id {} not found", workspace_id),
        };
        let configuration = workspace.compiler().await;
        let projects = workspace
            .project_links
            .iter()
            .map(|link| {
                self.projects_data
                    .get_project(link.project_id)
                    .ok_or_else(|| anyhow::anyhow!("Project with id {} not found", link.project_id))
            })
            .collect::<Result<Vec<_>>>()?;
        let compiler_name = configuration.product_name.clone();
        return Ok(CompilationParameters {
            projects,
            configuration,
            rebuild,
            only_one_project: false,
            banner: CompBanner::new(
                format!("Compiling Workspace {}", workspace.name),
                format!("Projects of Workspace '{}'", workspace.name),
                compiler_name,
                rebuild,
            ),
        });
    }

    async fn get_all_group_project_parameters<'a>(
        &'a self,
        rebuild: bool,
    ) -> Result<CompilationParameters<'a>> {
        let group_project = match &self.projects_data.group_project {
            Some(gp) => gp,
            _ => anyhow::bail!("No group project defined"),
        };
        let configuration = self.projects_data.group_projects_compiler().await;
        let projects = group_project
            .project_links
            .iter()
            .map(|link| {
                self.projects_data
                    .get_project(link.project_id)
                    .ok_or_else(|| anyhow::anyhow!("Project with id {} not found", link.project_id))
            })
            .collect::<Result<Vec<_>>>()?;
        let compiler_name = configuration.product_name.clone();
        return Ok(CompilationParameters {
            projects,
            configuration,
            rebuild,
            only_one_project: false,
            banner: CompBanner::new(
                format!("Compiling Group Project {}", group_project.name),
                format!("Projects of Group Project '{}'", group_project.name),
                compiler_name,
                rebuild,
            ),
        });
    }

    async fn get_from_link_parameters<'a>(
        &'a self,
        project_link_id: usize,
        rebuild: bool,
    ) -> Result<CompilationParameters<'a>> {
        let (projects, configuration, banner);
        if let Some(workspace_id) = self
            .projects_data
            .get_workspace_id_containing_project_link(project_link_id)
        {
            let workspace = self
                .projects_data
                .get_workspace(workspace_id)
                .ok_or_else(|| anyhow::anyhow!("Workspace with id {} not found", workspace_id))?;
            if let Some(index) = workspace.index_of(project_link_id) {
                projects = workspace.project_links[index..]
                    .iter()
                    .map(|link| {
                        self.projects_data
                            .get_project(link.project_id)
                            .ok_or_else(|| {
                                anyhow::anyhow!("Project with id {} not found", link.project_id)
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                configuration = workspace.compiler().await;
                let project_name = projects
                    .first()
                    .map(|p| p.name.clone())
                    .unwrap_or("<unknown>".to_string());
                banner = CompBanner::new(
                    format!("Compiling Workspace '{}' Project {project_name}", workspace.name),
                    format!(
                        "Projects of Workspace '{}' from project {project_name}",
                        workspace.name
                    ),
                    configuration.product_name.clone(),
                    rebuild,
                );
            } else {
                anyhow::bail!(
                    "Project link with id {} not found in workspace {}",
                    project_link_id,
                    workspace.name
                );
            }
        } else if let Some(group_project) = &self.projects_data.group_project {
            if let Some(index) = group_project.index_of(project_link_id) {
                projects = group_project.project_links[index..]
                    .iter()
                    .map(|link| {
                        self.projects_data
                            .get_project(link.project_id)
                            .ok_or_else(|| {
                                anyhow::anyhow!("Project with id {} not found", link.project_id)
                            })
                    })
                    .collect::<Result<Vec<_>>>()?;
                configuration = self.projects_data.group_projects_compiler().await;
                let project_name = projects
                    .first()
                    .map(|p| p.name.clone())
                    .unwrap_or("<unknown>".to_string());
                banner = CompBanner::new(
                    format!("Compiling Group Project '{}' Project {project_name}", group_project.name),
                    format!(
                        "Projects of Group Project '{}' from project {project_name}",
                        group_project.name
                    ),
                    configuration.product_name.clone(),
                    rebuild,
                );
            } else {
                anyhow::bail!(
                    "Project link with id {} not found in group project {}",
                    project_link_id,
                    group_project.name
                );
            }
        } else {
            anyhow::bail!(
                "No workspace or group project contains project link with id {}",
                project_link_id
            );
        }
        return Ok(CompilationParameters {
            projects,
            configuration,
            rebuild,
            only_one_project: false,
            banner,
        });
    }

    pub async fn compile(&self) -> Result<()> {
        if !compiler_state::activate() {
            anyhow::bail!(
                "Another compilation is already in progress. Please wait until it finishes."
            );
        }
        defer! {
            compiler_state::reset()
        }

        let parameters = match self.params {
            CompileProjectParams::Project {
                project_id,
                project_link_id,
                rebuild,
                event_id: _
            } => self.get_project_parameters(project_id, project_link_id, rebuild).await?,
            CompileProjectParams::AllInWorkspace {
                workspace_id,
                rebuild,
                event_id: _
            } => self.get_all_workspace_parameters(workspace_id, rebuild).await?,
            CompileProjectParams::AllInGroupProject {
                rebuild,
                event_id: _
            } => self.get_all_group_project_parameters(rebuild).await?,
            CompileProjectParams::FromLink {
                project_link_id,
                rebuild,
                event_id: _,
            } => self.get_from_link_parameters(project_link_id, rebuild).await?,
        };
        clear_stale_diagnostics(&self.client).await;
        // Actual compilation process
        let start_lines = if parameters.only_one_project {
            parameters.banner.into_header_vec()
        } else {
            parameters.banner.into_multi_header_vec()
        };
        CompilerProgress::notify_start(
            &self.client,
            start_lines
        ).await;
        let result = self.do_compile(&parameters).await;
        let cancelled = compiler_state::is_cancelled();
        // Treat cancellation as a non-error outcome so no upstream error is logged
        let result = if cancelled { Ok(()) } else { result };
        CompilerProgress::notify_completed(
            &self.client,
            compiler_state::is_success(),
            cancelled,
            compiler_state::get_code(),
            parameters.banner.into_footer_vec(),
        ).await;
        return result;
    }

    async fn do_compile(&self, parameters: &CompilationParameters<'_>) -> Result<()> {
        for project in &parameters.projects {
            if compiler_state::is_cancelled() {
                return Err(anyhow::anyhow!("Compilation cancelled by user."));
            }

            if !parameters.only_one_project {
                CompilerProgress::notify_single_project_started(
                    &self.client,
                    project.id,
                    CompBanner::new(
                        format!("Compiling Project: {}", project.name),
                        project.get_project_file()?.to_string_lossy().to_string(),
                        parameters.configuration.product_name.clone(),
                        parameters.rebuild,
                    ).into_project_header_vec()
                ).await;
            }

            let rsvars_path = PathBuf::from(&parameters.configuration.installation_path)
                .join("bin")
                .join("rsvars.bat");
            if !rsvars_path.exists() {
                anyhow::bail!(
                    "Cannot find rsvars.bat at path: {}",
                    rsvars_path.to_string_lossy()
                );
            }
            let envs = capture_rsvars_env(&rsvars_path.to_string_lossy()).await?;
            let project_file = project.get_project_file()?;
            let args = parameters.configuration.build_arguments.join(" ");
            let target = if parameters.rebuild { "Build" } else { "Make" };
            let msbuild_path = find_msbuild()?;
            let mut child_process = Command::new(msbuild_path)
                .arg(project_file)
                .arg(format!("/t:Clean,{}", target))
                .args(args.split_whitespace())
                .envs(envs)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()?;

            // Capture the PID before taking stdio handles so we can kill the
            // entire process tree on cancellation (taskkill /F /T kills MSBuild
            // AND every compiler child process it spawned, e.g. dcc32.exe).
            let child_pid = child_process.id();

            let stdout = child_process.stdout.take()
                .ok_or_else(|| anyhow::anyhow!("Unable to access child process STDOUT"))?;
            let stderr = child_process.stderr.take()
                .ok_or_else(|| anyhow::anyhow!("Unable to access child process STDERR"))?;

            let out_reader = BufReader::new(stdout);
            let err_reader = BufReader::new(stderr);

            let project_dir = project.get_project_file()?
                .parent()
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| PathBuf::from(&project.directory));

            let stdout_task = tokio::spawn(process_output_lines(
                self.client.clone(),
                out_reader,
                parameters.configuration.product_name.clone(),
                OutputKind::Stdout,
                project_dir.clone(),
            ));

            let stderr_task = tokio::spawn(process_output_lines(
                self.client.clone(),
                err_reader,
                parameters.configuration.product_name.clone(),
                OutputKind::Stderr,
                project_dir,
            ));

            let cancel_signal = async {
                loop {
                    tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    if compiler_state::is_cancelled() {
                        break;
                    }
                }
            };

            let result = tokio::select! {
                status = child_process.wait() => {
                    let status = status?;
                    stdout_task.await?;
                    stderr_task.await?;
                    compiler_state::set_success(status.success());
                    compiler_state::set_code(status.code().unwrap_or(-1));
                    Ok(())
                }
                _ = cancel_signal => {
                    // Kill the whole process tree so that child processes spawned
                    // by MSBuild (dcc32.exe, dcc64.exe, …) are also terminated.
                    // Without this, those processes keep file locks and the next
                    // compilation attempt on the same project fails immediately.
                    if let Some(pid) = child_pid {
                        let _ = std::process::Command::new("taskkill")
                            .args(["/F", "/T", "/PID", &pid.to_string()])
                            .output();
                    }
                    // Fallback: also ask Tokio to kill the root process handle.
                    let _ = child_process.kill().await;
                    stdout_task.abort();
                    stderr_task.abort();
                    compiler_state::set_success(false);
                    compiler_state::set_code(-1);
                    Err(anyhow::anyhow!("Compilation cancelled by user."))
                }
            };

            if !parameters.only_one_project {
                CompilerProgress::notify_single_project_completed(
                    &self.client,
                    project.id,
                    compiler_state::is_success(),
                    compiler_state::is_cancelled(),
                    compiler_state::get_code(),
                    CompBanner::new(
                        format!("Compiling Project: {}", project.name),
                        project.get_project_file()?.to_string_lossy().to_string(),
                        parameters.configuration.product_name.clone(),
                        parameters.rebuild,
                    ).into_project_footer_vec()
                ).await
            }
            if compiler_state::is_success() {
                let exe_missing = project.exe.as_deref()
                    .map_or(true, |p| p.is_empty() || !PathBuf::from(p).exists());
                if exe_missing {
                    let mut projects_data = PROJECTS_DATA.write().await;
                    if let Some(project) = projects_data.get_project_mut(project.id) &&
                       let Ok(_) = project.discover_paths()
                    {
                        let _ = projects_data.save().await;
                    }
                }
            }
            result?;
        }
        return Ok(());
    }
}

enum OutputKind {
    Stdout,
    Stderr,
}

async fn process_output_lines<R: AsyncRead + Unpin + Send>(
    client: tower_lsp::Client,
    mut reader: BufReader<R>,
    compiler_name: String,
    kind: OutputKind,
    project_dir: PathBuf,
) {
    use tokio::io::AsyncBufReadExt;
    let mut diagnostics: Vec<Diagnostic> = Vec::new();
    let mut last_file = String::new();
    let mut buf = Vec::new();

    loop {
        buf.clear();
        match reader.read_until(b'\n', &mut buf).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        // Decode using the configured compiler encoding
        let line = crate::encoding::decode_line(&buf)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if line.is_empty() {
            continue;
        }
        if compiler_state::is_cancelled() {
            break;
        }
        if let Some(mut diagnostic) = CompilerLineDiagnostic::from_line(&line, compiler_name.clone()) {
            // Resolve relative file paths against the project directory
            let file_path = PathBuf::from(&diagnostic.file);
            if file_path.is_relative() {
                let resolved = project_dir.join(&file_path);
                if resolved.exists() {
                    diagnostic.file = resolved.to_string_lossy().to_string();
                }
            }
            if last_file != diagnostic.file && !diagnostics.is_empty() {
                compiler_state::track_diagnosed_file(last_file.clone());
                publish_diagnostics(&client, &last_file, &diagnostics).await;
                diagnostics.clear();
            }
            last_file = diagnostic.file.clone();
            let formatted = format!("{}", &diagnostic);
            match kind {
                OutputKind::Stdout => CompilerProgress::notify_stdout(&client, formatted).await,
                OutputKind::Stderr => CompilerProgress::notify_stderr(&client, formatted).await,
            }
            diagnostics.push(diagnostic.into());
            continue;
        }
        match kind {
            OutputKind::Stdout => CompilerProgress::notify_stdout(&client, line).await,
            OutputKind::Stderr => CompilerProgress::notify_stderr(&client, line).await,
        }
    }

    if !diagnostics.is_empty() {
        compiler_state::track_diagnosed_file(last_file.clone());
        publish_diagnostics(&client, &last_file, &diagnostics).await;
    }
}

fn find_msbuild() -> Result<String> {
    let mut search: Vec<String> = SearchBuilder::default()
        .location(r"C:\Windows\Microsoft.NET\Framework\")
        .search_input("msbuild.exe")
        .depth(2)
        .ignore_case()
        .build()
        .collect();
    search.retain(|path| path.to_lowercase().ends_with("msbuild.exe"));
    search.sort_by(
        |left, right| {
            let left_version = left
                .split('\\')
                .rev()
                .nth(1)
                .unwrap_or("v0");
            let right_version = right
                .split('\\')
                .rev()
                .nth(1)
                .unwrap_or("v0");
            right_version.cmp(left_version)
        });
    if let Some(msbuild_path) = search.first() {
        return Ok(msbuild_path.clone());
    }
    anyhow::bail!(
        "Cannot find msbuild.exe in C:\\Windows\\Microsoft.NET\\Framework\\. Please ensure that MSBuild is installed and try again."
    );
}

pub async fn capture_rsvars_env(rsvars_path: &str) -> Result<HashMap<String, String>> {
    // List of variables to skip
    let skip_vars = [
        "PROCESSOR_ARCHITECTURE",
        "PROCESSOR_IDENTIFIER",
        "PROCESSOR_LEVEL",
        "PROCESSOR_REVISION",
        "NUMBER_OF_PROCESSORS",
    ];

    // Create a temporary batch file with a unique name.
    // We use into_temp_path() to close the file handle before cmd.exe reads it
    // (Windows won't allow concurrent access otherwise). The TempPath auto-deletes on drop.
    let temp_batch = {
        use std::io::Write;
        let mut f = tempfile::Builder::new()
            .suffix(".bat")
            .tempfile()?;
        write!(f, "@echo off\ncall \"{}\"\nset", rsvars_path)?;
        f.into_temp_path()
    };

    // Execute the temporary batch file
    let mut child = Command::new("cmd")
        .arg("/C")
        .arg(temp_batch.as_ref() as &std::path::Path)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;

    let stdout = child.stdout.take().expect("Child stdout missing");
    let mut reader = BufReader::new(stdout).lines();

    let mut env_vars = HashMap::new();
    while let Some(line) = reader.next_line().await? {
        if let Some((key, value)) = line.split_once('=') {
            if !skip_vars.contains(&key) {
                env_vars.insert(key.to_string(), value.to_string().replace(";;", ";"));
            }
        }
    }

    // Wait for the batch process to finish
    let status = child.wait().await?;
    if !status.success() {
        eprintln!("Warning: rsvars.bat environment capture exited with {}", status);
    }

    // temp_batch (TempPath) is dropped here, which deletes the file
    Ok(env_vars)
}

pub async fn parse_rsvars(path: &str) -> Result<HashMap<String, String>> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut env_vars: HashMap<String, String> = HashMap::new();

    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim_start();
        if trimmed.to_ascii_uppercase().starts_with("@SET ") {
            let rest = &trimmed[5..];
            if let Some((key, value)) = rest.split_once('=') {
                let key = key.trim().to_string();
                let mut value = value.trim().to_string();

                // Expand %VAR% references from already-parsed variables or system env
                while let Some(start) = value.find('%') {
                    if let Some(end) = value[start + 1..].find('%') {
                        let end = start + 1 + end;
                        let var_name = &value[start + 1..end];
                        let replacement = env_vars
                            .get(var_name)
                            .cloned()
                            .or_else(|| std::env::var(var_name).ok())
                            .unwrap_or_default();
                        value.replace_range(start..=end, &replacement);
                    } else {
                        break; // unmatched %, leave as is
                    }
                }

                env_vars.insert(key, value);
            }
        }
    }

    Ok(env_vars)
}

async fn clear_stale_diagnostics(client: &tower_lsp::Client) {
    let mut tasks = tokio::task::JoinSet::new();
    for file in compiler_state::take_diagnosed_files() {
        let client = client.clone();
        tasks.spawn(async move {
            publish_diagnostics(&client, &file, &vec![]).await;
        });
    }
    while tasks.join_next().await.is_some() {}
}

async fn publish_diagnostics(
    client: &tower_lsp::Client,
    file: &str,
    diagnostics: &Vec<Diagnostic>,
) {
    let uri = Url::from_file_path(file).unwrap_or_else(|_| Url::parse("untitled:unknown").unwrap());
    client
        .publish_diagnostics(uri, diagnostics.clone(), None)
        .await;
}

fn format_line(text: &str, total_width: usize) -> String {
    let padding = total_width.saturating_sub(text.len() + 2);
    if padding == 0 {
        return text.to_string();
    }
    let left_padding = padding / 2;
    format!(" {}{}", " ".repeat(left_padding), text)
}

#[derive(Debug, Clone)]
struct CompilationParameters<'compiler> {
    projects: Vec<&'compiler Project>,
    configuration: CompilerConfiguration,
    rebuild: bool,
    only_one_project: bool,
    banner: CompBanner,
}

unsafe impl Send for CompilationParameters<'_> {}
unsafe impl Sync for CompilationParameters<'_> {}

const BANNER_TOP: &str               = "╒══════════════════════════════════════════════════════════════════════╕";
const BANNER_BOTTOM: &str            = "╘══════════════════════════════════════════════════════════════════════╛";
const BANNER_ERROR_TOP: &str         = "╔══════════════════════════════════════════════════════════════════════╗";
const BANNER_ERROR_BOTTOM: &str      = "╚══════════════════════════════════════════════════════════════════════╝";
const BANNER_SUCCESS_TOP: &str       = "┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓";
const BANNER_SUCCESS_BOTTOM: &str    = "┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛";
const BANNER_CANCELLED_TOP: &str     = "╓──────────────────────────────────────────────────────────────────────╖";
const BANNER_CANCELLED_BOTTOM: &str  = "╙──────────────────────────────────────────────────────────────────────╜";
const BANNER_MULTI_TOP: &str         = "┍━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┑";
const BANNER_MULTI_BOTTOM: &str      = "┕━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┙";
const BANNER_PROJECT_TOP: &str       = "┌──────────────────────────────────────────────────────────────────────┐";
const BANNER_PROJECT_BOTTOM: &str    = "└──────────────────────────────────────────────────────────────────────┘";

#[derive(Debug, Clone)]
struct CompBanner {
    title: String,
    target: String,
    compiler_name: String,
    rebuild: bool,
}

impl CompBanner {
    fn new(title: String, target: String, compiler_name: String, rebuild: bool) -> Self {
        CompBanner {
            title,
            target,
            compiler_name,
            rebuild,
        }
    }

    fn action_str(&self) -> &str {
        if self.rebuild {
            "Rebuild (Clean;Build)"
        } else {
            "Compile (Clean;Make)"
        }
    }

    fn base_lines(&self) -> Vec<String> {
        vec![
            format_line(&self.title, 72),
            format_line(&format!("→ {} ←", self.target), 70),
            format_line(&format!("🛠️ Compiler: {}", self.compiler_name), 70),
            format_line(&format!("🗲 Action: {}", self.action_str()), 70),
        ]
    }

    fn into_header_vec(&self) -> Vec<String> {
        let mut lines = vec![BANNER_TOP.to_string()];
        lines.extend(self.base_lines());
        lines.push(BANNER_BOTTOM.to_string());
        lines
    }

    fn into_multi_header_vec(&self) -> Vec<String> {
        let mut lines = vec![BANNER_MULTI_TOP.to_string()];
        lines.extend(self.base_lines());
        lines.push(BANNER_MULTI_BOTTOM.to_string());
        lines
    }

    fn into_project_header_vec(&self) -> Vec<String> {
        let mut lines = vec![BANNER_PROJECT_TOP.to_string()];
        lines.extend(self.base_lines());
        lines.push(BANNER_PROJECT_BOTTOM.to_string());
        lines
    }

    fn into_project_footer_vec(&self) -> Vec<String> {
        let cancelled = compiler_state::is_cancelled();
        let success = compiler_state::is_success();
        let status_str = if cancelled {
            "⚠️  CANCELLED"
        } else if success {
            "✅ SUCCESS"
        } else {
            "❌ FAILED"
        };
        let mut lines = vec![BANNER_PROJECT_TOP.to_string()];
        lines.extend(self.base_lines());
        lines.push(format_line(&format!("Status: {}", status_str), 70));
        lines.push(BANNER_PROJECT_BOTTOM.to_string());
        lines
    }

    fn into_footer_vec(&self) -> Vec<String> {
        let cancelled = compiler_state::is_cancelled();
        let success = compiler_state::is_success();
        let (status_str, top, bottom) = if cancelled {
            ("⚠️  CANCELLED", BANNER_CANCELLED_TOP, BANNER_CANCELLED_BOTTOM)
        } else if success {
            ("✅ SUCCESS", BANNER_SUCCESS_TOP, BANNER_SUCCESS_BOTTOM)
        } else {
            ("❌ FAILED", BANNER_ERROR_TOP, BANNER_ERROR_BOTTOM)
        };
        let mut lines = vec![top.to_string()];
        lines.extend(self.base_lines());
        lines.push(format_line(&format!("Status: {}", status_str), 70));
        lines.push(bottom.to_string());
        lines
    }
}
