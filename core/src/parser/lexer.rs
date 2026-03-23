use crate::parser::{directives::DirectiveState, token::{MessageLevel, Token}};
use logos::Logos;

pub struct Lexer<'src> {
    source: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source }
    }

    /// Lex `source` and return a structured token sequence where compiler
    /// directives are represented as [`PrecToken::Directive`] nodes with
    /// their nested body tokens collected recursively.
    pub fn split_directives(&self) -> Vec<PrecToken<'_>> {
        let mut logos_lexer = Token::lexer(self.source);
        let (tokens, _) = collect_tokens(&mut logos_lexer);
        tokens
    }
}

// ---------------------------------------------------------------------------
// Recursive token collection
// ---------------------------------------------------------------------------

/// Signals why a [`collect_tokens`] call returned early.
enum BlockTerminator {
    /// `{$ENDIF}` / `{$IFEND}` terminated an IF-family block.
    EndIf,
    /// `{$ENDREGION}` terminated a REGION block.
    EndRegion,
    /// The token stream was exhausted.
    Eof,
}

/// Collect [`PrecToken`]s from `lexer` recursively.
///
/// Processing rules:
/// - Plain source tokens become [`PrecToken::Token`].
/// - Simple (non-block) directives become [`PrecToken::Directive`].
/// - Block-opening directives (`IF`, `IFDEF`, `IFNDEF`, `IFOPT`, `REGION`)
///   trigger a recursive call; the collected body is embedded in the directive.
/// - `{$ELSE}` and `{$ELSEIF}` collect their own body recursively, push
///   themselves into the **current** result, then return early — so they appear
///   nested inside the preceding IF/ELSEIF body.
/// - `{$ENDIF}`, `{$IFEND}`, `{$ENDREGION}` return early to signal the
///   parent call that the block is closed.
fn collect_tokens<'src>(
    lexer: &mut logos::Lexer<'src, Token<'src>>,
) -> (Vec<PrecToken<'src>>, BlockTerminator) {
    let mut result: Vec<PrecToken<'src>> = vec![];

    while let Some(tok) = lexer.next() {
        match tok {
            Ok(Token::DirectiveBrace(inner)) => {
                let trimmed = inner.trim();
                let (kw, rest) = split_directive(trimmed);
                let kw_up = kw.to_ascii_uppercase();

                match kw_up.as_str() {
                    // ----------------------------------------------------------
                    // Block-opening: IF family
                    // ----------------------------------------------------------
                    "IF" => {
                        let (body, _) = collect_tokens(lexer);
                        result.push(PrecToken::Directive(LexDirective::If {
                            condition: tokenize_expr(rest.trim()),
                            body,
                        }));
                    }
                    "IFDEF" => {
                        let (body, _) = collect_tokens(lexer);
                        result.push(PrecToken::Directive(LexDirective::IfDef {
                            directive: rest.trim(),
                            state: DirectiveState::Unknown,
                            body,
                        }));
                    }
                    "IFNDEF" => {
                        let (body, _) = collect_tokens(lexer);
                        result.push(PrecToken::Directive(LexDirective::IfNDef {
                            directive: rest.trim(),
                            state: DirectiveState::Unknown,
                            body,
                        }));
                    }
                    "IFOPT" => {
                        let (letter, on) = parse_ifopt(rest.trim());
                        let (body, _) = collect_tokens(lexer);
                        result.push(PrecToken::Directive(LexDirective::IfOpt {
                            letter,
                            on,
                            state: DirectiveState::Unknown,
                            body,
                        }));
                    }

                    // ----------------------------------------------------------
                    // ELSE: collect its own body, push, then terminate current block.
                    // This causes ELSE to appear nested inside the preceding IF body.
                    // ----------------------------------------------------------
                    "ELSE" => {
                        let (body, _) = collect_tokens(lexer);
                        result.push(PrecToken::Directive(LexDirective::Else { body }));
                        return (result, BlockTerminator::EndIf);
                    }

                    // ----------------------------------------------------------
                    // ELSEIF: collect its own body, push, then terminate current block.
                    // This causes ELSEIF to appear nested inside the preceding IF body.
                    // ----------------------------------------------------------
                    "ELSEIF" => {
                        let (body, _) = collect_tokens(lexer);
                        result.push(PrecToken::Directive(LexDirective::ElseIf {
                            condition: tokenize_expr(rest.trim()),
                            state: DirectiveState::Unknown,
                            body,
                        }));
                        return (result, BlockTerminator::EndIf);
                    }

                    // ----------------------------------------------------------
                    // Block-closing: IF terminators
                    // ----------------------------------------------------------
                    "ENDIF" | "IFEND" => return (result, BlockTerminator::EndIf),

                    // ----------------------------------------------------------
                    // Block-opening: REGION
                    // ----------------------------------------------------------
                    "REGION" => {
                        let label = rest.trim();
                        let (body, _) = collect_tokens(lexer);
                        let name = if label.is_empty() { None } else { Some(label) };
                        result.push(PrecToken::Directive(LexDirective::Region { name, body }));
                    }

                    // ----------------------------------------------------------
                    // Block-closing: ENDREGION
                    // ----------------------------------------------------------
                    "ENDREGION" => return (result, BlockTerminator::EndRegion),

                    // ----------------------------------------------------------
                    // Everything else: simple (non-block) directive
                    // ----------------------------------------------------------
                    _ => {
                        result.push(PrecToken::Directive(LexDirective::parse_simple(inner)));
                    }
                }
            }

            Ok(token) => result.push(PrecToken::Token(token)),
            Err(_) => {} // skip unrecognised input
        }
    }

    (result, BlockTerminator::Eof)
}

// ---------------------------------------------------------------------------
// CondToken
// ---------------------------------------------------------------------------

/// A single token from a directive condition expression, paired with the
/// source text it matched.
///
/// Needed because [`Token::Ident`] and other zero-data variants do not carry
/// the matched text; the text is captured separately via [`logos::Lexer::slice`].
pub struct CondToken<'src> {
    pub kind: Token<'src>,
    pub text: &'src str,
}

// ---------------------------------------------------------------------------
// PrecToken
// ---------------------------------------------------------------------------

/// A preprocessed token: either a plain source token or a structured
/// compiler directive (possibly containing nested tokens in its body).
pub enum PrecToken<'src> {
    /// A regular source token (not a compiler directive).
    Token(Token<'src>),
    /// A parsed compiler directive, potentially containing nested [`PrecToken`]s.
    Directive(LexDirective<'src>),
}

// ---------------------------------------------------------------------------
// LexDirective
// ---------------------------------------------------------------------------

/// A structured representation of a `{$…}` compiler directive.
///
/// Block directives (IF-family, REGION) recursively contain all nested
/// source tokens and inner directives in their `body` field.
pub enum LexDirective<'src> {
    // --- Conditional compilation (block directives) ---

    /// `{$IF <expr>} … {$ENDIF}`
    If {
        /// The condition expression, tokenized (excluding whitespace).
        condition: Vec<CondToken<'src>>,
        body: Vec<PrecToken<'src>>,
    },
    /// `{$IFDEF <directive>} … {$ENDIF}`
    IfDef {
        /// The directive name (a single identifier).
        directive: &'src str,
        state: DirectiveState,
        body: Vec<PrecToken<'src>>,
    },
    /// `{$IFNDEF <directive>} … {$ENDIF}`
    IfNDef {
        /// The directive name (a single identifier).
        directive: &'src str,
        state: DirectiveState,
        body: Vec<PrecToken<'src>>,
    },
    /// `{$IFOPT <option>} … {$ENDIF}` — e.g. `{$IFOPT C+}`
    IfOpt {
        /// The compiler-switch letter, upper-cased.
        letter: char,
        /// `true` if the option is `+`, `false` if `-`.
        on: bool,
        state: DirectiveState,
        body: Vec<PrecToken<'src>>,
    },
    /// `{$ELSEIF <expr>} … {$ENDIF}` — nested inside an IF body
    ElseIf {
        /// The condition expression, tokenized (excluding whitespace).
        condition: Vec<CondToken<'src>>,
        state: DirectiveState,
        body: Vec<PrecToken<'src>>,
    },
    /// `{$ELSE} … {$ENDIF}` — nested inside an IF/ELSEIF body
    Else {
        body: Vec<PrecToken<'src>>,
    },
    /// `{$ENDIF}` / `{$IFEND}` — block terminator.
    ///
    /// Normally consumed by the recursive collector; only surfaced here
    /// if encountered outside any IF block (malformed input).
    EndIf,
    /// `{$IFEND}` synonym (used with `{$LEGACYIFEND ON}`).
    IfEnd,

    // --- Define / undefine ---

    /// `{$DEFINE <symbol>}`
    Define(&'src str),
    /// `{$UNDEF <symbol>}` / `{$UNDEFINE <symbol>}`
    Undef(&'src str),

    // --- Include ---

    /// `{$I <filename>}` / `{$INCLUDE <filename>}`
    Include(&'src str),

    // --- Compiler switches ---

    /// A single-character compiler switch with `+`/`-` suffix, e.g. `{$O+}`.
    Switch {
        /// The switch letter, upper-cased.
        letter: char,
        /// `true` for `+`, `false` for `-`.
        on: bool,
    },
    /// A long-form compiler option, e.g. `{$HINTS ON}`, `{$OPTIMIZATION ON}`,
    /// `{$WARN SYMBOL_DEPRECATED OFF}`.
    Option {
        keyword: &'src str,
        value: &'src str,
    },
    /// `{$MESSAGE HINT|WARN|ERROR|FATAL <text>}`
    Message {
        level: MessageLevel,
        text: &'src str,
    },

    // --- Region ---

    /// `{$REGION <label>} … {$ENDREGION}`
    Region {
        name: Option<&'src str>,
        body: Vec<PrecToken<'src>>,
    },
    /// `{$ENDREGION}` — block terminator.
    ///
    /// Normally consumed by the recursive collector.
    EndRegion,

    // --- Misc compiler flags ---

    /// `{$LEGACYIFEND ON|OFF}`
    LegacyIfEnd(bool),
    /// `{$SCOPEDENUMS ON|OFF}`
    ScopedEnums(bool),
    /// `{$POINTERMATH ON|OFF}`
    PointerMath(bool),
    /// `{$MINENUMSIZE 1|2|4}`
    MinEnumSize(u8),
    /// `{$ALIGN 1|2|4|8|16}`
    Align(u8),
    /// `{$RTTI …}` — complex; carry raw text
    Rtti(&'src str),
    /// `{$TYPEINFO ON|OFF}`
    TypeInfo(bool),
    /// `{$APPTYPE CONSOLE|GUI}`
    AppType(&'src str),
    /// `{$LIBSUFFIX '…'}`
    LibSuffix(&'src str),
    /// `{$WEAKPACKAGEUNIT ON|OFF}`
    WeakPackageUnit(bool),
    /// `{$HPPEMIT '…'}`
    HppEmit(&'src str),
    /// `{$EXTERNALSYM <ident>}`
    ExternalSym(&'src str),
    /// `{$NOINCLUDE <ident>}`
    NoInclude(&'src str),
    /// `{$NODEFINE <ident>}`
    NoDefine(&'src str),
    /// `{$OBJEXPORTALL ON|OFF}`
    ObjExportAll(bool),
    /// Any directive not specifically recognised above.
    Other(&'src str),
}

impl<'src> LexDirective<'src> {
    /// Parse a **non-block** directive from its raw inner text (everything
    /// between `{$` and `}`).
    ///
    /// Block directives (IF family, REGION, and their ELSE/ELSEIF/ENDIF/ENDREGION
    /// terminators) are handled by [`collect_tokens`] and are never passed
    /// to this method.
    fn parse_simple(inner: &'src str) -> Self {
        let trimmed = inner.trim();
        let (kw, rest) = split_directive(trimmed);
        let kw_up = kw.to_ascii_uppercase();

        // Single-letter switch with no space: "O+", "R-" etc.
        if kw.len() == 2 && kw.is_ascii() {
            let mut chars = kw.chars();
            let letter = chars.next().unwrap().to_ascii_uppercase();
            let suffix = chars.next().unwrap();
            if rest.is_empty() && (suffix == '+' || suffix == '-') {
                return Self::Switch { letter, on: suffix == '+' };
            }
        }

        match kw_up.as_str() {
            "DEFINE" => Self::Define(rest.trim()),
            "UNDEF" | "UNDEFINE" => Self::Undef(rest.trim()),

            "I" | "INCLUDE" => Self::Include(rest.trim()),

            "MESSAGE" => {
                let (level_str, text) = split_directive(rest.trim());
                let level = match level_str.to_ascii_uppercase().as_str() {
                    "HINT" => MessageLevel::Hint,
                    "WARN" | "WARNING" => MessageLevel::Warn,
                    "ERROR" => MessageLevel::Error,
                    "FATAL" => MessageLevel::Fatal,
                    _ => MessageLevel::Hint,
                };
                Self::Message { level, text: text.trim() }
            }

            "LEGACYIFEND" => Self::LegacyIfEnd(parse_on_off(rest)),
            "SCOPEDENUMS" => Self::ScopedEnums(parse_on_off(rest)),
            "POINTERMATH" => Self::PointerMath(parse_on_off(rest)),
            "TYPEINFO" => Self::TypeInfo(parse_on_off(rest)),
            "WEAKPACKAGEUNIT" => Self::WeakPackageUnit(parse_on_off(rest)),
            "OBJEXPORTALL" => Self::ObjExportAll(parse_on_off(rest)),

            "MINENUMSIZE" => Self::MinEnumSize(rest.trim().parse().unwrap_or(1)),
            "ALIGN" | "A" => Self::Align(rest.trim().parse().unwrap_or(8)),

            "APPTYPE" => Self::AppType(rest.trim()),
            "LIBSUFFIX" => Self::LibSuffix(rest.trim()),
            "RTTI" => Self::Rtti(rest.trim()),
            "HPPEMIT" => Self::HppEmit(rest.trim()),
            "EXTERNALSYM" => Self::ExternalSym(rest.trim()),
            "NOINCLUDE" => Self::NoInclude(rest.trim()),
            "NODEFINE" => Self::NoDefine(rest.trim()),

            // Single-letter switch with a space: "O +" or "O -"
            _ if kw.len() == 1 && kw.is_ascii() => {
                let letter = kw.chars().next().unwrap().to_ascii_uppercase();
                let suffix = rest.trim();
                if suffix == "+" {
                    Self::Switch { letter, on: true }
                } else if suffix == "-" {
                    Self::Switch { letter, on: false }
                } else {
                    Self::Other(inner)
                }
            }

            // Long-form option with value: "HINTS ON", "WARN SYMBOL_DEPRECATED OFF" …
            _ if !rest.is_empty() => Self::Option {
                keyword: kw,
                value: rest.trim(),
            },

            _ => Self::Other(inner),
        }
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Lex a directive condition expression into a sequence of [`CondToken`]s,
/// discarding whitespace and newline trivia.
///
/// The input `expr` must be a slice of the original source so that
/// `logos::Lexer::slice` returns pointers with the correct `'src` lifetime.
fn tokenize_expr<'src>(expr: &'src str) -> Vec<CondToken<'src>> {
    let mut lex = Token::lexer(expr);
    let mut out = vec![];
    while let Some(Ok(kind)) = lex.next() {
        if !matches!(kind, Token::Whitespace | Token::Newline) {
            out.push(CondToken { kind, text: lex.slice() });
        }
    }
    out
}

/// Parse an `{$IFOPT}` operand such as `C+` or `R-` into `(letter, on)`.
///
/// Defaults to `('?', false)` for malformed input.
fn parse_ifopt(s: &str) -> (char, bool) {
    let mut chars = s.chars();
    let letter = chars.next().unwrap_or('?').to_ascii_uppercase();
    let on = chars.next().map_or(false, |c| c == '+');
    (letter, on)
}

/// Split `text` at the first ASCII-whitespace boundary.
///
/// Returns `(first_word, rest_of_string)`.  `rest_of_string` keeps its
/// leading whitespace trimmed by the caller if desired.
fn split_directive(text: &str) -> (&str, &str) {
    match text.find(|c: char| c.is_ascii_whitespace()) {
        Some(pos) => (&text[..pos], &text[pos + 1..]),
        _ => (text, ""),
    }
}

/// Parse `ON` / `OFF` string to `bool`.  Defaults to `true`.
fn parse_on_off(s: &str) -> bool {
    !s.trim().eq_ignore_ascii_case("OFF")
}