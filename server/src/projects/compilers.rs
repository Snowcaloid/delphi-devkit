use anyhow::Result;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub(crate) const DEFAULT_COMPILERS: &str = include_str!("presets/default_compilers.ron");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialCompilerConfiguration {
    pub condition: Option<String>,
    pub product_name: Option<String>,
    pub product_version: Option<f32>,
    pub package_version: Option<usize>,
    pub compiler_version: Option<f32>,
    pub installation_path: Option<String>,
    pub build_arguments: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfiguration {
    pub condition: String,
    pub product_name: String,
    pub product_version: f32,
    pub package_version: usize,
    pub compiler_version: f32,
    pub installation_path: String,
    pub build_arguments: Vec<String>,
}

impl CompilerConfiguration {
    pub fn update(&mut self, partial: &PartialCompilerConfiguration) {
        if let Some(condition) = &partial.condition {
            self.condition = condition.clone();
        }
        if let Some(product_name) = &partial.product_name {
            self.product_name = product_name.clone();
        }
        if let Some(product_version) = partial.product_version {
            self.product_version = product_version;
        }
        if let Some(package_version) = partial.package_version {
            self.package_version = package_version;
        }
        if let Some(compiler_version) = partial.compiler_version {
            self.compiler_version = compiler_version;
        }
        if let Some(installation_path) = &partial.installation_path {
            self.installation_path = installation_path.clone();
        }
        if let Some(build_arguments) = &partial.build_arguments {
            self.build_arguments = build_arguments.clone();
        }
    }
}

pub type CompilerConfigurations = HashMap<String, CompilerConfiguration>;

fn compilers_file_path() -> Result<std::path::PathBuf> {
    let path = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("Could not determine config directory"))?
        .join("ddk")
        .join("compilers.ron");
    return Ok(path)
}

pub fn load_compilers() -> Result<CompilerConfigurations> {
    let path = compilers_file_path()?;

    if path.exists() {
        let content = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("Failed to read compilers file: {}", e))?;
        let compilers: CompilerConfigurations = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("Failed to parse compilers file: {}", e))?;
        return Ok(compilers)
    } else {
        let compilers: CompilerConfigurations = serde_json::from_str::<CompilerConfigurations>(DEFAULT_COMPILERS).map_err(|e| anyhow::anyhow!("Failed to parse default compilers: {}", e))?;
        return Ok(compilers)
    }
}

pub fn compiler_exists(key: &str) -> Result<bool> {
    let compilers = load_compilers()?;
    Ok(compilers.contains_key(key))
}

pub fn validate_compilers(compilers: &CompilerConfigurations) -> Result<()> {
    for (key, compiler) in compilers {
        if key.trim().is_empty() {
            anyhow::bail!("Compiler key cannot be empty.");
        }
        if compiler.condition.trim().is_empty() {
            anyhow::bail!("Compiler condition cannot be empty for key: {}", key);
        }
        if compiler.product_name.trim().is_empty() {
            anyhow::bail!("Compiler product name cannot be empty for key: {}", key);
        }
        if compiler.installation_path.trim().is_empty() {
            anyhow::bail!("Compiler installation path cannot be empty for key: {}", key);
        }
        let path = PathBuf::from(&compiler.installation_path);
        if !path.exists() {
            anyhow::bail!("Compiler installation path does not exist for key: {}: {}", key, compiler.installation_path);
        }
        if !path.is_dir() {
            anyhow::bail!("Compiler installation path is not a directory for key: {}: {}", key, compiler.installation_path);
        }
        let rsvars_path = path.join("bin").join("rsvars.bat");
        if !rsvars_path.exists() {
            anyhow::bail!("rsvars.bat not found in compiler installation path for key: {}: {}", key, rsvars_path.display());
        }
    }
    Ok(())
}

pub fn save_compilers(compilers: &CompilerConfigurations) -> Result<()> {
    let path = compilers_file_path()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| anyhow::anyhow!("Failed to create config directory: {}", e))?;
    }

    let content = serde_json::to_string_pretty(compilers)
        .map_err(|e| anyhow::anyhow!("Failed to serialize compilers: {}", e))?;
    std::fs::write(&path, content)
        .map_err(|e| anyhow::anyhow!("Failed to write compilers file: {}", e))?;
    Ok(())
}
