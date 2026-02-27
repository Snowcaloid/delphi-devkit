//! Standalone MCP (Model Context Protocol) server for DDK.
//!
//! This binary exposes Delphi project management tools via STDIO transport,
//! making it compatible with any MCP client:
//!   - VS Code (registered via McpStdioServerDefinition)
//!   - Claude Desktop
//!   - Any other MCP-capable tool
//!
//! State is shared with ddk-server through RON files on disk. Changes to the
//! selected project or compiler are picked up by ddk-server's file watcher,
//! which pushes the updated state to VS Code automatically.

use ddk_projects::mcp::DdkMcpHandler;
use ddk_projects::projects::{ProjectsData, CompilerConfigurations};
use ddk_projects::state::Stateful;

use rust_mcp_sdk::{
    McpServer, ToMcpServerHandler, StdioTransport, TransportOptions,
    mcp_server::{server_runtime, McpServerOptions, ServerRuntime},
    schema::{
        InitializeResult, Implementation, ServerCapabilities, ServerCapabilitiesTools,
        ProtocolVersion,
    },
    error::SdkResult,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> SdkResult<()> {
    // Ensure state files exist (creates defaults if first run).
    ProjectsData::initialize().expect("Failed to initialize projects data");
    CompilerConfigurations::initialize().expect("Failed to initialize compiler configurations");

    let server_details = InitializeResult {
        server_info: Implementation {
            name: "ddk-mcp-server".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            title: Some("DDK - Delphi Development Kit".into()),
            description: Some(
                "MCP server for managing Delphi projects, compilers, and running compilations."
                    .into(),
            ),
            icons: vec![],
            website_url: None,
        },
        capabilities: ServerCapabilities {
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default()
        },
        protocol_version: ProtocolVersion::V2025_11_25.into(),
        instructions: Some(
            "Use these tools to query and manage Delphi projects and compiler configurations, \
             and to compile the currently active Delphi project."
                .into(),
        ),
        meta: None,
    };

    let transport = StdioTransport::new(TransportOptions::default())?;
    let handler = DdkMcpHandler;

    let server: Arc<ServerRuntime> = server_runtime::create_server(McpServerOptions {
        server_details,
        transport,
        handler: handler.to_mcp_server_handler(),
        task_store: None,
        client_task_store: None,
    });

    server.start().await
}
