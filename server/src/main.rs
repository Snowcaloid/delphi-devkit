pub mod projects;
pub mod lexorank;
pub mod lsp_types;
pub mod files;
pub mod utils;
pub mod format;

use std::sync::atomic::Ordering;
use anyhow::Result;
use tokio::io::{stdin, stdout};
use tower_lsp::{Client, async_trait, jsonrpc};
use tower_lsp::{LanguageServer, LspService, Server};
use tower_lsp::lsp_types::*;

pub(crate) use lsp_types::*;
use projects::*;
use crate::format::Formatter;

#[derive(Debug, Clone)]
struct DelphiLsp {
    client: Client,
}

impl DelphiLsp {
    pub fn new(client: Client) -> Self {
        return DelphiLsp { client }
    }

    async fn projects_compile(
        &self,
        params: CompileProjectParams,
    ) -> tower_lsp::jsonrpc::Result<()> {
        if let Err(e) = Compiler::new(self.client.clone(), &params).compile().await {
            NotifyError::notify(&self.client, format!("Failed to compile project: {}", e), None).await;
        }
        try_finish_event!(self.client, params);
    }

    async fn projects_compile_cancel(
        &self,
        _params: CancelCompilationParams,
    ) -> tower_lsp::jsonrpc::Result<()> {
        CANCEL_COMPILATION.store(true, std::sync::atomic::Ordering::SeqCst);
        try_finish_event!(self.client, "compilation cancelled");
    }

    async fn configuration_fetch(
        &self,
        _params: serde_json::Value,
    ) -> tower_lsp::jsonrpc::Result<ConfigurationFetchResponse> {
        Ok(ConfigurationFetchResponse {
            projects: ProjectsData::new(),
            compilers: CompilerConfigurations::new(),
        })
    }

    async fn custom_document_format(
        &self,
        params: CustomDocumentFormat,
    ) -> tower_lsp::jsonrpc::Result<TextEdit> {
        let formatter = Formatter::new(params.content)
            .map_err(|error| {
                lsp_error!(self.client, "Failed to initialize formatter: {}", error);
                jsonrpc::Error::invalid_params(format!(
                    "Failed to initialize formatter: {}",
                    error
                ))
            })?;
        let new_text = formatter.execute().map_err(|error| {
            lsp_error!(self.client, "Failed to format document: {}", error);
            jsonrpc::Error::invalid_params(format!(
                "Failed to format document: {}",
                error
            ))
        })?;
        let range = params.range.unwrap_or(Range::new(Position::new(0,0), Position::new(u32::MAX, u32::MAX)));
        return Ok(TextEdit {
            range,
            new_text,
        });
    }
}

#[macro_export]
macro_rules! lsp_debug {
    ($client:expr, $($arg:tt)*) => {
        let inner = $client.clone();
        let inner_message = format!($($arg)*);
        tokio::spawn(async move {
            inner.log_message(tower_lsp::lsp_types::MessageType::LOG, inner_message).await;
        });
    };
}

#[macro_export]
macro_rules! lsp_info {
    ($client:expr, $($arg:tt)*) => {
        let inner = $client.clone();
        let inner_message = format!($($arg)*);
        tokio::spawn(async move {
            inner.log_message(tower_lsp::lsp_types::MessageType::INFO, inner_message).await;
        });
    };
}

#[macro_export]
macro_rules! lsp_error {
    ($client:expr, $($arg:tt)*) => {
        let inner = $client.clone();
        let inner_message = format!($($arg)*);
        tokio::spawn(async move {
            inner.log_message(tower_lsp::lsp_types::MessageType::ERROR, inner_message).await;
        });
    };
}

#[async_trait]
impl LanguageServer for DelphiLsp {
    async fn initialize(&self, _params: InitializeParams) -> jsonrpc::Result<InitializeResult> {
        return Ok(InitializeResult {
            capabilities: ServerCapabilities::default(), // none
            server_info: Some(ServerInfo {
                name: "DDK - Delphi Server".to_string(),
                version: Some("0.1.0".to_string()),
            }),
        });
    }

    async fn initialized(&self, _params: InitializedParams) {
        lsp_info!(self.client, "Delphi LSP Relay server initialized");
    }

    async fn shutdown(&self) -> jsonrpc::Result<()> {
        CANCEL_COMPILATION.store(true, Ordering::SeqCst);
        return Ok(())
    }

    async fn did_change_configuration(&self, params: DidChangeConfigurationParams) {
        let client = self.client.clone();
        let settings = params.settings.clone();
        if let Err(error) = projects::update(settings.clone(), client).await {
            lsp_error!(self.client, "Failed to apply configuration changes: {}", error);
            NotifyError::notify_json(&self.client, format!("Failed to apply configuration changes: {}", error), &settings).await;
        }
        try_finish_event!(self.client, settings, ());
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let (service, socket) = LspService::build(|client| {
        let watcher_client = client.clone();
        tokio::spawn(async move {
            let _ = ProjectsData::initialize()
                .expect("Failed to initialize projects data");
            let _ = CompilerConfigurations::initialize()
                .expect("Failed to initialize compiler configuration");
            if let Err(e) = start_file_watchers(watcher_client) {
                eprintln!("File watcher error: {}", e);
            }
        });
        DelphiLsp::new(client)
    })
        .custom_method("projects/compile", DelphiLsp::projects_compile)
        .custom_method("configuration/fetch", DelphiLsp::configuration_fetch)
        .custom_method("projects/compile-cancel", DelphiLsp::projects_compile_cancel)
        .custom_method("custom/document/format", DelphiLsp::custom_document_format)
        .finish();

    Server::new(stdin(), stdout(), socket).serve(service).await;

    return Ok(())
}