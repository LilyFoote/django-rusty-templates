use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use unicode_xid::UnicodeXID;

use super::QUOTE_LEN;

const START_TRANSLATE_LEN: usize = 2;
const END_TRANSLATE_LEN: usize = 1;

#[derive(Clone, Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum LexerError {
    #[error("Expected a complete string literal")]
    IncompleteString {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a complete translation string")]
    IncompleteTranslatedString {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a valid variable name")]
    InvalidVariableName {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Could not parse the remainder")]
    InvalidRemainder {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a string literal within translation")]
    MissingTranslatedString {
        #[label("here")]
        at: SourceSpan,
    },
}

pub fn lex_variable(byte: usize, rest: &str) -> ((usize, usize), usize, &str) {
    let mut in_text = None;
    let mut end = 0;
    for c in rest.chars() {
        match c {
            '"' => match in_text {
                None => in_text = Some('"'),
                Some('"') => in_text = None,
                _ => {}
            },
            '\'' => match in_text {
                None => in_text = Some('\''),
                Some('\'') => in_text = None,
                _ => {}
            },
            _ if in_text.is_some() => {}
            c if !c.is_xid_continue() && c != '.' && c != '|' && c != ':' => break,
            _ => {}
        }
        end += c.len_utf8();
    }
    let at = (byte, end);
    let rest = &rest[end..];
    let byte = byte + end;
    (at, byte, rest)
}

pub fn lex_text<'t>(
    byte: usize,
    rest: &'t str,
    chars: &mut std::str::Chars,
    end: char,
) -> Result<((usize, usize), usize, &'t str), LexerError> {
    let mut count = 1;
    loop {
        let next = match chars.next() {
            None => {
                let at = (byte, count);
                return Err(LexerError::IncompleteString { at: at.into() });
            }
            Some(c) => c,
        };
        count += next.len_utf8();
        if next == '\\' {
            let next = match chars.next() {
                Some(next) => next,
                None => {
                    let at = (byte, count);
                    return Err(LexerError::IncompleteString { at: at.into() });
                }
            };
            count += next.len_utf8();
        } else if next == end {
            let at = (byte, count);
            let rest = &rest[count..];
            let byte = byte + count;
            return Ok((at, byte, rest));
        }
    }
}

pub fn lex_translated<'t>(
    byte: usize,
    rest: &'t str,
    chars: &mut std::str::Chars,
) -> Result<((usize, usize), usize, &'t str), LexerError> {
    let start = byte;
    let byte = byte + START_TRANSLATE_LEN;
    let rest = &rest[START_TRANSLATE_LEN..];
    let (_at, byte, rest) = match chars.next() {
        None => {
            let at = (start, START_TRANSLATE_LEN);
            return Err(LexerError::MissingTranslatedString { at: at.into() });
        }
        Some('\'') => lex_text(byte, rest, chars, '\'')?,
        Some('"') => lex_text(byte, rest, chars, '"')?,
        _ => {
            let at = (start, rest.len() + START_TRANSLATE_LEN);
            return Err(LexerError::MissingTranslatedString { at: at.into() });
        }
    };
    match chars.next() {
        Some(')') => {
            let byte = byte + END_TRANSLATE_LEN;
            let rest = &rest[END_TRANSLATE_LEN..];
            let at = (start, byte - start);
            Ok((at, byte, rest))
        }
        _ => {
            let at = (start, byte - start);
            Err(LexerError::IncompleteTranslatedString { at: at.into() })
        }
    }
}

pub fn lex_numeric(byte: usize, rest: &str) -> ((usize, usize), usize, &str) {
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '-' || c == '.' || c == 'e'))
        .unwrap_or(rest.len());
    let content = &rest[..end];
    // Match django bug
    let end = match content[1..].find('-') {
        Some(n) => n + 1,
        None => end,
    };
    // End match django bug
    let at = (byte, end);
    (at, byte + end, &rest[end..])
}

pub fn trim_variable(variable: &str) -> &str {
    match variable.find(|c: char| !c.is_xid_continue() && c != '.') {
        Some(end) => &variable[..end],
        None => variable,
    }
}

pub fn check_variable_attrs(variable: &str, start: usize) -> Result<(), LexerError> {
    let mut offset = 0;
    for var in variable.split('.') {
        match var.chars().next() {
            Some(c) if c != '_' => {
                offset += var.len() + 1;
                continue;
            }
            _ => {
                let at = (start + offset, var.len());
                return Err(LexerError::InvalidVariableName { at: at.into() });
            }
        }
    }
    Ok(())
}

pub fn lex_variable_argument(
    byte: usize,
    rest: &str,
) -> Result<((usize, usize), usize, &str), LexerError> {
    let content = trim_variable(rest);
    check_variable_attrs(content, byte)?;
    let end = content.len();
    let at = (byte, end);
    Ok((at, byte + end, &rest[end..]))
}

pub fn text_content_at(at: (usize, usize)) -> (usize, usize) {
    let (start, len) = at;
    let start = start + QUOTE_LEN;
    let len = len - 2 * QUOTE_LEN;
    (start, len)
}

pub fn translated_text_content_at(at: (usize, usize)) -> (usize, usize) {
    let (start, len) = at;
    let start = start + START_TRANSLATE_LEN + QUOTE_LEN;
    let len = len - START_TRANSLATE_LEN - END_TRANSLATE_LEN - 2 * QUOTE_LEN;
    (start, len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_text_non_ascii() {
        let template = "'N\u{ec655}'";
        let mut chars = template.chars();
        chars.next();
        let (at, byte, rest) = lex_text(1, template, &mut chars, '\'').unwrap();
        assert_eq!(at, (1, 7));
        assert_eq!(byte, 8);
        assert_eq!(rest, "");
    }

    #[test]
    fn test_lex_argument_non_ascii() {
        let template = "ZJ5G4YXZJUH6|default:\"#`´କ¯\"";
        let (at, byte, rest) = lex_variable(0, template);
        assert_eq!(at, (0, 32));
        assert_eq!(byte, 32);
        assert_eq!(rest, "");
    }
}
