use std::sync::RwLock;
use encoding_rs::Encoding;

/// The special value "oem" means: auto-detect using the Windows OEM codepage.
static COMPILER_ENCODING: RwLock<&str> = RwLock::new("oem");

/// Decode raw bytes to a UTF-8 `String` using the given encoding label.
///
/// Recognised special values: `"oem"` (Windows system OEM codepage auto-detected
/// at runtime), `"utf-8"`, `"utf-32le"`, `"utf-32be"`.
/// Every other value is passed to `encoding_rs` by label.
pub fn decode_bytes(bytes: &[u8], label: &str) -> String {
    let lower = label.to_lowercase();
    match lower.as_str() {
        "oem" => decode_oem(bytes),
        "utf-8" | "utf8" => String::from_utf8_lossy(bytes).to_string(),
        "utf-32le" => decode_utf32(bytes, true),
        "utf-32be" => decode_utf32(bytes, false),
        _ => match Encoding::for_label(lower.as_bytes()) {
            Some(enc) => {
                let (decoded, _, _) = enc.decode(bytes);
                decoded.into_owned()
            }
            None => String::from_utf8_lossy(bytes).to_string(),
        },
    }
}

/// Encode a UTF-8 `str` to raw bytes using the given encoding label.
///
/// `"oem"` maps to the Windows system OEM codepage (via `WideCharToMultiByte`
/// on Windows, falls back to UTF-8 on other platforms).
/// Every other value is passed to `encoding_rs` by label.
pub fn encode_string(s: &str, label: &str) -> Vec<u8> {
    let lower = label.to_lowercase();
    match lower.as_str() {
        "oem" => encode_oem(s),
        "utf-8" | "utf8" => s.as_bytes().to_vec(),
        "utf-32le" => {
            let mut out = Vec::with_capacity(s.chars().count() * 4);
            for c in s.chars() {
                out.extend_from_slice(&(c as u32).to_le_bytes());
            }
            out
        }
        "utf-32be" => {
            let mut out = Vec::with_capacity(s.chars().count() * 4);
            for c in s.chars() {
                out.extend_from_slice(&(c as u32).to_be_bytes());
            }
            out
        }
        _ => match Encoding::for_label(lower.as_bytes()) {
            Some(enc) => {
                let (bytes, _, _) = enc.encode(s);
                bytes.into_owned()
            }
            None => s.as_bytes().to_vec(),
        },
    }
}

/// Encode a string to the system OEM codepage using `WideCharToMultiByte`.
///
/// On non-Windows platforms, falls back to returning the raw UTF-8 bytes.
#[cfg(windows)]
fn encode_oem(s: &str) -> Vec<u8> {
    // CP_OEMCP = 1 — the current system OEM codepage.
    const CP_OEMCP: u32 = 1;
    unsafe extern "system" {
        fn WideCharToMultiByte(
            code_page: u32,
            dw_flags: u32,
            lp_wide_char_str: *const u16,
            cch_wide_char: i32,
            lp_multi_byte_str: *mut u8,
            cb_multi_byte: i32,
            lp_default_char: *const u8,
            lp_used_default_char: *mut i32,
        ) -> i32;
    }
    let wide: Vec<u16> = s.encode_utf16().collect();
    let size = unsafe {
        WideCharToMultiByte(
            CP_OEMCP, 0,
            wide.as_ptr(), wide.len() as i32,
            std::ptr::null_mut(), 0,
            std::ptr::null(), std::ptr::null_mut(),
        )
    };
    if size <= 0 {
        return s.as_bytes().to_vec();
    }
    let mut out = vec![0u8; size as usize];
    unsafe {
        WideCharToMultiByte(
            CP_OEMCP, 0,
            wide.as_ptr(), wide.len() as i32,
            out.as_mut_ptr(), size,
            std::ptr::null(), std::ptr::null_mut(),
        );
    }
    out
}

#[cfg(not(windows))]
fn encode_oem(s: &str) -> Vec<u8> {
    s.as_bytes().to_vec()
}

/// Set the compiler output encoding label (e.g. "windows-1252", "utf-8", "oem").
///
/// "oem" (the default) auto-detects the system's OEM codepage at decode time,
/// which is the encoding that console programs like MSBuild use for stdout.
pub fn set_encoding(label: &str) {
    let lower = label.to_lowercase();
    if lower == "oem"
        || lower == "utf-32le"
        || lower == "utf-32be"
        || Encoding::for_label(lower.as_bytes()).is_some()
    {
        *COMPILER_ENCODING.write().unwrap() = Box::leak(lower.into_boxed_str());
    }
}

/// Decode raw bytes from compiler output using the configured encoding.
pub fn decode_line(bytes: &[u8]) -> String {
    let label = *COMPILER_ENCODING.read().unwrap();
    match label {
        "oem" => decode_oem(bytes),
        "utf-8" => String::from_utf8_lossy(bytes).to_string(),
        "utf-32le" => decode_utf32(bytes, true),
        "utf-32be" => decode_utf32(bytes, false),
        _ => match Encoding::for_label(label.as_bytes()) {
            Some(encoding) => {
                let (decoded, _, _) = encoding.decode(bytes);
                decoded.into_owned()
            }
            None => String::from_utf8_lossy(bytes).to_string(),
        },
    }
}

// ---------------------------------------------------------------------------
// OEM codepage auto-detection (Windows)
// ---------------------------------------------------------------------------

/// Detect the system OEM codepage and decode accordingly.
fn decode_oem(bytes: &[u8]) -> String {
    let cp = oem_codepage();
    match cp {
        65001 => String::from_utf8_lossy(bytes).to_string(),
        850 => decode_single_byte(bytes, &CP850_HIGH),
        437 => decode_single_byte(bytes, &CP437_HIGH),
        _ => {
            // Try mapping the codepage number to an encoding_rs label.
            if let Some(enc) = codepage_to_encoding(cp) {
                let (decoded, _, _) = enc.decode(bytes);
                decoded.into_owned()
            } else {
                // Last resort: lossy UTF-8
                String::from_utf8_lossy(bytes).to_string()
            }
        }
    }
}

/// Get the active OEM codepage. On non-Windows, falls back to UTF-8 (65001).
#[cfg(windows)]
fn oem_codepage() -> u32 {
    // kernel32!GetOEMCP – always available, no additional crate needed.
    unsafe extern "system" {
        fn GetOEMCP() -> u32;
    }
    unsafe { GetOEMCP() }
}

#[cfg(not(windows))]
fn oem_codepage() -> u32 {
    65001 // UTF-8
}

/// Map a Windows codepage number to an `encoding_rs` encoding (where possible).
fn codepage_to_encoding(cp: u32) -> Option<&'static Encoding> {
    let label: &[u8] = match cp {
        866 => b"ibm866",
        874 => b"windows-874",
        1250 => b"windows-1250",
        1251 => b"windows-1251",
        1252 => b"windows-1252",
        1253 => b"windows-1253",
        1254 => b"windows-1254",
        1255 => b"windows-1255",
        1256 => b"windows-1256",
        1257 => b"windows-1257",
        1258 => b"windows-1258",
        28591 => b"iso-8859-1",
        28592 => b"iso-8859-2",
        28593 => b"iso-8859-3",
        28594 => b"iso-8859-4",
        28595 => b"iso-8859-5",
        28596 => b"iso-8859-6",
        28597 => b"iso-8859-7",
        28598 => b"iso-8859-8",
        28599 => b"iso-8859-9",
        28600 => b"iso-8859-10",
        28603 => b"iso-8859-13",
        28604 => b"iso-8859-14",
        28605 => b"iso-8859-15",
        20866 => b"koi8-r",
        21866 => b"koi8-u",
        932 => b"shift_jis",
        936 => b"gbk",
        949 => b"euc-kr",
        950 => b"big5",
        51932 => b"euc-jp",
        _ => return None,
    };
    Encoding::for_label(label)
}

// ---------------------------------------------------------------------------
// Single-byte OEM codepage decode (CP850, CP437)
// ---------------------------------------------------------------------------

/// Decode bytes using a 128-entry high-half table (indices 0x80..0xFF).
/// Bytes 0x00..0x7F are ASCII-identical for all OEM codepages.
fn decode_single_byte(bytes: &[u8], high_table: &[char; 128]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        if b < 0x80 {
            out.push(b as char);
        } else {
            out.push(high_table[(b - 0x80) as usize]);
        }
    }
    out
}

/// CP850 high-half (0x80..0xFF) – DOS Latin-1 (Western European).
/// This is the default OEM codepage on German, French, Spanish, etc. Windows.
#[rustfmt::skip]
static CP850_HIGH: [char; 128] = [
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©', '╣', '║', '╗', '╝', '¢', '¥', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦', '╠', '═', '╬', '¤',
    // 0xD0
    'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì', '▀',
    // 0xE0
    'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´',
    // 0xF0
    '\u{00AD}', '±', '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{00A0}',
];

/// CP437 high-half (0x80..0xFF) – the original IBM PC codepage (US English).
#[rustfmt::skip]
static CP437_HIGH: [char; 128] = [
    // 0x80
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
    // 0x90
    'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
    // 0xA0
    'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
    // 0xB0
    '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
    // 0xC0
    '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
    // 0xD0
    '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
    // 0xE0
    'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
    // 0xF0
    '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{00A0}',
];

// ---------------------------------------------------------------------------
// UTF-32 decode
// ---------------------------------------------------------------------------

fn decode_utf32(bytes: &[u8], little_endian: bool) -> String {
    let mut chars = Vec::with_capacity(bytes.len() / 4);
    for chunk in bytes.chunks_exact(4) {
        let code_point = if little_endian {
            u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        } else {
            u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        };
        match char::from_u32(code_point) {
            Some(c) => chars.push(c),
            None => chars.push(char::REPLACEMENT_CHARACTER),
        }
    }
    chars.into_iter().collect()
}
