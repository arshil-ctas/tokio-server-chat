//! Very small, dependency-free syntax highlighter for code that shows up in
//! chat messages. It looks for two things in a message body:
//!   - fenced snippets: ```lang code here```
//!   - inline code: `code here`
//! and tokenizes them into keyword/string/number/comment/type/fn spans using
//! the active theme's colors. It's not a real parser (no crate for it fits
//! a single-line chat message anyway) but it's enough to make code readable
//! at a glance instead of one flat color.

use crate::theme::Theme;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

const KEYWORDS: &[&str] = &[
    // rust
    "fn", "let", "mut", "pub", "struct", "enum", "impl", "trait", "match", "if", "else",
    "for", "while", "loop", "return", "break", "continue", "use", "mod", "const", "static",
    "async", "await", "move", "ref", "self", "Self", "where", "dyn", "as", "in", "unsafe",
    // js/ts/py/go/generic overlap
    "function", "var", "const", "class", "extends", "import", "export", "from", "default",
    "def", "elif", "None", "True", "False", "lambda", "yield", "with", "except", "raise",
    "try", "catch", "finally", "throw", "new", "this", "void", "interface", "type",
    "package", "func", "go", "chan", "select", "defer", "range", "nil",
];

const TYPES: &[&str] = &[
    "String", "str", "u8", "u16", "u32", "u64", "usize", "i8", "i16", "i32", "i64", "isize",
    "f32", "f64", "bool", "char", "Vec", "Option", "Result", "Box", "Rc", "Arc", "HashMap",
    "int", "float", "bool", "list", "dict", "number", "string", "boolean", "any",
];

enum Tok {
    Word(String),
    Str(String),
    Num(String),
    Comment(String),
    Punct(String),
    Space(String),
}

fn tokenize(code: &str) -> Vec<Tok> {
    let mut out = Vec::new();
    let chars: Vec<char> = code.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            out.push(Tok::Comment(chars[i..].iter().collect()));
            break;
        }
        if c == '#' {
            out.push(Tok::Comment(chars[i..].iter().collect()));
            break;
        }
        if c == '"' || c == '\'' {
            let quote = c;
            let start = i;
            i += 1;
            while i < chars.len() && chars[i] != quote {
                i += 1;
            }
            i = (i + 1).min(chars.len());
            out.push(Tok::Str(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_ascii_digit() {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Num(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Tok::Word(chars[start..i].iter().collect()));
            continue;
        }
        if c.is_whitespace() {
            let start = i;
            while i < chars.len() && chars[i].is_whitespace() {
                i += 1;
            }
            out.push(Tok::Space(chars[start..i].iter().collect()));
            continue;
        }
        out.push(Tok::Punct(c.to_string()));
        i += 1;
    }
    out
}

/// Render one code snippet's tokens as styled spans using theme colors.
fn spans_for_code(code: &str, theme: &Theme) -> Vec<Span<'static>> {
    let toks = tokenize(code);
    let mut spans = Vec::with_capacity(toks.len());
    let mut prev_word_was_fn_kw = false;
    for (idx, t) in toks.iter().enumerate() {
        match t {
            Tok::Comment(s) => spans.push(Span::styled(
                s.clone(),
                Style::default().fg(theme.syn_comment).add_modifier(Modifier::ITALIC),
            )),
            Tok::Str(s) => spans.push(Span::styled(s.clone(), Style::default().fg(theme.syn_string))),
            Tok::Num(s) => spans.push(Span::styled(s.clone(), Style::default().fg(theme.syn_number))),
            Tok::Punct(s) => spans.push(Span::styled(s.clone(), Style::default().fg(theme.syn_default))),
            Tok::Space(s) => spans.push(Span::raw(s.clone())),
            Tok::Word(w) => {
                let next_is_paren = toks
                    .get(idx + 1)
                    .map(|n| matches!(n, Tok::Punct(p) if p == "("))
                    .unwrap_or(false);
                let style = if KEYWORDS.contains(&w.as_str()) {
                    prev_word_was_fn_kw = w == "fn" || w == "def" || w == "function" || w == "func";
                    Style::default().fg(theme.syn_keyword).add_modifier(Modifier::BOLD)
                } else if TYPES.contains(&w.as_str()) || w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                    Style::default().fg(theme.syn_type)
                } else if next_is_paren || prev_word_was_fn_kw {
                    Style::default().fg(theme.syn_fn)
                } else {
                    Style::default().fg(theme.syn_default)
                };
                if !(w == "fn" || w == "def" || w == "function" || w == "func") {
                    prev_word_was_fn_kw = false;
                }
                spans.push(Span::styled(w.clone(), style));
            }
        }
    }
    spans
}

/// Scan a message body for fenced (```lang code```) and inline (`code`)
/// snippets and return the body broken into plain-text and highlighted
/// spans, in order. Plain text keeps `plain_style`.
pub fn highlight_body(body: &str, theme: &Theme, plain_style: Style) -> Vec<Span<'static>> {
    let mut out = Vec::new();
    let bytes: Vec<char> = body.chars().collect();
    let mut i = 0;
    let mut plain_start = 0;

    let flush_plain = |out: &mut Vec<Span<'static>>, s: &str| {
        if !s.is_empty() {
            out.push(Span::styled(s.to_string(), plain_style));
        }
    };

    while i < bytes.len() {
        // fenced ```lang body```
        if bytes[i] == '`' && bytes.get(i + 1) == Some(&'`') && bytes.get(i + 2) == Some(&'`') {
            if let Some(end) = find_seq(&bytes, i + 3, "```") {
                flush_plain(&mut out, &bytes[plain_start..i].iter().collect::<String>());
                let inner: String = bytes[i + 3..end].iter().collect();
                // optional leading language tag before first space
                let code = match inner.split_once(' ') {
                    Some((lang, rest)) if lang.chars().all(|c| c.is_ascii_alphanumeric()) && !lang.is_empty() => rest,
                    _ => inner.as_str(),
                };
                out.push(Span::styled("[ ", Style::default().fg(theme.text_dim)));
                out.extend(spans_for_code(code, theme));
                out.push(Span::styled(" ]", Style::default().fg(theme.text_dim)));
                i = end + 3;
                plain_start = i;
                continue;
            }
        }
        // inline `code`
        if bytes[i] == '`' {
            if let Some(end) = find_seq(&bytes, i + 1, "`") {
                flush_plain(&mut out, &bytes[plain_start..i].iter().collect::<String>());
                let inner: String = bytes[i + 1..end].iter().collect();
                out.push(Span::styled("`", Style::default().fg(theme.text_dim)));
                out.extend(spans_for_code(&inner, theme));
                out.push(Span::styled("`", Style::default().fg(theme.text_dim)));
                i = end + 1;
                plain_start = i;
                continue;
            }
        }
        i += 1;
    }
    flush_plain(&mut out, &bytes[plain_start..].iter().collect::<String>());
    if out.is_empty() {
        out.push(Span::styled(body.to_string(), plain_style));
    }
    out
}

fn find_seq(chars: &[char], from: usize, seq: &str) -> Option<usize> {
    let seq: Vec<char> = seq.chars().collect();
    let n = seq.len();
    if from + n > chars.len() {
        // still allow searching, just means no match possible past here
    }
    let mut i = from;
    while i + n <= chars.len() {
        if chars[i..i + n] == seq[..] {
            return Some(i);
        }
        i += 1;
    }
    None
}
