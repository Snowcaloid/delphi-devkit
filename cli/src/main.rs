//! DDK CLI – command-line interface for Delphi project management.
//!
//! Thin wrapper around `ddk_core::commands`. Shares the same RON-based
//! state as ddk-server (LSP) and ddk-mcp-server, so changes made via the
//! CLI are automatically picked up by the other tools.

use anyhow::Result;
use clap::{Parser, Subcommand};

use ddk_core::commands;
use ddk_core::projects::{CompilerConfigurations, ProjectsData};
use ddk_core::state::Stateful;

/// DDK – Delphi Development Kit CLI
#[derive(Parser)]
#[command(name = "ddk", version, about, long_about = None)]
struct Cli {
    /// Output results as JSON instead of human-readable text.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage Delphi projects.
    #[command(subcommand)]
    Project(ProjectCmd),

    /// Manage Delphi compiler configurations.
    #[command(subcommand)]
    Compiler(CompilerCmd),

    /// Compile a project. Compiles the active project by default.
    Compile {
        /// Rebuild from scratch instead of incremental compile.
        #[arg(long)]
        rebuild: bool,

        /// Project ID to compile. If provided, selects the project first.
        #[arg(long, short)]
        project: Option<usize>,
    },

    /// Show environment info for the active project.
    Env,

    /// Print the DDK extension README.
    Info,
}

#[derive(Subcommand)]
enum ProjectCmd {
    /// List all known projects.
    List,
    /// Select a project by its numeric ID.
    Select {
        /// The project ID to select.
        id: usize,
    },
}

#[derive(Subcommand)]
enum CompilerCmd {
    /// List all available compiler configurations.
    List,
    /// Set the group project compiler by key.
    Set {
        /// The compiler configuration key (e.g. "12.0").
        key: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Ensure state files exist (creates defaults if first run).
    ProjectsData::initialize()?;
    CompilerConfigurations::initialize()?;

    let cli = Cli::parse();

    match cli.command {
        Commands::Info => {
            let readme = include_str!("../../README.md");
            println!("{readme}");
        }

        Commands::Env => {
            let info = commands::cmd_get_environment_info().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&info)?);
            } else {
                print!("{info}");
            }
        }

        Commands::Project(cmd) => match cmd {
            ProjectCmd::List => {
                let projects = commands::cmd_list_projects().await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&projects)?);
                } else if projects.is_empty() {
                    println!("No projects found.");
                } else {
                    for p in &projects {
                        println!("{p}");
                    }
                }
            }
            ProjectCmd::Select { id } => {
                let result = commands::cmd_select_project(id).await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{result}");
                }
            }
        },

        Commands::Compiler(cmd) => match cmd {
            CompilerCmd::List => {
                let compilers = commands::cmd_list_compilers().await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&compilers)?);
                } else if compilers.is_empty() {
                    println!("No compiler configurations available.");
                } else {
                    for c in &compilers {
                        println!("{c}");
                    }
                }
            }
            CompilerCmd::Set { key } => {
                let result = commands::cmd_set_group_compiler(key).await?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("{result}");
                }
            }
        },

        Commands::Compile { rebuild, project } => {
            let output = commands::cmd_compile(rebuild, project).await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                print!("{output}");
            }
        }
    }

    Ok(())
}
