use ddk_core::projects::CompilerLineDiagnostic;

// ═══════════════════════════════════════════════════════════════════════════════
//  CompilerLineDiagnostic::from_line – valid inputs
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn parses_full_format_with_column() {
    let line = r"C:\Projects\Unit1.pas(42,5): error E2003: Undeclared identifier: 'Foo' [C:\Projects\MyProject.dproj]";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into());
    assert!(diag.is_some());
    let diag = diag.unwrap();
    assert_eq!(diag.file, r"C:\Projects\Unit1.pas");
    assert_eq!(diag.line, 42);
    assert_eq!(diag.column, Some(5));
    assert_eq!(diag.code, "E2003");
    assert_eq!(diag.message, "Undeclared identifier: 'Foo'");
    assert_eq!(diag.compiler_name, "dcc32");
}

#[test]
fn parses_format_without_column() {
    let line = r"Unit1.pas(10): warning W1000: Symbol 'X' is deprecated";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc64".into());
    assert!(diag.is_some());
    let diag = diag.unwrap();
    assert_eq!(diag.file, "Unit1.pas");
    assert_eq!(diag.line, 10);
    assert_eq!(diag.column, None);
    assert_eq!(diag.code, "W1000");
}

#[test]
fn parses_hint() {
    let line = r"Unit2.pas(100): hint H2164: Variable 'Y' is declared but never used";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into()).unwrap();
    assert_eq!(diag.code, "H2164");
    assert!(format!("{}", diag.kind) == "HINT");
}

#[test]
fn parses_fatal_error() {
    let line = r"Unit3.pas(1): fatal F2039: Could not create output file";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into()).unwrap();
    assert_eq!(diag.code, "F2039");
    // Fatal errors start with 'F', which falls through to ERROR
    assert!(format!("{}", diag.kind) == "ERROR");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  CompilerLineDiagnostic::from_line – non-matching inputs
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn rejects_non_matching_line() {
    assert!(CompilerLineDiagnostic::from_line("Build succeeded.", "dcc32".into()).is_none());
}

#[test]
fn rejects_empty_string() {
    assert!(CompilerLineDiagnostic::from_line("", "dcc32".into()).is_none());
}

#[test]
fn rejects_random_text() {
    assert!(CompilerLineDiagnostic::from_line("Something completely different", "dcc32".into()).is_none());
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Severity classification by code prefix
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn severity_error_from_e_prefix() {
    let line = r"file.pas(1): error E1234: some error";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc".into()).unwrap();
    assert_eq!(format!("{}", diag.kind), "ERROR");
}

#[test]
fn severity_warning_from_w_prefix() {
    let line = r"file.pas(1): warning W5678: some warning";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc".into()).unwrap();
    assert_eq!(format!("{}", diag.kind), "WARN");
}

#[test]
fn severity_hint_from_h_prefix() {
    let line = r"file.pas(1): hint H9999: some hint";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc".into()).unwrap();
    assert_eq!(format!("{}", diag.kind), "HINT");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Display impl
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn display_with_column() {
    let line = r"file.pas(10,5): error E2003: something";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into()).unwrap();
    let display = format!("{}", diag);
    assert!(display.contains("[ERROR]"));
    assert!(display.contains("[E2003]"));
    assert!(display.contains("file.pas:10:5"));
    assert!(display.contains("something"));
}

#[test]
fn display_without_column() {
    let line = r"file.pas(10): warning W1000: something";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into()).unwrap();
    let display = format!("{}", diag);
    assert!(display.contains("[WARN]"));
    assert!(display.contains("file.pas:10"));
    assert!(!display.contains("file.pas:10:"));
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Into<Diagnostic> – LSP conversion
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn lsp_diagnostic_line_is_zero_based() {
    use tower_lsp::lsp_types::Diagnostic;

    let line = r"file.pas(10,5): error E2003: msg";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into()).unwrap();
    let lsp_diag: Diagnostic = diag.into();
    // MSBuild line 10 → LSP line 9
    assert_eq!(lsp_diag.range.start.line, 9);
    // MSBuild column 5 → LSP character 4
    assert_eq!(lsp_diag.range.start.character, 4);
}

#[test]
fn lsp_diagnostic_severity_mapping() {
    use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity};

    let err_line = r"file.pas(1): error E1000: err";
    let err_diag: Diagnostic = CompilerLineDiagnostic::from_line(err_line, "dcc".into()).unwrap().into();
    assert_eq!(err_diag.severity, Some(DiagnosticSeverity::ERROR));

    let warn_line = r"file.pas(1): warning W1000: warn";
    let warn_diag: Diagnostic = CompilerLineDiagnostic::from_line(warn_line, "dcc".into()).unwrap().into();
    assert_eq!(warn_diag.severity, Some(DiagnosticSeverity::WARNING));

    let hint_line = r"file.pas(1): hint H1000: hint";
    let hint_diag: Diagnostic = CompilerLineDiagnostic::from_line(hint_line, "dcc".into()).unwrap().into();
    assert_eq!(hint_diag.severity, Some(DiagnosticSeverity::HINT));
}

#[test]
fn lsp_diagnostic_no_column_defaults_to_zero() {
    use tower_lsp::lsp_types::Diagnostic;

    let line = r"file.pas(5): error E2003: msg";
    let diag = CompilerLineDiagnostic::from_line(line, "dcc32".into()).unwrap();
    let lsp_diag: Diagnostic = diag.into();
    // No column → default 1, minus 1 → 0
    assert_eq!(lsp_diag.range.start.character, 0);
}

#[test]
fn lsp_diagnostic_source_is_compiler_name() {
    use tower_lsp::lsp_types::Diagnostic;

    let line = r"file.pas(1): error E1000: msg";
    let diag = CompilerLineDiagnostic::from_line(line, "my-compiler".into()).unwrap();
    let lsp_diag: Diagnostic = diag.into();
    assert_eq!(lsp_diag.source, Some("my-compiler".to_string()));
}
