use async_trait::async_trait;
use rust_mcp_sdk::{
    McpServer,
    macros,
    mcp_server::ServerHandler,
    schema::{
        schema_utils::CallToolError, CallToolRequestParams, CallToolResult,
        ListToolsResult, PaginatedRequestParams, RpcError, TextContent,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::sync::Arc;

use ddk_core::projects::*;
use ddk_core::state::*;

// ---------------------------------------------------------------------------
// README content embedded at compile time
// ---------------------------------------------------------------------------

static README_CONTENT: &str = include_str!("../../README.md");

// ---------------------------------------------------------------------------
// Tool input types (mcp_tool! generates ::tool() returning a Tool definition)
// ---------------------------------------------------------------------------

#[macros::mcp_tool(
    name = "get_ddk_extension_info",
    description = "Returns the DDK (Delphi Development Kit) extension README, describing all available features, commands, settings, and project views. Use this to understand what the extension can do."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct GetDdkExtensionInfoArgs {}

#[macros::mcp_tool(
    name = "delphi_get_environment_info",
    description = "Returns the currently active Delphi project and its associated compiler configuration. If no project is active, returns only the group project compiler configuration (if any). This information is best presented in a small formatted table. This is only relevant if we are working with Delphi."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct GetEnvironmentInfoArgs {}

#[macros::mcp_tool(
    name = "delphi_list_projects",
    description = "Lists all known Delphi projects with their IDs, names, and key paths. Use this when no project is currently selected to discover available projects before selecting one."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct ListProjectsArgs {}

#[macros::mcp_tool(
    name = "delphi_select_project",
    description = "Selects a Delphi project by its ID, making it the active project for subsequent operations (compile, run, etc.). Use delphi_list_projects first to discover available project IDs."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct SelectProjectArgs {
    /// The numeric ID of the project to select.
    pub project_id: u64,
}

#[macros::mcp_tool(
    name = "delphi_get_available_compilers",
    description = "Returns all available Delphi compiler configurations with their keys, product names, versions, and installation paths. Use this to discover valid compiler keys before calling delphi_set_group_projects_compiler. If this information is asked for from the user, it is most useful to present it in a clearly formatted table."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct GetAvailableCompilersArgs {}

#[macros::mcp_tool(
    name = "delphi_set_group_projects_compiler",
    description = "Sets the compiler configuration used by the group project. The compiler parameter must be a valid compiler configuration key from the available configurations. Call delphi_get_available_compilers first to discover the available compiler keys."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct SetGroupProjectsCompilerArgs {
    /// The compiler configuration key to set for the group project.
    pub compiler: String,
}

#[macros::mcp_tool(
    name = "delphi_compile_selected_project",
    description = "Compiles the currently selected/active Delphi project. \
        BEFORE calling this tool: always call delphi_get_environment_info first to \
        check which project is active, then verify it matches what the user asked for. \
        When matching: always prioritize an explicit project name in the user's request \
        (e.g. 'compile be', 'build MyProject') over whatever file is currently open in \
        the editor. Match by project name, then use delphi_select_project to switch if \
        needed. Never assume the active project is correct without checking. \
        The tool returns the raw compiler output as a single string. \
        Because this runs via MCP (not LSP), the user cannot see any output directly — \
        YOU are responsible for presenting the results. \
        Always show: (1) banner lines from the start of compilation, \
        (2) all errors verbatim with file path and line number, \
        (3) the final summary line (e.g. 'Compile ok' or error count). \
        For hints and warnings: only surface those in files you modified during \
        the current session — the project may have hundreds of pre-existing warnings \
        that are not relevant to the current change."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct CompileSelectedProjectArgs {
    /// If true, rebuilds the project from scratch. If false, performs an incremental compile.
    pub rebuild: Option<bool>,
}

rust_mcp_sdk::tool_box!(DdkTools, [
    GetDdkExtensionInfoArgs,
    GetEnvironmentInfoArgs,
    ListProjectsArgs,
    SelectProjectArgs,
    GetAvailableCompilersArgs,
    SetGroupProjectsCompilerArgs,
    CompileSelectedProjectArgs,
]);

// ---------------------------------------------------------------------------
// MCP server handler
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct DdkMcpHandler;

#[async_trait]
impl ServerHandler for DdkMcpHandler {
    async fn handle_list_tools_request(
        &self,
        _request: Option<PaginatedRequestParams>,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<ListToolsResult, RpcError> {
        Ok(ListToolsResult {
            tools: DdkTools::tools(),
            meta: None,
            next_cursor: None,
        })
    }

    async fn handle_call_tool_request(
        &self,
        params: CallToolRequestParams,
        _runtime: Arc<dyn McpServer>,
    ) -> Result<CallToolResult, CallToolError> {
        let name = params.name.as_str();
        let args = Value::Object(params.arguments.clone().unwrap_or_default());
        let result_text = match name {
            "get_ddk_extension_info"          => get_ddk_extension_info().await,
            "delphi_get_environment_info"     => get_environment_info().await,
            "delphi_list_projects"            => list_projects().await,
            "delphi_select_project"           => select_project(&args).await,
            "delphi_get_available_compilers"  => get_available_compilers().await,
            "delphi_set_group_projects_compiler" => set_group_projects_compiler(&args).await,
            "delphi_compile_selected_project" => compile_selected_project(&args).await,
            _ => format!("Unknown tool: {name}"),
        };
        Ok(CallToolResult::text_content(vec![TextContent::from(result_text)]))
    }
}

async fn get_ddk_extension_info() -> String {
    README_CONTENT.to_string()
}

async fn get_environment_info() -> String {
    let projects_data = PROJECTS_DATA.read().await;
    let compilers = COMPILER_CONFIGURATIONS.read().await;

    let active_project = match projects_data.active_project_id {
        Some(id) => projects_data.get_project(id),
        None => None,
    };

    match active_project {
        Some(project) => {
            let mut compiler_map = serde_json::Map::new();

            for workspace in &projects_data.workspaces {
                for link in &workspace.project_links {
                    if link.project_id == project.id {
                        if let Some(compiler) = compilers.get(&workspace.compiler_id) {
                            compiler_map.insert(
                                workspace.name.clone(),
                                serde_json::to_value(compiler).unwrap_or(Value::Null),
                            );
                        }
                    }
                }
            }

            if let Some(group_project) = &projects_data.group_project {
                for link in &group_project.project_links {
                    if link.project_id == project.id {
                        if let Some(compiler) =
                            compilers.get(&projects_data.group_project_compiler_id)
                        {
                            compiler_map.insert(
                                "group_project".to_string(),
                                serde_json::to_value(compiler).unwrap_or(Value::Null),
                            );
                        }
                    }
                }
            }

            let result = json!({
                "project": serde_json::to_value(project).unwrap_or(Value::Null),
                "compilers": compiler_map,
            });
            serde_json::to_string_pretty(&result).unwrap_or_default()
        }
        None => {
            let compiler = compilers.get(&projects_data.group_project_compiler_id);
            match compiler {
                Some(c) => format!(
                    "No active project.\n\nGroup project compiler:\n{}",
                    serde_json::to_string_pretty(c).unwrap_or_default()
                ),
                None => "No active project and no group project compiler configured.".to_string(),
            }
        }
    }
}

async fn list_projects() -> String {
    let projects_data = PROJECTS_DATA.read().await;
    if projects_data.projects.is_empty() {
        return "No projects found.".to_string();
    }

    let active_id = projects_data.active_project_id;
    let list: Vec<Value> = projects_data
        .projects
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "name": p.name,
                "directory": p.directory,
                "dproj": p.dproj,
                "active": Some(p.id) == active_id,
            })
        })
        .collect();

    serde_json::to_string_pretty(&list).unwrap_or_default()
}

async fn select_project(args: &Value) -> String {
    let project_id = match args.get("project_id").and_then(|v| v.as_u64()) {
        Some(id) => id as usize,
        None => return "Missing required parameter: project_id".to_string(),
    };

    {
        let data = PROJECTS_DATA.read().await;
        if data.get_project(project_id).is_none() {
            return format!("No project found with ID {project_id}.");
        }
    }

    let change = Change::SelectProject { project_id };
    match change.execute().await {
        Ok(_) => {
            let data = PROJECTS_DATA.read().await;
            let name = data
                .get_project(project_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("ID {project_id}"));
            // Note: ddk-server's file watcher will detect the RON change
            // and send ProjectsUpdate to VS Code automatically.
            format!("Selected project: {name} (ID {project_id}).")
        }
        Err(e) => format!("Failed to select project: {e}"),
    }
}

async fn get_available_compilers() -> String {
    let configs = COMPILER_CONFIGURATIONS.read().await;
    let list: Vec<Value> = configs
        .iter()
        .map(|(key, cfg)| {
            json!({
                "key": key,
                "product_name": cfg.product_name,
                "product_version": cfg.product_version,
                "compiler_version": cfg.compiler_version,
                "installation_path": cfg.installation_path,
            })
        })
        .collect();

    if list.is_empty() {
        return "No compiler configurations available.".to_string();
    }

    serde_json::to_string_pretty(&list).unwrap_or_default()
}

async fn set_group_projects_compiler(args: &Value) -> String {
    let compiler_key = match args.get("compiler").and_then(|v| v.as_str()) {
        Some(k) => k.to_string(),
        None => return "Missing required parameter: compiler".to_string(),
    };

    {
        let configs = COMPILER_CONFIGURATIONS.read().await;
        if !configs.contains_key(&compiler_key) {
            let available: Vec<String> = configs.keys().cloned().collect();
            return format!(
                "Unknown compiler key: \"{compiler_key}\". Available keys: {}",
                available.join(", ")
            );
        }
    }

    let change = Change::SetGroupProjectCompiler {
        compiler: compiler_key.clone(),
    };
    match change.execute().await {
        Ok(_) => {
            // Note: ddk-server's file watcher will detect the RON change
            // and send CompilersUpdate/ProjectsUpdate to VS Code automatically.
            let configs = COMPILER_CONFIGURATIONS.read().await;
            let name = configs
                .get(&compiler_key)
                .map(|c| c.product_name.clone())
                .unwrap_or_default();
            format!("Group project compiler set to: {name} ({compiler_key}).")
        }
        Err(e) => format!("Failed to set group project compiler: {e}"),
    }
}

async fn compile_selected_project(args: &Value) -> String {
    let rebuild = args.get("rebuild").and_then(|v| v.as_bool()).unwrap_or(false);

    let (project_name, project_id, link_id) = {
        let data = PROJECTS_DATA.read().await;
        let active_id = match data.active_project_id {
            Some(id) => id,
            None => return "No active project selected.".to_string(),
        };
        let project = match data.get_project(active_id) {
            Some(p) => p,
            None => return "Active project not found.".to_string(),
        };
        let name = project.name.clone();

        let mut found_link: Option<usize> = None;
        for ws in &data.workspaces {
            if let Some(link) = ws.project_links.iter().find(|l| l.project_id == active_id) {
                found_link = Some(link.id);
                break;
            }
        }
        if found_link.is_none() {
            if let Some(gp) = &data.group_project {
                if let Some(link) = gp.project_links.iter().find(|l| l.project_id == active_id) {
                    found_link = Some(link.id);
                }
            }
        }
        match found_link {
            Some(lid) => (name, active_id, lid),
            None => {
                return format!("Project \"{name}\" has no compiled links.");
            }
        }
    };

    let params = ddk_core::lsp_types::CompileProjectParams::Project {
        project_id,
        project_link_id: Some(link_id),
        rebuild,
        event_id: "mcp-compile".to_string(),
    };

    // Collect broadcast messages concurrently with compilation.
    // Using try_recv() after the fact is unreliable: the loop exits early on
    // Lagged errors (buffer overflow) and there is a race between spawned
    // output-processing tasks and the outer await returning.  Instead we
    // drive a dedicated collection task in parallel and abort it once the
    // compile has finished (plus a small settling window for in-flight sends).
    let collected: std::sync::Arc<std::sync::Mutex<Vec<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let collected_clone = collected.clone();
    let mut receiver = ddk_core::lsp_types::CompilerProgress::subscribe();

    let collect_handle = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(event) => {
                    let mut lines = collected_clone.lock().unwrap();
                    match event {
                        ddk_core::lsp_types::CompilerProgressParams::Start { lines: ls }
                        | ddk_core::lsp_types::CompilerProgressParams::SingleProjectStarted { lines: ls, .. }
                        | ddk_core::lsp_types::CompilerProgressParams::Completed { lines: ls, .. }
                        | ddk_core::lsp_types::CompilerProgressParams::SingleProjectCompleted { lines: ls, .. } => {
                            lines.extend(ls);
                        }
                        ddk_core::lsp_types::CompilerProgressParams::Stdout { line }
                        | ddk_core::lsp_types::CompilerProgressParams::Stderr { line } => {
                            lines.push(line);
                        }
                    }
                }
                // Channel closed – shouldn't happen with a static sender, but
                // treat it as a clean exit.
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                // We fell behind; skip the dropped messages and keep reading.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    });

    let compiler = Compiler::new_standalone(&params).await;
    let compile_result = compiler.compile().await;

    // Brief settling window so any in-flight broadcast sends from the
    // compiler's spawned output tasks have time to arrive, then stop.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    collect_handle.abort();
    let _ = collect_handle.await; // wait for the abort to fully unwind

    // The spawned task has dropped its Arc clone, so try_unwrap should succeed.
    let output_lines = match std::sync::Arc::try_unwrap(collected) {
        Ok(mutex) => mutex.into_inner().unwrap_or_default(),
        Err(arc)  => arc.lock().unwrap().clone(),
    };

    let output_section = if output_lines.is_empty() {
        String::new()
    } else {
        format!("\n\nCompiler output:\n{}", output_lines.join("\n"))
    };

    match compile_result {
        Ok(result) => {
            let summary = if result.cancelled {
                format!("Compilation of \"{project_name}\" was cancelled.")
            } else if result.success {
                format!("Project \"{project_name}\" compiled successfully.")
            } else {
                format!(
                    "Compilation of \"{project_name}\" finished with errors (exit code {}).",
                    result.code
                )
            };
            format!("{summary}{output_section}")
        }
        Err(e) => format!("Compilation failed: {e}{output_section}"),
    }
}
