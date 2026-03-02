use anyhow::Result;
use roxmltree::Document;
use std::path::PathBuf;

pub fn parse_groupproj(file_path: PathBuf) -> Result<Vec<PathBuf>> {
    let content = std::fs::read_to_string(&file_path)?;
    let parent_directory = file_path.parent().ok_or_else(|| anyhow::anyhow!("Failed to get parent directory"))?;
    let xml_content = Document::parse(&content)?;
    let mut project_paths = Vec::new();
    let item_groups = xml_content.descendants().filter(|n| n.has_tag_name("ItemGroup"));
    for item_group in item_groups {
        let projects = item_group
            .children()
            .filter(|n| n.has_tag_name("Projects"));
        for project in projects {
            if let Some(include_attr) = project.attribute("Include") {
                let project_path = parent_directory.join(include_attr);
                if project_path.exists() {
                    project_paths.push(project_path);
                }
            }
        }
    }
    Ok(project_paths)
}