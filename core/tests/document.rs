use ddk_core::utils::Document;
use tower_lsp::lsp_types::{Position, Range};

// ═══════════════════════════════════════════════════════════════════════════════
//  Document::range – single line
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn range_single_line_extraction() {
    let content = "Hello, world!";
    let doc = Document::new(content);
    let range = Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 0, character: 5 },
    };
    assert_eq!(doc.range(range), "Hello");
}

#[test]
fn range_single_line_mid() {
    let content = "Hello, world!";
    let doc = Document::new(content);
    let range = Range {
        start: Position { line: 0, character: 7 },
        end: Position { line: 0, character: 12 },
    };
    assert_eq!(doc.range(range), "world");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Document::range – multi-line
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn range_multi_line() {
    let content = "line one\nline two\nline three";
    let doc = Document::new(content);
    let range = Range {
        start: Position { line: 0, character: 5 },
        end: Position { line: 1, character: 4 },
    };
    assert_eq!(doc.range(range), "one\nline");
}

#[test]
fn range_entire_content() {
    let content = "abc\ndef";
    let doc = Document::new(content);
    let range = Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 1, character: 3 },
    };
    assert_eq!(doc.range(range), "abc\ndef");
}

// ═══════════════════════════════════════════════════════════════════════════════
//  Document::range – edge cases
// ═══════════════════════════════════════════════════════════════════════════════

#[test]
fn range_start_at_zero() {
    let content = "abc\ndef";
    let doc = Document::new(content);
    let range = Range {
        start: Position { line: 0, character: 0 },
        end: Position { line: 0, character: 3 },
    };
    assert_eq!(doc.range(range), "abc");
}

#[test]
fn range_second_line_only() {
    let content = "abc\ndef\nghi";
    let doc = Document::new(content);
    let range = Range {
        start: Position { line: 1, character: 0 },
        end: Position { line: 1, character: 3 },
    };
    assert_eq!(doc.range(range), "def");
}
