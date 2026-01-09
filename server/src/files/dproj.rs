use anyhow::Result;
use std::path::PathBuf;

pub fn get_main_source(dproj_path: &PathBuf) -> Result<PathBuf> {
    let content = std::fs::read_to_string(dproj_path)?;
    let parent_directory = dproj_path.parent().ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;
    let xml_content = roxmltree::Document::parse(&content)?;
    for property_group in xml_content.descendants().filter(|n| n.has_tag_name("PropertyGroup")) {
        let main_source_node = property_group
            .children()
            .find(|n| n.has_tag_name("MainSource"));
        if let Some(main_source_node) = main_source_node {
            if let Some(path) = main_source_node.text() {
                let main_source_path = parent_directory.join(path);
                if main_source_path.exists() {
                    return Ok(main_source_path);
                }
            }
        }
    }
    anyhow::bail!("Main source file not found in DPROJ");
}

pub fn get_exe_path(dproj_path: &PathBuf) -> Result<PathBuf> {
    let content = std::fs::read_to_string(dproj_path)?;
    let parent_directory = dproj_path.parent().ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;
    let exe_file_name = dproj_path.with_extension("exe");
    let exe_file_name = exe_file_name.file_stem().ok_or_else(|| anyhow::anyhow!("Failed to get exe file name"))?;
    let xml_content = roxmltree::Document::parse(&content)?;
    for property_group in xml_content.descendants().filter(|n| n.has_tag_name("PropertyGroup")) {
        let output_dir_node = property_group
            .children()
            .find(|n| n.has_tag_name("DCC_DependencyCheckOutputName"));
        if let Some(output_dir_node) = output_dir_node {
            if let Some(path) = output_dir_node.text() {
                let output_dir_path = parent_directory.join(path);
                if output_dir_path.exists() {
                    return Ok(output_dir_path);
                }
            }
        }
        let dcc_exe_output = property_group
            .children()
            .find(|n| n.has_tag_name("DCC_ExeOutput"));
        if let Some(output_dir_node) = dcc_exe_output {
            if let Some(path) = output_dir_node.text() {
                let output_dir_path = parent_directory.join(path).join(exe_file_name);
                if output_dir_path.exists() {
                    return Ok(output_dir_path);
                }
            }
        }
    }
    anyhow::bail!("Output directory not found in DPROJ");
}

pub fn find_dproj_file(main_file_path: &PathBuf) -> Result<PathBuf> {
    let dproj_path = main_file_path.with_extension("dproj");
    if dproj_path.exists() {
        return Ok(dproj_path);
    } else {
        anyhow::bail!("DPROJ file not found for main file: {}", main_file_path.display());
    }
}