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
use serde_json::Value;
use std::sync::Arc;

use ddk_core::commands;

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
    description = "Lists all known Delphi projects grouped by their workspace or group project. Each workspace has its own compiler configuration. Projects are shown with their IDs, names, and paths. Use this to discover available projects and their hierarchy before selecting one."
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
    /// Optional project ID to compile. If provided, that project is selected
    /// first. If omitted, the currently active project is compiled.
    pub project_id: Option<u64>,
}

#[macros::mcp_tool(
    name = "delphi_format_file",
    description = "Formats a Delphi source file (.pas / .dpr / .dpk) in-place using the DDK formatter. \
        The file is read from disk, reformatted, and written back to the same path. \
        Requires at least one Delphi compiler installation to be present. \
        Specify the encoding when the file is not UTF-8, e.g. \"windows-1252\" for ANSI or \"oem\" for the system OEM codepage."
)]
#[derive(Debug, Deserialize, Serialize, macros::JsonSchema)]
pub struct FormatFileArgs {
    /// Absolute or relative path to the Delphi source file to format.
    pub file_path: String,
    /// Encoding of the source file, e.g. "utf-8", "windows-1252", "oem".
    /// Defaults to "utf-8" when not specified.
    pub encoding: Option<String>,
}

rust_mcp_sdk::tool_box!(DdkTools, [
    GetDdkExtensionInfoArgs,
    GetEnvironmentInfoArgs,
    ListProjectsArgs,
    SelectProjectArgs,
    GetAvailableCompilersArgs,
    SetGroupProjectsCompilerArgs,
    CompileSelectedProjectArgs,
    FormatFileArgs,
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
            "delphi_format_file"              => format_file(&args).await,
            _ => format!("Unknown tool: {name}"),
        };
        Ok(CallToolResult::text_content(vec![TextContent::from(result_text)]))
    }
}

async fn get_ddk_extension_info() -> String {
    README_CONTENT.to_string()
}

async fn get_environment_info() -> String {
    match commands::cmd_get_environment_info().await {
        Ok(info) => serde_json::to_string_pretty(&info).unwrap_or_default(),
        Err(e) => format!("Error: {e}"),
    }
}

async fn list_projects() -> String {
    match commands::cmd_list_projects().await {
        Ok(result) => serde_json::to_string_pretty(&result).unwrap_or_default(),
        Err(e) => format!("Error: {e}"),
    }
}

async fn select_project(args: &Value) -> String {
    let project_id = match args.get("project_id").and_then(|v| v.as_u64()) {
        Some(id) => id as usize,
        None => return "Missing required parameter: project_id".to_string(),
    };
    match commands::cmd_select_project(project_id).await {
        Ok(result) => result.to_string(),
        Err(e) => format!("{e}"),
    }
}

async fn get_available_compilers() -> String {
    match commands::cmd_list_compilers().await {
        Ok(compilers) => {
            if compilers.is_empty() {
                return "No compiler configurations available.".to_string();
            }
            serde_json::to_string_pretty(&compilers).unwrap_or_default()
        }
        Err(e) => format!("Error: {e}"),
    }
}

async fn set_group_projects_compiler(args: &Value) -> String {
    let compiler_key = match args.get("compiler").and_then(|v| v.as_str()) {
        Some(k) => k.to_string(),
        None => return "Missing required parameter: compiler".to_string(),
    };
    match commands::cmd_set_group_compiler(compiler_key).await {
        Ok(result) => result.to_string(),
        Err(e) => format!("{e}"),
    }
}

async fn compile_selected_project(args: &Value) -> String {
    let rebuild = args.get("rebuild").and_then(|v| v.as_bool()).unwrap_or(false);
    let project_id = args.get("project_id").and_then(|v| v.as_u64()).map(|id| id as usize);
    match commands::cmd_compile(rebuild, project_id).await {
        Ok(output) => output.to_string(),
        Err(e) => format!("{e}"),
    }
}
