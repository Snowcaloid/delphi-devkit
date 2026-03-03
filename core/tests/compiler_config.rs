use ddk_core::projects::{CompilerConfiguration, PartialCompilerConfiguration, sanitize_build_arguments};

// ═══════════════════════════════════════════════════════════════════════════════
//  CompilerConfiguration::update
// ═══════════════════════════════════════════════════════════════════════════════

fn sample_config() -> CompilerConfiguration {
    CompilerConfiguration {
        condition: "12.0".to_string(),
        product_name: "Delphi 12".to_string(),
        product_version: 29,
        package_version: 230,
        compiler_version: 36,
        installation_path: r"C:\Delphi\12.0".to_string(),
        build_arguments: vec!["/t:Build".to_string()],
    }
}

#[test]
fn update_all_none_changes_nothing() {
    let mut config = sample_config();
    let original = config.clone();
    let partial = PartialCompilerConfiguration {
        condition: None,
        product_name: None,
        product_version: None,
        package_version: None,
        compiler_version: None,
        installation_path: None,
        build_arguments: None,
    };
    config.update(&partial);
    assert_eq!(config, original);
}

#[test]
fn update_all_some_applies_all() {
    let mut config = sample_config();
    let partial = PartialCompilerConfiguration {
        condition: Some("14.0".to_string()),
        product_name: Some("Delphi 14".to_string()),
        product_version: Some(31),
        package_version: Some(250),
        compiler_version: Some(38),
        installation_path: Some(r"C:\Delphi\14.0".to_string()),
        build_arguments: Some(vec!["/t:Rebuild".to_string()]),
    };
    config.update(&partial);
    assert_eq!(config.condition, "14.0");
    assert_eq!(config.product_name, "Delphi 14");
    assert_eq!(config.product_version, 31);
    assert_eq!(config.package_version, 250);
    assert_eq!(config.compiler_version, 38);
    assert_eq!(config.installation_path, r"C:\Delphi\14.0");
    assert_eq!(config.build_arguments, vec!["/t:Rebuild"]);
}

#[test]
fn update_partial_fields() {
    let mut config = sample_config();
    let partial = PartialCompilerConfiguration {
        condition: None,
        product_name: Some("New Name".to_string()),
        product_version: None,
        package_version: None,
        compiler_version: None,
        installation_path: None,
        build_arguments: None,
    };
    config.update(&partial);
    assert_eq!(config.product_name, "New Name");
    assert_eq!(config.condition, "12.0"); // unchanged
}

// ═══════════════════════════════════════════════════════════════════════════════
//  sanitize_build_arguments
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn sanitize_removes_configuration_arg() {
    let mut args = vec![
        "/t:Build".to_string(),
        "/p:Configuration=Debug".to_string(),
    ];
    sanitize_build_arguments(&mut args);
    assert_eq!(args, vec!["/t:Build"]);
}

#[test]
fn sanitize_removes_platform_arg() {
    let mut args = vec![
        "/p:Platform=Win32".to_string(),
        "/t:Build".to_string(),
    ];
    sanitize_build_arguments(&mut args);
    assert_eq!(args, vec!["/t:Build"]);
}

#[test]
fn sanitize_removes_both() {
    let mut args = vec![
        "/p:Configuration=Release".to_string(),
        "/p:Platform=Win64".to_string(),
        "/t:Build".to_string(),
    ];
    sanitize_build_arguments(&mut args);
    assert_eq!(args, vec!["/t:Build"]);
}

#[test]
fn sanitize_case_insensitive() {
    let mut args = vec![
        "/P:CONFIGURATION=Debug".to_string(),
        "/P:PLATFORM=Win32".to_string(),
        "/t:Build".to_string(),
    ];
    sanitize_build_arguments(&mut args);
    assert_eq!(args, vec!["/t:Build"]);
}

#[test]
fn sanitize_preserves_unrelated_args() {
    let mut args = vec![
        "/t:Build".to_string(),
        "/p:OutputDir=bin".to_string(),
        "/v:minimal".to_string(),
    ];
    let expected = args.clone();
    sanitize_build_arguments(&mut args);
    assert_eq!(args, expected);
}

#[test]
fn sanitize_empty_args() {
    let mut args: Vec<String> = vec![];
    sanitize_build_arguments(&mut args);
    assert!(args.is_empty());
}
