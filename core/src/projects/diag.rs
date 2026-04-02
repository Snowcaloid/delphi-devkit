use chrono::{DateTime, Local};
use tower_lsp::lsp_types::*;
use std::fmt::Display;

// Standard MSBuild / dcc32 format:
// <file>(<line>[,<col>]): (error|warning|hint|fatal) <CODE>: <message> [<project>]
const MSBUILD_OUTPUT_REGEX: &str = r"^(?P<file>.*?)[(](?P<line>\d+)(?:,(?P<column>\d+))?[)]:\s+(?P<kind>.*?)\s+(?P<code>[A-Z]\d+):\s+(?P<message>.*?)(?:\s+\[.*\])?$";

// Delphi 2007 / Borland MSBuild wrapper format:
// <target_file> : (warning|error|hint|fatal) : <source_file>(<line>) <localized_label>: <CODE> <message> [<project>]
const DELPHI2007_MSBUILD_REGEX: &str = r"^.*?\s+:\s+(?:warning|error|hint|fatal)\s+:\s+(?P<file>.*?)[(](?P<line>\d+)(?:,(?P<column>\d+))?[)]\s+\S+\s+(?P<code>[A-Z]\d+)\s+(?P<message>.*?)(?:\s+\[.*\])?$";

// Delphi 2007 simple / duplicate format (indented line, no MSBuild wrapper):
//   <source_file>(<line>) <localized_label>: <CODE> <message>
const DELPHI2007_SIMPLE_REGEX: &str = r"^\s+(?P<file>[A-Za-z]:\\[^(]*?)[(](?P<line>\d+)(?:,(?P<column>\d+))?[)]\s+\S+\s+(?P<code>[A-Z]\d+)\s+(?P<message>.*)$";

#[derive(Debug)]
pub enum DiagnosticKind {
    ERROR,
    WARN,
    HINT,
}

impl Display for DiagnosticKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticKind::ERROR => write!(f, "ERROR"),
            DiagnosticKind::WARN => write!(f, "WARN"),
            DiagnosticKind::HINT => write!(f, "HINT"),
        }
    }
}

pub struct CompilerLineDiagnostic {
    pub time: DateTime<Local>,
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
    pub message: String,
    pub code: String,
    pub kind: DiagnosticKind,
    pub compiler_name: String,
}

impl Display for CompilerLineDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time = self.time.format("%H:%M:%S%.3f");
        let kind = &self.kind;
        let code = &self.code;
        let file = &self.file;
        let line = &self.line;
        let message = &self.message;
        if let Some(column) = self.column {
            write!(
                f,
                "{time}: [{kind}][{code}] {file}:{line}:{column} - {message}",
            )
        } else {
            write!(
                f,
                "{time}: [{kind}][{code}] {file}:{line} - {message}"
            )
        }
    }
}

lazy_static::lazy_static! {
    pub static ref COMPILER_OUTPUT_REGEX: regex::Regex = regex::Regex::new(MSBUILD_OUTPUT_REGEX).unwrap();
    static ref DELPHI2007_MSBUILD_OUTPUT_REGEX: regex::Regex = regex::Regex::new(DELPHI2007_MSBUILD_REGEX).unwrap();
    static ref DELPHI2007_SIMPLE_OUTPUT_REGEX: regex::Regex = regex::Regex::new(DELPHI2007_SIMPLE_REGEX).unwrap();
}

fn build_from_captures(captures: regex::Captures, compiler_name: String) -> Option<CompilerLineDiagnostic> {
    let file = captures.name("file")?.as_str().trim().to_string();
    let line_num = captures.name("line")?.as_str().parse().ok()?;
    let column = captures
        .name("column")
        .and_then(|m| m.as_str().parse().ok());
    let message = captures.name("message")?.as_str().to_string();
    let code = captures.name("code")?.as_str().to_string();
    let kind = if code.starts_with('H') {
        DiagnosticKind::HINT
    } else if code.starts_with('W') {
        DiagnosticKind::WARN
    } else {
        DiagnosticKind::ERROR
    };
    Some(CompilerLineDiagnostic {
        time: Local::now(),
        file,
        line: line_num,
        column,
        message,
        code,
        kind,
        compiler_name,
    })
}

impl CompilerLineDiagnostic {
    /// Try to parse a raw compiler output line into a [`CompilerLineDiagnostic`].
    ///
    /// Attempts three formats in order:
    /// 1. Standard MSBuild / dcc32 format
    /// 2. Delphi 2007 Borland.Delphi.Targets MSBuild wrapper
    /// 3. Delphi 2007 indented simple / duplicate format
    pub fn from_line(line: &str, compiler_name: String) -> Option<Self> {
        if let Some(captures) = COMPILER_OUTPUT_REGEX.captures(line) {
            return build_from_captures(captures, compiler_name);
        }
        if let Some(captures) = DELPHI2007_MSBUILD_OUTPUT_REGEX.captures(line) {
            return build_from_captures(captures, compiler_name);
        }
        if let Some(captures) = DELPHI2007_SIMPLE_OUTPUT_REGEX.captures(line) {
            return build_from_captures(captures, compiler_name);
        }
        None
    }
}

impl Into<Diagnostic> for CompilerLineDiagnostic {
    fn into(self) -> Diagnostic {
        return Diagnostic {
            range: Range {
                start: Position {
                    line: self.line.saturating_sub(1),
                    character: self.column.unwrap_or(1).saturating_sub(1),
                },
                end: Position {
                    line: self.line.saturating_sub(1),
                    character: self.column.unwrap_or(1).saturating_sub(1) + 1,
                },
            },
            severity: match self.kind {
                DiagnosticKind::ERROR => Some(DiagnosticSeverity::ERROR),
                DiagnosticKind::WARN => Some(DiagnosticSeverity::WARNING),
                DiagnosticKind::HINT => Some(DiagnosticSeverity::HINT),
            },
            code: Some(NumberOrString::String(self.code.clone())),
            source: Some(self.compiler_name.to_string()),
            message: self.message.clone(),
            ..Default::default()
        };
    }
}
