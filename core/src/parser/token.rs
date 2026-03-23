//! Delphi/Object Pascal lexer token definitions using `logos`.
//!
//! ## Design notes
//!
//! * **Case-insensitive** — Delphi identifiers and reserved words are not
//!   case-sensitive. Every keyword variant uses a `logos` regex that is
//!   case-folded (`(?i:...)`), so the lexer accepts `BEGIN`, `begin`,
//!   `Begin`, etc. as the same token.
//!
//! * **Compiler directives are first-class tokens** — `{$IFDEF}`, `{$IF}`,
//!   `{$ENDIF}`, etc. are captured as dedicated `Token` variants (not
//!   discarded as comments) so the preprocessor can consume them directly.
//!
//! * **Comments are preserved** — both `{ … }`, `(* … *)` block comments and
//!   `// …` line comments are emitted as `Token::Comment` / `Token::LineComment`
//!   so a future lossless CST layer can round-trip source text.
//!
//! * **Whitespace and newlines** are each emitted as a single token for the
//!   same reason.
//!
//! ## Coverage
//!
//! ### Reserved words (cannot be used as identifiers)
//! `and`, `array`, `as`, `asm`, `begin`, `case`, `class`, `const`,
//! `constructor`, `destructor`, `dispinterface`, `div`, `do`, `downto`,
//! `else`, `end`, `except`, `exports`, `file`, `finalization`, `finally`,
//! `for`, `function`, `goto`, `if`, `implementation`, `in`, `inherited`,
//! `initialization`, `inline`, `interface`, `is`, `label`, `library`,
//! `mod`, `nil`, `not`, `object`, `of`, `on`, `operator`, `or`, `out`,
//! `packed`, `procedure`, `program`, `property`, `raise`, `record`,
//! `repeat`, `resourcestring`, `set`, `shl`, `shr`, `string`, `then`,
//! `threadvar`, `to`, `try`, `type`, `unit`, `until`, `uses`, `var`,
//! `while`, `with`, `xor`
//!
//! ### Directive keywords (context-sensitive, usable as identifiers)
//! `absolute`, `abstract`, `assembler`, `at`, `automated`, `cdecl`,
//! `contains`, `default`, `delayed`, `deprecated`, `dispid`, `dynamic`,
//! `experimental`, `export`, `external`, `far`, `final`, `forward`,
//! `helper`, `implements`, `index`, `library` (as dir), `local`,
//! `message`, `name`, `near`, `nodefault`, `operator` (as dir),
//! `overload`, `override`, `package`, `pascal`, `platform`, `private`,
//! `protected`, `public`, `published`, `read`, `readonly`, `reintroduce`,
//! `requires`, `resident`, `safecall`, `sealed`, `static`, `stdcall`,
//! `stored`, `strict`, `unsafe`, `varargs`, `virtual`, `winapi`, `write`,
//! `writeonly`
//!
//! ### Compiler directives
//! All `{$…}` and `(*$…*)` forms. See [`CompilerDirectiveKind`].

use logos::Logos;

// ---------------------------------------------------------------------------
// Span helpers
// ---------------------------------------------------------------------------

/// A byte-offset range within a source file.
///
/// `logos` hands us `Range<usize>` slices; we newtype-wrap them so higher
/// layers can store spans without depending on `logos` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    #[inline]
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    #[inline]
    pub fn len(self) -> usize {
        (self.end - self.start) as usize
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.start == self.end
    }
}

impl From<std::ops::Range<usize>> for Span {
    fn from(r: std::ops::Range<usize>) -> Self {
        Self::new(r.start, r.end)
    }
}

// ---------------------------------------------------------------------------
// Token
// ---------------------------------------------------------------------------

/// A single lexical token in a Delphi / Object Pascal source file.
///
/// Produced by the `logos`-generated lexer.  Every variant that matches a
/// fixed keyword uses a case-insensitive regex (`(?i:…)`).
///
/// The `#[logos(skip …)]` attribute is intentionally **not** used — whitespace
/// and comments are emitted as tokens so a lossless CST layer (or the
/// preprocessor) can round-trip source text and preserve trivia.
#[derive(Logos, Debug, Clone, PartialEq)]
#[logos(error = LexError)]
// Global extras: nothing — we keep the lexer stateless.
pub enum Token<'src> {
    // -----------------------------------------------------------------------
    // Trivia
    // -----------------------------------------------------------------------

    /// One or more ASCII space / tab characters (no newline).
    #[regex(r"[ \t]+")]
    Whitespace,

    /// A newline sequence: `\n`, `\r\n`, or `\r`.
    #[regex(r"\r\n|\r|\n")]
    Newline,

    /// A block comment that is NOT a compiler directive: `{ … }`.
    ///
    /// Carries the full raw source slice (e.g. `{ comment }`) as a
    /// zero-copy borrow — no heap allocation.
    /// Compiler directives are `{$…}` and matched before this rule.
    #[regex(r"\{[^$][^}]*\}", lex_comment_slice, priority = 1)]
    #[regex(r"\{\}", lex_comment_slice, priority = 1)]   // empty brace comment
    BlockComment(&'src str),

    /// A block comment that is NOT a compiler directive: `(* … *)`.
    ///
    /// Carries the full raw source slice as a zero-copy borrow.
    /// The regex uses `([^*]|\*[^)])*` to avoid non-greedy quantifiers
    /// (which logos does not support).
    #[regex(r"\(\*[^$]([^*]|\*[^)])*\*\)", lex_comment_slice, priority = 1)]
    #[regex(r"\(\*\*\)", lex_comment_slice, priority = 1)] // empty paren-star comment
    BlockCommentParen(&'src str),

    /// A line comment: `// …` (everything to end of line).
    ///
    /// Carries the full raw source slice (e.g. `// my comment`) as a
    /// zero-copy borrow.
    #[regex(r"//[^\r\n]*", lex_comment_slice, allow_greedy = true)]
    LineComment(&'src str),

    // -----------------------------------------------------------------------
    // Compiler directives  — must be matched BEFORE generic comments
    // -----------------------------------------------------------------------

    /// A compiler directive in brace form: `{$KEYWORD …}`.
    ///
    /// The variant carries the raw inner text (everything after `{$`
    /// up to but not including `}`).  Higher-level parsing of the directive
    /// kind and arguments is done by the preprocessor.
    #[regex(r"\{\$[^}]*\}", lex_directive_brace)]
    DirectiveBrace(&'src str),

    // /// A compiler directive in paren-star form: `(*$KEYWORD …*)`.
    // ///
    // /// Uses `([^*]|\*[^)])*` to avoid non-greedy quantifiers.
    // #[regex(r"\(\*\$([^*]|\*[^)])*\*\)", lex_directive_paren)]
    // DirectiveParen(Box<str>),

    // -----------------------------------------------------------------------
    // Literals
    // -----------------------------------------------------------------------

    /// An integer literal in decimal, hexadecimal (`$FF`), or binary (`%1010`).
    #[regex(r"[0-9]+", priority = 2)]
    #[regex(r"\$[0-9A-Fa-f]+", priority = 2)]
    #[regex(r"%[01]+", priority = 2)]
    IntLiteral,

    /// A floating-point literal: `3.14`, `1.5e-3`, `1.5E+10`.
    #[regex(r"[0-9]+\.[0-9]+([eE][+\-]?[0-9]+)?", priority = 3)]
    #[regex(r"[0-9]+[eE][+\-]?[0-9]+", priority = 3)]
    FloatLiteral,

    /// A quoted string literal: `'hello'` (doubled single-quote for escape).
    #[regex(r"'([^']|'')*'")]
    StringLiteral,

    /// A character literal: `#65`, `#$41`.
    #[regex(r"#[0-9]+")]
    #[regex(r"#\$[0-9A-Fa-f]+")]
    CharLiteral,

    // -----------------------------------------------------------------------
    // Reserved words
    // All variants use `(?i:…)` for case-insensitivity.
    // -----------------------------------------------------------------------

    /// `and`
    #[token("and", ignore(case))]
    And,

    /// `array`
    #[token("array", ignore(case))]
    Array,

    /// `as`
    #[token("as", ignore(case))]
    As,

    /// `asm`
    #[token("asm", ignore(case))]
    Asm,

    /// `begin`
    #[token("begin", ignore(case))]
    Begin,

    /// `case`
    #[token("case", ignore(case))]
    Case,

    /// `class`
    #[token("class", ignore(case))]
    Class,

    /// `const`
    #[token("const", ignore(case))]
    Const,

    /// `constructor`
    #[token("constructor", ignore(case))]
    Constructor,

    /// `destructor`
    #[token("destructor", ignore(case))]
    Destructor,

    /// `dispinterface`
    #[token("dispinterface", ignore(case))]
    DispInterface,

    /// `div`
    #[token("div", ignore(case))]
    Div,

    /// `do`
    #[token("do", ignore(case))]
    Do,

    /// `downto`
    #[token("downto", ignore(case))]
    DownTo,

    /// `else`
    #[token("else", ignore(case))]
    Else,

    /// `end`
    #[token("end", ignore(case))]
    End,

    /// `except`
    #[token("except", ignore(case))]
    Except,

    /// `exports`
    #[token("exports", ignore(case))]
    Exports,

    /// `file`
    #[token("file", ignore(case))]
    File,

    /// `finalization`
    #[token("finalization", ignore(case))]
    Finalization,

    /// `finally`
    #[token("finally", ignore(case))]
    Finally,

    /// `for`
    #[token("for", ignore(case))]
    For,

    /// `function`
    #[token("function", ignore(case))]
    Function,

    /// `goto`
    #[token("goto", ignore(case))]
    Goto,

    /// `if`
    #[token("if", ignore(case))]
    If,

    /// `implementation`
    #[token("implementation", ignore(case))]
    Implementation,

    /// `in`
    #[token("in", ignore(case))]
    In,

    /// `inherited`
    #[token("inherited", ignore(case))]
    Inherited,

    /// `initialization`
    #[token("initialization", ignore(case))]
    Initialization,

    /// `inline`
    #[token("inline", ignore(case))]
    Inline,

    /// `interface`
    #[token("interface", ignore(case))]
    Interface,

    /// `is`
    #[token("is", ignore(case))]
    Is,

    /// `label`
    #[token("label", ignore(case))]
    Label,

    /// `library`
    #[token("library", ignore(case))]
    Library,

    /// `mod`
    #[token("mod", ignore(case))]
    Mod,

    /// `nil`
    #[token("nil", ignore(case))]
    Nil,

    /// `not`
    #[token("not", ignore(case))]
    Not,

    /// `object`
    #[token("object", ignore(case))]
    Object,

    /// `of`
    #[token("of", ignore(case))]
    Of,

    /// `on`
    #[token("on", ignore(case))]
    On,

    /// `operator`
    #[token("operator", ignore(case))]
    Operator,

    /// `or`
    #[token("or", ignore(case))]
    Or,

    /// `out`
    #[token("out", ignore(case))]
    Out,

    /// `packed`
    #[token("packed", ignore(case))]
    Packed,

    /// `procedure`
    #[token("procedure", ignore(case))]
    Procedure,

    /// `program`
    #[token("program", ignore(case))]
    Program,

    /// `property`
    #[token("property", ignore(case))]
    Property,

    /// `raise`
    #[token("raise", ignore(case))]
    Raise,

    /// `record`
    #[token("record", ignore(case))]
    Record,

    /// `repeat`
    #[token("repeat", ignore(case))]
    Repeat,

    /// `resourcestring`
    #[token("resourcestring", ignore(case))]
    ResourceString,

    /// `set`
    #[token("set", ignore(case))]
    Set,

    /// `shl`
    #[token("shl", ignore(case))]
    Shl,

    /// `shr`
    #[token("shr", ignore(case))]
    Shr,

    /// `string`
    #[token("string", ignore(case))]
    String,

    /// `then`
    #[token("then", ignore(case))]
    Then,

    /// `threadvar`
    #[token("threadvar", ignore(case))]
    ThreadVar,

    /// `to`
    #[token("to", ignore(case))]
    To,

    /// `try`
    #[token("try", ignore(case))]
    Try,

    /// `type`
    #[token("type", ignore(case))]
    Type,

    /// `unit`
    #[token("unit", ignore(case))]
    Unit,

    /// `until`
    #[token("until", ignore(case))]
    Until,

    /// `uses`
    #[token("uses", ignore(case))]
    Uses,

    /// `var`
    #[token("var", ignore(case))]
    Var,

    /// `while`
    #[token("while", ignore(case))]
    While,

    /// `with`
    #[token("with", ignore(case))]
    With,

    /// `xor`
    #[token("xor", ignore(case))]
    Xor,

    // -----------------------------------------------------------------------
    // Directive / modifier keywords  (context-sensitive, valid as identifiers)
    // -----------------------------------------------------------------------

    /// `absolute`
    #[token("absolute", ignore(case))]
    Absolute,

    /// `abstract`
    #[token("abstract", ignore(case))]
    Abstract,

    /// `assembler`
    #[token("assembler", ignore(case))]
    Assembler,

    /// `at`  (used in `raise … at …` and `on E: Exception at …`)
    #[token("at", ignore(case))]
    At,

    /// `automated`  (legacy COM automation section)
    #[token("automated", ignore(case))]
    Automated,

    /// `cdecl`
    #[token("cdecl", ignore(case))]
    CDecl,

    /// `contains`  (package source file)
    #[token("contains", ignore(case))]
    Contains,

    /// `default`
    #[token("default", ignore(case))]
    Default,

    /// `delayed`  (delayed external linking)
    #[token("delayed", ignore(case))]
    Delayed,

    /// `deprecated`
    #[token("deprecated", ignore(case))]
    Deprecated,

    /// `dispid`
    #[token("dispid", ignore(case))]
    DispId,

    /// `dynamic`
    #[token("dynamic", ignore(case))]
    Dynamic,

    /// `experimental`
    #[token("experimental", ignore(case))]
    Experimental,

    /// `export`
    #[token("export", ignore(case))]
    Export,

    /// `external`
    #[token("external", ignore(case))]
    External,

    /// `far`  (16-bit legacy, still parsed)
    #[token("far", ignore(case))]
    Far,

    /// `final`
    #[token("final", ignore(case))]
    Final,

    /// `forward`
    #[token("forward", ignore(case))]
    Forward,

    /// `helper`
    #[token("helper", ignore(case))]
    Helper,

    /// `implements`  (property directive)
    #[token("implements", ignore(case))]
    Implements,

    /// `index`  (property / dispid index)
    #[token("index", ignore(case))]
    Index,

    /// `local`  (platform hint)
    #[token("local", ignore(case))]
    Local,

    /// `message`  (Windows message handler directive)
    #[token("message", ignore(case))]
    Message,

    /// `name`  (external name linking)
    #[token("name", ignore(case))]
    Name,

    /// `near`  (16-bit legacy)
    #[token("near", ignore(case))]
    Near,

    /// `nodefault`
    #[token("nodefault", ignore(case))]
    NoDefault,

    /// `overload`
    #[token("overload", ignore(case))]
    Overload,

    /// `override`
    #[token("override", ignore(case))]
    Override,

    /// `package`  (package source file header)
    #[token("package", ignore(case))]
    Package,

    /// `pascal`  (calling convention)
    #[token("pascal", ignore(case))]
    Pascal,

    /// `platform`  (platform hint)
    #[token("platform", ignore(case))]
    Platform,

    /// `private`
    #[token("private", ignore(case))]
    Private,

    /// `protected`
    #[token("protected", ignore(case))]
    Protected,

    /// `public`
    #[token("public", ignore(case))]
    Public,

    /// `published`
    #[token("published", ignore(case))]
    Published,

    /// `read`  (property accessor)
    #[token("read", ignore(case))]
    Read,

    /// `readonly`
    #[token("readonly", ignore(case))]
    ReadOnly,

    /// `reference`  (used in `reference to` anonymous method types)
    #[token("reference", ignore(case))]
    Reference,

    /// `reintroduce`
    #[token("reintroduce", ignore(case))]
    Reintroduce,

    /// `requires`  (package source file)
    #[token("requires", ignore(case))]
    Requires,

    /// `resident`  (legacy)
    #[token("resident", ignore(case))]
    Resident,

    /// `safecall`
    #[token("safecall", ignore(case))]
    SafeCall,

    /// `sealed`
    #[token("sealed", ignore(case))]
    Sealed,

    /// `static`
    #[token("static", ignore(case))]
    Static,

    /// `stdcall`
    #[token("stdcall", ignore(case))]
    StdCall,

    /// `stored`  (property directive)
    #[token("stored", ignore(case))]
    Stored,

    /// `strict`  (used in `strict private` / `strict protected`)
    #[token("strict", ignore(case))]
    Strict,

    /// `unsafe`
    #[token("unsafe", ignore(case))]
    Unsafe,

    /// `varargs`
    #[token("varargs", ignore(case))]
    VarArgs,

    /// `virtual`
    #[token("virtual", ignore(case))]
    Virtual,

    /// `winapi`
    #[token("winapi", ignore(case))]
    WinApi,

    /// `write`  (property accessor)
    #[token("write", ignore(case))]
    Write,

    /// `writeonly`
    #[token("writeonly", ignore(case))]
    WriteOnly,

    // -----------------------------------------------------------------------
    // Predefined type identifiers
    // These appear in the language spec as type names but are NOT reserved
    // words — they CAN be shadowed by user declarations.  We emit them as
    // dedicated tokens so the parser can handle them without ambiguity in
    // common cases, and fall back to `Ident` when they are shadowed.
    // -----------------------------------------------------------------------

    /// `boolean`
    #[token("boolean", ignore(case))]
    Boolean,

    /// `byte`
    #[token("byte", ignore(case))]
    Byte,

    /// `bytebool`
    #[token("bytebool", ignore(case))]
    ByteBool,

    /// `cardinal`
    #[token("cardinal", ignore(case))]
    Cardinal,

    /// `char`
    #[token("char", ignore(case))]
    Char,

    /// `comp`  (80-bit BCD integer, legacy)
    #[token("comp", ignore(case))]
    Comp,

    /// `currency`
    #[token("currency", ignore(case))]
    Currency,

    /// `double`
    #[token("double", ignore(case))]
    Double,

    /// `extended`
    #[token("extended", ignore(case))]
    Extended,

    /// `int8`
    #[token("int8", ignore(case))]
    Int8,

    /// `int16`
    #[token("int16", ignore(case))]
    Int16,

    /// `int32`
    #[token("int32", ignore(case))]
    Int32,

    /// `int64`
    #[token("int64", ignore(case))]
    Int64,

    /// `integer`
    #[token("integer", ignore(case))]
    Integer,

    /// `longbool`
    #[token("longbool", ignore(case))]
    LongBool,

    /// `longint`
    #[token("longint", ignore(case))]
    LongInt,

    /// `longword`
    #[token("longword", ignore(case))]
    LongWord,

    /// `nativeint`
    #[token("nativeint", ignore(case))]
    NativeInt,

    /// `nativeuint`
    #[token("nativeuint", ignore(case))]
    NativeUInt,

    /// `pansichar`
    #[token("pansichar", ignore(case))]
    PAnsiChar,

    /// `pchar`
    #[token("pchar", ignore(case))]
    PChar,

    /// `pointer`
    #[token("pointer", ignore(case))]
    Pointer,

    /// `pwidechar`
    #[token("pwidechar", ignore(case))]
    PWideChar,

    /// `real`
    #[token("real", ignore(case))]
    Real,

    /// `real48`  (legacy 6-byte real)
    #[token("real48", ignore(case))]
    Real48,

    /// `shortint`
    #[token("shortint", ignore(case))]
    ShortInt,

    /// `shortstring`
    #[token("shortstring", ignore(case))]
    ShortString,

    /// `single`
    #[token("single", ignore(case))]
    Single,

    /// `smallint`
    #[token("smallint", ignore(case))]
    SmallInt,

    /// `text`
    #[token("text", ignore(case))]
    Text,

    /// `uint8`
    #[token("uint8", ignore(case))]
    UInt8,

    /// `uint16`
    #[token("uint16", ignore(case))]
    UInt16,

    /// `uint32`
    #[token("uint32", ignore(case))]
    UInt32,

    /// `uint64`
    #[token("uint64", ignore(case))]
    UInt64,

    /// `word`
    #[token("word", ignore(case))]
    Word,

    /// `wordbool`
    #[token("wordbool", ignore(case))]
    WordBool,

    /// `ansichar`
    #[token("ansichar", ignore(case))]
    AnsiChar,

    /// `ansistring`
    #[token("ansistring", ignore(case))]
    AnsiString,

    /// `rawbytestring`
    #[token("rawbytestring", ignore(case))]
    RawByteString,

    /// `unicodestring`
    #[token("unicodestring", ignore(case))]
    UnicodeString,

    /// `utf8string`
    #[token("utf8string", ignore(case))]
    Utf8String,

    /// `widechar`
    #[token("widechar", ignore(case))]
    WideChar,

    /// `widestring`
    #[token("widestring", ignore(case))]
    WideString,

    // -----------------------------------------------------------------------
    // Special predefined identifiers
    // -----------------------------------------------------------------------

    /// `true`
    #[token("true", ignore(case))]
    True,

    /// `false`
    #[token("false", ignore(case))]
    False,

    /// `result`  (implicit function result variable)
    #[token("result", ignore(case))]
    Result,

    /// `self`  (implicit object reference inside methods)
    #[token("self", ignore(case))]
    Self_,

    // -----------------------------------------------------------------------
    // Punctuation and operators
    // -----------------------------------------------------------------------

    // --- Arithmetic operators ---
    /// `+`
    #[token("+")]
    Plus,

    /// `-`
    #[token("-")]
    Minus,

    /// `*`
    #[token("*")]
    Star,

    /// `/`
    #[token("/")]
    Slash,

    // --- Relational operators ---
    /// `=`
    #[token("=")]
    Eq,

    /// `<>`
    #[token("<>")]
    NEq,

    /// `<`
    #[token("<")]
    Lt,

    /// `>`
    #[token(">")]
    Gt,

    /// `<=`
    #[token("<=")]
    LtEq,

    /// `>=`
    #[token(">=")]
    GtEq,

    // --- Assignment and binding ---
    /// `:=`
    #[token(":=")]
    Assign,

    /// `:`
    #[token(":")]
    Colon,

    /// `;`
    #[token(";")]
    Semicolon,

    /// `,`
    #[token(",")]
    Comma,

    /// `.`
    #[token(".")]
    Dot,

    /// `..`  (subrange)
    #[token("..")]
    DotDot,

    /// `^`  (pointer dereference and pointer type former)
    #[token("^")]
    Caret,

    /// `@`  (address-of)
    #[token("@")]
    At_,

    // --- Delimiters ---
    /// `(`
    #[token("(")]
    LParen,

    /// `)`
    #[token(")")]
    RParen,

    /// `[`
    #[token("[")]
    LBracket,

    /// `]`
    #[token("]")]
    RBracket,

    // --- String concatenation via `#` is covered by CharLiteral ---

    // -----------------------------------------------------------------------
    // Identifiers  (lowest priority — matched only if no keyword matched)
    // -----------------------------------------------------------------------

    /// Any user-defined identifier: letter/underscore followed by
    /// letters, digits, underscores.
    ///
    /// Must have **lower priority** than all keyword tokens (logos resolves
    /// longest-match first, then priority; keywords always win over `Ident`
    /// for the same span).
    #[regex(r"[A-Za-z_][A-Za-z0-9_]*", priority = 0)]
    Ident,

    // -----------------------------------------------------------------------
    // Error token
    // -----------------------------------------------------------------------

    /// Produced by the logos error callback for any byte sequence that
    /// doesn't match any other rule.
    Error,
}

// ---------------------------------------------------------------------------
// Directive inner-text extractors
// ---------------------------------------------------------------------------

/// Return the full raw source slice of a comment token as a zero-copy
/// borrow.  Used for `BlockComment`, `BlockCommentParen`, and `LineComment`.
fn lex_comment_slice<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> &'src str {
    lex.slice()
}

/// Extract the inner text from `{$…}`.
///
/// Strips the leading `{$` and trailing `}`, then boxes the slice.
fn lex_directive_brace<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> &'src str {
    let s = lex.slice();
    // s = "{$...}" → inner = everything between {$ and }
    debug_assert!(s.starts_with("{$") && s.ends_with('}'));
    &s[2..s.len() - 1]
}

/// Extract the inner text from `(*$…*)`.
fn lex_directive_paren<'src>(lex: &mut logos::Lexer<'src, Token<'src>>) -> &'src str {
    let s = lex.slice();
    debug_assert!(s.starts_with("(*$") && s.ends_with("*)"));
    &s[3..s.len() - 2]
}

// ---------------------------------------------------------------------------
// Lex error
// ---------------------------------------------------------------------------

/// Error type returned by the logos lexer for unrecognised input.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LexError;

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("unrecognised token")
    }
}

// ---------------------------------------------------------------------------
// CompilerDirectiveKind — structured form of directive inner text
// ---------------------------------------------------------------------------

/// The kind of a `{$…}` compiler directive, parsed from the inner text
/// that [`Token::DirectiveBrace`] / [`Token::DirectiveParen`] carries.
///
/// Only the directives relevant to **conditional compilation, inclusion, and
/// define management** are individually named here; everything else falls
/// into [`CompilerDirectiveKind::Switch`] or [`CompilerDirectiveKind::Other`].
///
/// The preprocessor calls [`CompilerDirectiveKind::from_inner`] on the inner
/// text captured by the lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum CompilerDirectiveKind {
    // --- Conditional compilation ---

    /// `{$IF <expr>}`
    If(Box<str>),

    /// `{$IFDEF <symbol>}`
    IfDef(Box<str>),

    /// `{$IFNDEF <symbol>}`
    IfNDef(Box<str>),

    /// `{$IFOPT <option>}` — e.g. `{$IFOPT C+}`
    IfOpt(Box<str>),

    /// `{$ELSEIF <expr>}`
    ElseIf(Box<str>),

    /// `{$ELSE}`
    Else,

    /// `{$ENDIF}` or `{$IFEND}` (controlled by `{$LEGACYIFEND}`)
    EndIf,

    /// `{$IFEND}`  (synonym for `{$ENDIF}` when `{$LEGACYIFEND ON}`)
    IfEnd,

    // --- Define / undefine ---

    /// `{$DEFINE <symbol>}`
    Define(Box<str>),

    /// `{$UNDEF <symbol>}` / `{$UNDEFINE <symbol>}`
    Undef(Box<str>),

    // --- Include ---

    /// `{$I <filename>}` / `{$INCLUDE <filename>}`
    Include(Box<str>),

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
        keyword: Box<str>,
        value: Box<str>,
    },

    // --- Miscellaneous ---

    /// `{$MESSAGE HINT|WARN|ERROR|FATAL <text>}`
    Message {
        level: MessageLevel,
        text: Box<str>,
    },

    /// `{$REGION <label>}` / `{$ENDREGION}`
    Region(Option<Box<str>>),

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
    Rtti(Box<str>),

    /// `{$TYPEINFO ON|OFF}`
    TypeInfo(bool),

    /// `{$APPTYPE CONSOLE|GUI}`
    AppType(Box<str>),

    /// `{$LIBSUFFIX '…'}`
    LibSuffix(Box<str>),

    /// `{$WEAKPACKAGEUNIT ON|OFF}`
    WeakPackageUnit(bool),

    /// `{$HPPEMIT '…'}`
    HppEmit(Box<str>),

    /// `{$EXTERNALSYM <ident>}`
    ExternalSym(Box<str>),

    /// `{$NOINCLUDE <ident>}`
    NoInclude(Box<str>),

    /// `{$NODEFINE <ident>}`
    NoDefine(Box<str>),

    /// `{$OBJEXPORTALL ON|OFF}`
    ObjExportAll(bool),

    /// Anything else: carry the raw inner text verbatim.
    Other(Box<str>),
}

/// Severity level for `{$MESSAGE …}` directives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageLevel {
    Hint,
    Warn,
    Error,
    Fatal,
}

impl CompilerDirectiveKind {
    /// Parse the *inner text* of a compiler directive (everything after `{$`
    /// and before `}`) into a structured [`CompilerDirectiveKind`].
    ///
    /// Parsing is **case-insensitive**.
    pub fn from_inner(inner: &str) -> Self {
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

// ---------------------------------------------------------------------------
// Private helpers (shared within this module)
// ---------------------------------------------------------------------------

/// Split `text` at the first ASCII-whitespace boundary.
///
/// Returns `(first_word, rest_of_string)`.
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lex(src: &str) -> Vec<Token<'_>> {
        Token::lexer(src)
            .map(|r| r.unwrap_or(Token::Error))
            .collect()
    }

    // -----------------------------------------------------------------------
    // Reserved words — case insensitivity
    // -----------------------------------------------------------------------

    #[test]
    fn reserved_words_case_insensitive() {
        for src in &["begin", "BEGIN", "Begin", "bEgIn"] {
            let tokens = lex(src);
            assert_eq!(tokens, vec![Token::Begin], "failed for `{src}`");
        }
        for src in &["end", "END", "End"] {
            let tokens = lex(src);
            assert_eq!(tokens, vec![Token::End], "failed for `{src}`");
        }
    }

    #[test]
    fn all_reserved_words() {
        let cases: &[(&str, Token)] = &[
            ("and", Token::And),
            ("array", Token::Array),
            ("as", Token::As),
            ("asm", Token::Asm),
            ("begin", Token::Begin),
            ("case", Token::Case),
            ("class", Token::Class),
            ("const", Token::Const),
            ("constructor", Token::Constructor),
            ("destructor", Token::Destructor),
            ("dispinterface", Token::DispInterface),
            ("div", Token::Div),
            ("do", Token::Do),
            ("downto", Token::DownTo),
            ("else", Token::Else),
            ("end", Token::End),
            ("except", Token::Except),
            ("exports", Token::Exports),
            ("file", Token::File),
            ("finalization", Token::Finalization),
            ("finally", Token::Finally),
            ("for", Token::For),
            ("function", Token::Function),
            ("goto", Token::Goto),
            ("if", Token::If),
            ("implementation", Token::Implementation),
            ("in", Token::In),
            ("inherited", Token::Inherited),
            ("initialization", Token::Initialization),
            ("inline", Token::Inline),
            ("interface", Token::Interface),
            ("is", Token::Is),
            ("label", Token::Label),
            ("library", Token::Library),
            ("mod", Token::Mod),
            ("nil", Token::Nil),
            ("not", Token::Not),
            ("object", Token::Object),
            ("of", Token::Of),
            ("on", Token::On),
            ("operator", Token::Operator),
            ("or", Token::Or),
            ("out", Token::Out),
            ("packed", Token::Packed),
            ("procedure", Token::Procedure),
            ("program", Token::Program),
            ("property", Token::Property),
            ("raise", Token::Raise),
            ("record", Token::Record),
            ("repeat", Token::Repeat),
            ("resourcestring", Token::ResourceString),
            ("set", Token::Set),
            ("shl", Token::Shl),
            ("shr", Token::Shr),
            ("string", Token::String),
            ("then", Token::Then),
            ("threadvar", Token::ThreadVar),
            ("to", Token::To),
            ("try", Token::Try),
            ("type", Token::Type),
            ("unit", Token::Unit),
            ("until", Token::Until),
            ("uses", Token::Uses),
            ("var", Token::Var),
            ("while", Token::While),
            ("with", Token::With),
            ("xor", Token::Xor),
        ];
        for (src, expected) in cases {
            let tokens = lex(src);
            assert_eq!(tokens, vec![expected.clone()], "failed for `{src}`");
        }
    }

    // -----------------------------------------------------------------------
    // Directive keywords
    // -----------------------------------------------------------------------

    #[test]
    fn directive_keywords() {
        let cases: &[(&str, Token)] = &[
            ("virtual", Token::Virtual),
            ("override", Token::Override),
            ("overload", Token::Overload),
            ("abstract", Token::Abstract),
            ("stdcall", Token::StdCall),
            ("cdecl", Token::CDecl),
            ("safecall", Token::SafeCall),
            ("external", Token::External),
            ("forward", Token::Forward),
            ("deprecated", Token::Deprecated),
            ("platform", Token::Platform),
            ("strict", Token::Strict),
            ("sealed", Token::Sealed),
            ("final", Token::Final),
            ("helper", Token::Helper),
            ("reference", Token::Reference),
        ];
        for (src, expected) in cases {
            let tokens = lex(src);
            assert_eq!(tokens, vec![expected.clone()], "failed for `{src}`");
        }
    }

    // -----------------------------------------------------------------------
    // Identifiers vs keywords
    // -----------------------------------------------------------------------

    #[test]
    fn identifier_not_confused_with_keyword() {
        let tokens = lex("MyVar");
        assert_eq!(tokens, vec![Token::Ident]);

        let tokens = lex("begin2");
        assert_eq!(tokens, vec![Token::Ident]);

        let tokens = lex("_result");
        assert_eq!(tokens, vec![Token::Ident]);
    }

    // -----------------------------------------------------------------------
    // Operators and punctuation
    // -----------------------------------------------------------------------

    #[test]
    fn operators() {
        let tokens = lex(":=");
        assert_eq!(tokens, vec![Token::Assign]);

        let tokens = lex("<>");
        assert_eq!(tokens, vec![Token::NEq]);

        let tokens = lex("..");
        assert_eq!(tokens, vec![Token::DotDot]);

        let tokens = lex("<=");
        assert_eq!(tokens, vec![Token::LtEq]);

        let tokens = lex(">=");
        assert_eq!(tokens, vec![Token::GtEq]);
    }

    // -----------------------------------------------------------------------
    // Literals
    // -----------------------------------------------------------------------

    #[test]
    fn integer_literals() {
        assert_eq!(lex("42"), vec![Token::IntLiteral]);
        assert_eq!(lex("$FF"), vec![Token::IntLiteral]);
        assert_eq!(lex("%1010"), vec![Token::IntLiteral]);
    }

    #[test]
    fn float_literals() {
        assert_eq!(lex("3.14"), vec![Token::FloatLiteral]);
        assert_eq!(lex("1.5e-3"), vec![Token::FloatLiteral]);
        assert_eq!(lex("1e10"), vec![Token::FloatLiteral]);
    }

    #[test]
    fn string_literals() {
        assert_eq!(lex("'hello'"), vec![Token::StringLiteral]);
        assert_eq!(lex("'it''s'"), vec![Token::StringLiteral]);
    }

    #[test]
    fn char_literals() {
        assert_eq!(lex("#65"), vec![Token::CharLiteral]);
        assert_eq!(lex("#$41"), vec![Token::CharLiteral]);
    }

    // -----------------------------------------------------------------------
    // Comments
    // -----------------------------------------------------------------------

    #[test]
    fn block_comment() {
        assert_eq!(lex("{ this is a comment }"), vec![Token::BlockComment("{ this is a comment }")]);
        assert_eq!(lex("{}"), vec![Token::BlockComment("{}")]);
    }

    #[test]
    fn paren_comment() {
        assert_eq!(
            lex("(* this is a comment *)"),
            vec![Token::BlockCommentParen("(* this is a comment *)")]
        );
    }

    #[test]
    fn line_comment() {
        assert_eq!(lex("// hello"), vec![Token::LineComment("// hello")]);
    }

    // -----------------------------------------------------------------------
    // Compiler directives
    // -----------------------------------------------------------------------

    #[test]
    fn directive_brace_ifdef() {
        let tokens = lex("{$IFDEF DEBUG}");
        assert_eq!(tokens.len(), 1);
        match &tokens[0] {
            Token::DirectiveBrace(inner) => assert_eq!(inner, &"IFDEF DEBUG"),
            other => panic!("unexpected token: {other:?}"),
        }
    }

    #[test]
    fn directive_brace_endif() {
        let tokens = lex("{$ENDIF}");
        match &tokens[0] {
            Token::DirectiveBrace(inner) => assert_eq!(inner, &"ENDIF"),
            other => panic!("{other:?}"),
        }
    }

    // #[test]
    // fn directive_paren() {
    //     let tokens = lex("(*$IFDEF WIN32*)");
    //     match &tokens[0] {
    //         Token::DirectiveParen(inner) => assert_eq!(inner.as_ref(), "IFDEF WIN32"),
    //         other => panic!("{other:?}"),
    //     }
    // }

    #[test]
    fn directive_not_confused_with_comment() {
        // A plain brace comment must NOT produce a DirectiveBrace
        assert!(matches!(lex("{ not a directive }")[0], Token::BlockComment(_)));
        // A compiler directive must NOT produce a BlockComment
        assert!(matches!(lex("{$DEFINE FOO}")[0], Token::DirectiveBrace(_)));
    }

    // -----------------------------------------------------------------------
    // CompilerDirectiveKind parsing
    // -----------------------------------------------------------------------

    #[test]
    fn parse_ifdef_kind() {
        let k = CompilerDirectiveKind::from_inner("IFDEF DEBUG");
        assert_eq!(k, CompilerDirectiveKind::IfDef("DEBUG".into()));
    }

    #[test]
    fn parse_define_kind() {
        let k = CompilerDirectiveKind::from_inner("DEFINE MY_SYMBOL");
        assert_eq!(k, CompilerDirectiveKind::Define("MY_SYMBOL".into()));
    }

    #[test]
    fn parse_include_short() {
        let k = CompilerDirectiveKind::from_inner("I MyFile.inc");
        assert_eq!(k, CompilerDirectiveKind::Include("MyFile.inc".into()));
    }

    #[test]
    fn parse_include_long() {
        let k = CompilerDirectiveKind::from_inner("INCLUDE MyFile.inc");
        assert_eq!(k, CompilerDirectiveKind::Include("MyFile.inc".into()));
    }

    #[test]
    fn parse_switch_on() {
        let k = CompilerDirectiveKind::from_inner("O+");
        assert_eq!(k, CompilerDirectiveKind::Switch { letter: 'O', on: true });
    }

    #[test]
    fn parse_switch_off() {
        let k = CompilerDirectiveKind::from_inner("R-");
        assert_eq!(k, CompilerDirectiveKind::Switch { letter: 'R', on: false });
    }

    #[test]
    fn parse_hints_on() {
        match CompilerDirectiveKind::from_inner("HINTS ON") {
            CompilerDirectiveKind::Option { keyword, value } => {
                assert_eq!(keyword.as_ref(), "HINTS");
                assert_eq!(value.as_ref(), "ON");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn parse_legacyifend() {
        assert_eq!(
            CompilerDirectiveKind::from_inner("LEGACYIFEND ON"),
            CompilerDirectiveKind::LegacyIfEnd(true)
        );
        assert_eq!(
            CompilerDirectiveKind::from_inner("LEGACYIFEND OFF"),
            CompilerDirectiveKind::LegacyIfEnd(false)
        );
    }

    #[test]
    fn parse_case_insensitive_directive_kind() {
        assert_eq!(
            CompilerDirectiveKind::from_inner("ifdef DEBUG"),
            CompilerDirectiveKind::IfDef("DEBUG".into())
        );
    }

    // -----------------------------------------------------------------------
    // Full snippet
    // -----------------------------------------------------------------------

    #[test]
    fn lex_small_unit() {
        let src = r#"unit Foo;

interface

uses
  System.SysUtils;

type
  TFoo = class
  private
    FBar: Integer;
  public
    constructor Create;
    destructor Destroy; override;
    function GetBar: Integer;
  end;

implementation

{ TFoo }

constructor TFoo.Create;
begin
  inherited;
  FBar := 0;
end;

end."#;

        let tokens = lex(src);
        // spot checks
        assert!(tokens.contains(&Token::Unit));
        assert!(tokens.contains(&Token::Interface));
        assert!(tokens.contains(&Token::Uses));
        assert!(tokens.contains(&Token::Type));
        assert!(tokens.contains(&Token::Class));
        assert!(tokens.contains(&Token::Private));
        assert!(tokens.contains(&Token::Public));
        assert!(tokens.contains(&Token::Constructor));
        assert!(tokens.contains(&Token::Destructor));
        assert!(tokens.contains(&Token::Override));
        assert!(tokens.contains(&Token::Function));
        assert!(tokens.contains(&Token::Implementation));
        assert!(tokens.contains(&Token::Begin));
        assert!(tokens.contains(&Token::End));
        assert!(tokens.contains(&Token::Inherited));
        assert!(tokens.contains(&Token::Integer));
        assert!(tokens.contains(&Token::Assign));
        assert!(tokens.contains(&Token::IntLiteral));
        assert!(tokens.iter().any(|t| matches!(t, Token::BlockComment(_))));
        assert!(tokens.contains(&Token::Dot));
    }
}
