use crate::parser::token::Token;
use logos::Logos;

pub struct Lexer<'src> {
    source: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { source }
    }

    pub fn split_directives(&self) -> Vec<PrecToken> {
        let mut lexer = Token::lexer(&self.source);
        let mut result = vec![];
        while let Some(token) = lexer.next() {
            if let Ok(token) = token {
                result.push(PrecToken::from_token(lexer, token));
            } else {
                dbg!("Error: {}", token.unwrap_err());
            }
        }
        result
    }
}

pub enum PrecToken<'src> {
    Token(Token<'src>),
    Directive(LexDirective<'src>)
}

impl<'src> PrecToken<'src> {
    pub fn from_token(lexer: &mut logos::Lexer<'src, Token<'src>>, token: Token<'src>) -> Self {
        match token {
            Token::DirectiveBrace(inner) => Self::Directive(LexDirective::new(inner, lexer)),
            _ => Self::Token(token),
        }
    }
}
pub enum LexDirective<'src> {
    /// `{$IF <expr>}`
    If {
        condition: &'src str,
        code: &'src str,
    },
    /// `{$IFDEF <symbol>}`
    IfDef {
        condition: &'src str,
        code: &'src str,
    },
    /// `{$IFNDEF <symbol>}`
    IfNDef {
        condition: &'src str,
        code: &'src str,
    },
    /// `{$IFOPT <option>}` — e.g. `{$IFOPT C+}`
    IfOpt {
        condition: &'src str,
        code: &'src str,
    },
    /// `{$ELSEIF <expr>}`
    ElseIf {
        condition: &'src str,
        code: &'src str,
    },
    /// `{$ELSE}`
    Else {
        code: &'src str,
    },
    /// `{$ENDIF}` or `{$IFEND}` (controlled by `{$LEGACYIFEND}`)
    EndIf,
    /// `{$IFEND}`  (synonym for `{$ENDIF}` when `{$LEGACYIFEND ON}`)
    IfEnd,
    // --- Define / undefine ---
    /// `{$DEFINE <symbol>}`
    Define(&'src str),
    /// `{$UNDEF <symbol>}` / `{$UNDEFINE <symbol>}`
    Undef(&'src str),
    // --- Include ---
    /// `{$I <filename>}` / `{$INCLUDE <filename>}`
    Include(&'src str),
    // --- Compiler switch  e.g. `{$O+}`, `{$HINTS ON}` ---
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
    /// `{$REGION <label>}` / `{$ENDREGION}`
    Region {
        name: Option<&'src str>,
        code: &'src str,
    },
    /// `{$ENDREGION}`
    EndRegion,
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
    Other(&'src str),
}

impl<'src> LexDirective<'src> {
    /// Parse the *inner text* of a compiler directive (everything after `{$`
    /// and before `}`) into a structured [`LexDirective`].
    ///
    /// Parsing is **case-insensitive**.
    pub fn new(inner: &'src str, lexer: &mut logos::Lexer<'src, Token<'src>>) -> Self {
        let trimmed = inner.trim();
        // Split on the first whitespace to get keyword + rest.
        let (kw, rest) = split_directive(trimmed);
        let kw_up: std::string::String = kw.to_ascii_uppercase();

        // Single-letter switch with no space: "O+", "R-", "A8" etc.
        // Detect this before the keyword match so we don't fall through.
        if kw.len() == 2 && kw.is_ascii() {
            let mut chars = kw.chars();
            let letter = chars.next().unwrap().to_ascii_uppercase();
            let suffix = chars.next().unwrap();
            if rest.is_empty() {
                if suffix == '+' {
                    return Self::Switch { letter, on: true };
                } else if suffix == '-' {
                    return Self::Switch { letter, on: false };
                }
            }
        }

        match kw_up.as_str() {
            "IF" => Self::If(rest.into()),
            "IFDEF" => Self::IfDef(rest.into()),
            "IFNDEF" => Self::IfNDef(rest.into()),
            "IFOPT" => Self::IfOpt(rest.into()),
            "ELSEIF" => Self::ElseIf(rest.into()),
            "ELSE" => Self::Else,
            "ENDIF" => Self::EndIf,
            "IFEND" => Self::IfEnd,

            "DEFINE" => Self::Define(rest.trim().into()),
            "UNDEF" | "UNDEFINE" => Self::Undef(rest.trim().into()),

            "I" | "INCLUDE" => Self::Include(rest.trim().into()),

            "REGION" => {
                let label = rest.trim();
                if label.is_empty() {
                    Self::Region(None)
                } else {
                    Self::Region(Some(label.into()))
                }
            }
            "ENDREGION" => Self::EndRegion,

            "MESSAGE" => {
                let (level_str, text) = split_directive(rest.trim());
                let level = match level_str.to_ascii_uppercase().as_str() {
                    "HINT" => MessageLevel::Hint,
                    "WARN" | "WARNING" => MessageLevel::Warn,
                    "ERROR" => MessageLevel::Error,
                    "FATAL" => MessageLevel::Fatal,
                    _ => MessageLevel::Hint, // fallback
                };
                Self::Message {
                    level,
                    text: text.trim().into(),
                }
            }

            "LEGACYIFEND" => Self::LegacyIfEnd(parse_on_off(rest)),
            "SCOPEDENUMS" => Self::ScopedEnums(parse_on_off(rest)),
            "POINTERMATH" => Self::PointerMath(parse_on_off(rest)),
            "TYPEINFO" => Self::TypeInfo(parse_on_off(rest)),
            "WEAKPACKAGEUNIT" => Self::WeakPackageUnit(parse_on_off(rest)),
            "OBJEXPORTALL" => Self::ObjExportAll(parse_on_off(rest)),

            "MINENUMSIZE" => {
                let n: u8 = rest.trim().parse().unwrap_or(1);
                Self::MinEnumSize(n)
            }
            "ALIGN" | "A" => {
                let n: u8 = rest.trim().parse().unwrap_or(8);
                Self::Align(n)
            }

            "APPTYPE" => Self::AppType(rest.trim().into()),
            "LIBSUFFIX" => Self::LibSuffix(rest.trim().into()),
            "RTTI" => Self::Rtti(rest.trim().into()),
            "HPPEMIT" => Self::HppEmit(rest.trim().into()),
            "EXTERNALSYM" => Self::ExternalSym(rest.trim().into()),
            "NOINCLUDE" => Self::NoInclude(rest.trim().into()),
            "NODEFINE" => Self::NoDefine(rest.trim().into()),

            // Single-letter switch with a space: "O +" or "O -"
            _ if kw.len() == 1 && kw.is_ascii() => {
                let letter = kw.chars().next().unwrap().to_ascii_uppercase();
                let suffix = rest.trim();
                if suffix == "+" {
                    Self::Switch { letter, on: true }
                } else if suffix == "-" {
                    Self::Switch { letter, on: false }
                } else {
                    Self::Other(inner.into())
                }
            }

            // Long-form option with value: "HINTS ON", "WARN SYMBOL_DEPRECATED OFF" …
            _ if !rest.is_empty() => Self::Option {
                keyword: kw_up.as_str().into(),
                value: rest.trim().into(),
            },

            _ => Self::Other(inner.into()),
        }
    }
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