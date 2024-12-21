use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::lex::common::{
    check_variable_attrs, lex_numeric, lex_text, lex_translated, lex_variable_argument,
    trim_variable, LexerError,
};
use crate::lex::{END_TRANSLATE_LEN, QUOTE_LEN, START_TRANSLATE_LEN};

#[derive(Debug, PartialEq, Eq)]
pub enum ArgumentType {
    Numeric,
    Text,
    TranslatedText,
    Variable,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Argument {
    pub argument_type: ArgumentType,
    pub at: (usize, usize),
}

impl<'t> Argument {
    pub fn content_at(&self) -> (usize, usize) {
        match self.argument_type {
            ArgumentType::Variable => self.at,
            ArgumentType::Numeric => self.at,
            ArgumentType::Text => {
                let (start, len) = self.at;
                let start = start + QUOTE_LEN;
                let len = len - 2 * QUOTE_LEN;
                (start, len)
            }
            ArgumentType::TranslatedText => {
                let (start, len) = self.at;
                let start = start + START_TRANSLATE_LEN + QUOTE_LEN;
                let len = len - START_TRANSLATE_LEN - END_TRANSLATE_LEN - 2 * QUOTE_LEN;
                (start, len)
            }
        }
    }

    pub fn content(&self, template: &'t str) -> &'t str {
        let (start, len) = self.content_at();
        &template[start..start + len]
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct FilterToken {
    pub at: (usize, usize),
    pub argument: Option<Argument>,
}

impl<'t> FilterToken {
    pub fn content(&self, template: &'t str) -> &'t str {
        let (start, len) = self.at;
        &template[start..start + len]
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct VariableToken {
    pub at: (usize, usize),
}

impl<'t> VariableToken {
    pub fn content(&self, template: &'t str) -> &'t str {
        let (start, len) = self.at;
        &template[start..start + len]
    }
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum VariableLexerError {
    #[error("Variables and attributes may not begin with underscores")]
    LeadingUnderscore {
        #[label("here")]
        at: SourceSpan,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] LexerError),
    #[error("Expected a valid filter name")]
    InvalidFilterName {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected a valid variable name")]
    InvalidVariableName {
        #[label("here")]
        at: SourceSpan,
    },
}

pub fn lex_variable(
    variable: &str,
    start: usize,
) -> Result<Option<(VariableToken, FilterLexer)>, VariableLexerError> {
    let rest = variable.trim_start();
    if rest.trim().is_empty() {
        return Ok(None);
    }

    let start = start + variable.len() - rest.len();
    let content = trim_variable(rest);
    if content.is_empty() {
        let at = (start, rest.trim().len());
        return Err(VariableLexerError::InvalidVariableName { at: at.into() });
    }

    check_variable_attrs(content, start)?;

    let end = content.len();
    let at = (start, end);
    Ok(Some((
        VariableToken { at },
        FilterLexer::new(&rest[end..], start + end),
    )))
}

#[derive(Debug)]
pub struct FilterLexer<'t> {
    rest: &'t str,
    byte: usize,
}

impl<'t> FilterLexer<'t> {
    fn new(variable: &'t str, start: usize) -> Self {
        let offset = match variable.find('|') {
            Some(n) => n + 1,
            None => {
                return Self {
                    rest: "",
                    byte: start + variable.len(),
                }
            }
        };
        let variable = &variable[offset..];
        let rest = variable.trim_start();
        Self {
            rest: rest.trim_end(),
            byte: start + offset + variable.len() - rest.len(),
        }
    }

    fn lex_text(
        &mut self,
        chars: &mut std::str::Chars,
        end: char,
    ) -> Result<Argument, VariableLexerError> {
        match lex_text(self.byte, self.rest, chars, end) {
            Ok((at, byte, rest)) => {
                self.rest = rest;
                self.byte = byte;
                Ok(Argument {
                    argument_type: ArgumentType::Text,
                    at,
                })
            }
            Err(e) => {
                self.rest = "";
                Err(e.into())
            }
        }
    }

    fn lex_translated(
        &mut self,
        chars: &mut std::str::Chars,
    ) -> Result<Argument, VariableLexerError> {
        match lex_translated(self.byte, self.rest, chars) {
            Ok((at, byte, rest)) => {
                self.rest = rest;
                self.byte = byte;
                Ok(Argument {
                    argument_type: ArgumentType::TranslatedText,
                    at,
                })
            }
            Err(e) => {
                self.rest = "";
                Err(e.into())
            }
        }
    }

    fn lex_numeric(&mut self) -> Argument {
        let (at, byte, rest) = lex_numeric(self.byte, self.rest);
        self.rest = rest;
        self.byte = byte;
        Argument {
            argument_type: ArgumentType::Numeric,
            at,
        }
    }

    fn lex_variable_argument(&mut self) -> Result<Argument, VariableLexerError> {
        match lex_variable_argument(self.byte, self.rest) {
            Ok((at, byte, rest)) => {
                self.byte = byte;
                self.rest = rest;
                Ok(Argument {
                    argument_type: ArgumentType::Variable,
                    at,
                })
            }
            Err(e) => {
                self.rest = "";
                Err(e.into())
            }
        }
    }

    fn lex_filter(&mut self) -> Result<FilterToken, VariableLexerError> {
        let filter = self.rest.trim_start();
        let start = self.rest.len() - filter.len();
        self.byte += start;
        self.rest = &self.rest[start..];

        let end = filter
            .find(|c: char| !c.is_xid_continue())
            .unwrap_or(filter.len());
        let filter = &filter[..end];

        match filter.chars().next() {
            Some(c) if c.is_xid_start() => {
                let at = (self.byte, end);
                self.byte += end;
                self.rest = &self.rest[end..];
                let argument = self.lex_argument()?;
                Ok(FilterToken { at, argument })
            }
            _ => {
                let next = self.rest.find("|").unwrap_or(self.rest.len());
                let at = (self.byte, next);
                self.rest = "";
                Err(VariableLexerError::InvalidFilterName { at: at.into() })
            }
        }
    }

    fn lex_argument(&mut self) -> Result<Option<Argument>, VariableLexerError> {
        let next = match (self.rest.find("|"), self.rest.find(":")) {
            (_, None) => return Ok(None),
            (Some(f), Some(a)) if f < a => return Ok(None),
            (_, Some(a)) => a + 1,
        };
        self.rest = &self.rest[next..];
        self.byte += next;

        let mut chars = self.rest.chars();
        Ok(Some(match chars.next().unwrap() {
            '_' => {
                if let Some('(') = chars.next() {
                    self.lex_translated(&mut chars)?
                } else {
                    let end = self
                        .rest
                        .find(char::is_whitespace)
                        .unwrap_or(self.rest.len());
                    let at = (self.byte, end);
                    self.byte += self.rest.len();
                    self.rest = "";
                    return Err(VariableLexerError::LeadingUnderscore { at: at.into() });
                }
            }
            '\'' => self.lex_text(&mut chars, '\'')?,
            '"' => self.lex_text(&mut chars, '"')?,
            '0'..='9' | '-' => self.lex_numeric(),
            _ => self.lex_variable_argument()?,
        }))
    }

    fn lex_remainder(
        &mut self,
        token: FilterToken,
        remainder: &'t str,
        start_next: usize,
    ) -> Result<FilterToken, VariableLexerError> {
        match remainder.find(|c: char| !c.is_whitespace()) {
            None => {
                self.rest = &self.rest[start_next..];
                self.byte += start_next;
                Ok(token)
            }
            Some(n) => {
                let at = (self.byte + n, remainder.trim().len());
                self.rest = "";
                Err(LexerError::InvalidRemainder { at: at.into() }.into())
            }
        }
    }

    fn remainder_to_filter_or_argument(&mut self) -> (&'t str, usize) {
        match (self.rest.find("|"), self.rest.find(":")) {
            (None, None) => (self.rest, self.rest.len()),
            (None, Some(a)) => (&self.rest[..a], a + 1),
            (Some(f), Some(a)) if a < f => (&self.rest[..a], a + 1),
            (Some(f), _) => (&self.rest[..f], f + 1),
        }
    }
}

impl Iterator for FilterLexer<'_> {
    type Item = Result<FilterToken, VariableLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }
        let token = match self.lex_filter() {
            Err(e) => return Some(Err(e)),
            Ok(token) => token,
        };
        let (remainder, start_next) = self.remainder_to_filter_or_argument();
        Some(self.lex_remainder(token, remainder, start_next))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::lex::{END_TAG_LEN, START_TAG_LEN};

    fn contents(
        template: &str,
        tokens: Vec<Result<FilterToken, VariableLexerError>>,
    ) -> Vec<(&str, Option<&str>)> {
        tokens
            .iter()
            .map(|t| match t {
                Ok(t) => match t.argument {
                    Some(ref a) => (t.content(template), Some(a.content(template))),
                    None => (t.content(template), None),
                },
                Err(_) => unreachable!(),
            })
            .collect()
    }

    fn trim_variable(template: &str) -> &str {
        &template[START_TAG_LEN..(template.len() - END_TAG_LEN)]
    }

    #[test]
    fn test_lex_empty() {
        let variable = "  ";
        assert!(lex_variable(variable, START_TAG_LEN).unwrap().is_none());
    }

    #[test]
    fn test_lex_variable() {
        let template = "{{ foo.bar }}";
        let variable = trim_variable(template);
        let (token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        assert_eq!(token, VariableToken { at: (3, 7) });
        assert_eq!(token.content(template), "foo.bar");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable_index() {
        let template = "{{ 1 }}";
        let variable = trim_variable(template);
        let (token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        assert_eq!(token, VariableToken { at: (3, 1) });
        assert_eq!(token.content(template), "1");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_variable_negative_index() {
        let template = "{{ -1 }}";
        let variable = trim_variable(template);
        let err = lex_variable(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            VariableLexerError::InvalidVariableName { at: (3, 2).into() }
        );
    }

    #[test]
    fn test_lex_variable_start_underscore() {
        let variable = " _foo.bar ";
        let err = lex_variable(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidVariableName { at: (3, 4).into() }.into()
        );
    }

    #[test]
    fn test_lex_attribute_start_underscore() {
        let variable = " foo._bar ";
        let err = lex_variable(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidVariableName { at: (7, 4).into() }.into()
        );
    }

    #[test]
    fn test_lex_attribute_index() {
        let template = "{{ foo.1 }}";
        let variable = trim_variable(template);
        let (token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        assert_eq!(token, VariableToken { at: (3, 5) });
        assert_eq!(token.content(template), "foo.1");
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn test_lex_attribute_negative_index() {
        let template = "{{ foo.-1 }}";
        let variable = trim_variable(template);
        let err = lex_variable(variable, START_TAG_LEN).unwrap_err();
        assert_eq!(
            err,
            LexerError::InvalidVariableName { at: (7, 0).into() }.into()
        );
    }

    #[test]
    fn test_lex_filter() {
        let template = "{{ foo.bar|title }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                at: (11, 5),
                argument: None,
            })]
        );
        assert_eq!(contents(template, tokens), vec![("title", None)]);
    }

    #[test]
    fn test_lex_filter_chain() {
        let template = "{{ foo.bar|title|length }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: None,
                    at: (11, 5),
                }),
                Ok(FilterToken {
                    argument: None,
                    at: (17, 6),
                }),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("title", None), ("length", None)]
        );
    }

    #[test]
    fn test_lex_filter_remainder() {
        let template = "{{ foo.bar|title'foo' }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (16, 5).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_filter_invalid_start() {
        let template = "{{ foo.bar|'foo' }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::InvalidFilterName {
                at: (11, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_text_argument_single_quote() {
        let template = "{{ foo.bar|default:'foo' }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Text,
                    at: (19, 5),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_text_argument_double_quote() {
        let template = "{{ foo.bar|default:\"foo\" }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Text,
                    at: (19, 5),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_text_argument_escaped() {
        let template = "{{ foo.bar|default:'foo\\\'' }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Text,
                    at: (19, 7),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("default", Some("foo\\\'"))]
        );
    }

    #[test]
    fn test_lex_translated_text_argument() {
        let template = "{{ foo.bar|default:_('foo') }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::TranslatedText,
                    at: (19, 8),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_translated_text_argument_double_quoted() {
        let template = "{{ foo.bar|default:_(\"foo\") }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::TranslatedText,
                    at: (19, 8),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("foo"))]);
    }

    #[test]
    fn test_lex_numeric_argument() {
        let template = "{{ foo.bar|default:500 }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Numeric,
                    at: (19, 3),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("500"))]);
    }

    #[test]
    fn test_lex_numeric_argument_negative() {
        let template = "{{ foo.bar|default:-0.5 }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Numeric,
                    at: (19, 4),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("-0.5"))]);
    }

    #[test]
    fn test_lex_numeric_argument_scientific() {
        let template = "{{ foo.bar|default:5.2e3 }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Numeric,
                    at: (19, 5),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("5.2e3"))]);
    }

    #[test]
    fn test_lex_numeric_argument_scientific_negative_exponent() {
        // Django mishandles this case, so we do too:
        // https://code.djangoproject.com/ticket/35816
        let template = "{{ foo.bar|default:5.2e-3 }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Err(LexerError::InvalidRemainder { at: (23, 2).into() }.into()),
                /* When fixed we can do:
                Ok(FilterToken {
                    argument: Some(Argument {
                        argument_type: ArgumentType::Numeric,
                        at: (19, 6),
                    }),
                    at: (11, 7),
                })
                */
            ]
        );
        //assert_eq!(contents(template, tokens), vec![("default", Some("5.2e-3"))]);
    }

    #[test]
    fn test_lex_variable_argument() {
        let template = "{{ foo.bar|default:spam }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Ok(FilterToken {
                argument: Some(Argument {
                    argument_type: ArgumentType::Variable,
                    at: (19, 4),
                }),
                at: (11, 7),
            })]
        );
        assert_eq!(contents(template, tokens), vec![("default", Some("spam"))]);
    }

    #[test]
    fn test_lex_variable_argument_then_filter() {
        let template = "{{ foo.bar|default:spam|title }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: Some(Argument {
                        argument_type: ArgumentType::Variable,
                        at: (19, 4),
                    }),
                    at: (11, 7),
                }),
                Ok(FilterToken {
                    argument: None,
                    at: (24, 5),
                }),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("default", Some("spam")), ("title", None)]
        );
    }

    #[test]
    fn test_lex_string_argument_then_filter() {
        let template = "{{ foo.bar|default:\"spam\"|title }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![
                Ok(FilterToken {
                    argument: Some(Argument {
                        argument_type: ArgumentType::Text,
                        at: (19, 6),
                    }),
                    at: (11, 7),
                }),
                Ok(FilterToken {
                    argument: None,
                    at: (26, 5),
                }),
            ]
        );
        assert_eq!(
            contents(template, tokens),
            vec![("default", Some("spam")), ("title", None)]
        );
    }

    #[test]
    fn test_lex_argument_with_leading_underscore() {
        let template = "{{ foo.bar|default:_spam }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::LeadingUnderscore {
                at: (19, 5).into()
            })]
        );
    }

    #[test]
    fn test_lex_argument_with_only_underscore() {
        let template = "{{ foo.bar|default:_ }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(VariableLexerError::LeadingUnderscore {
                at: (19, 1).into()
            })]
        );
    }

    #[test]
    fn test_lex_text_argument_incomplete() {
        let template = "{{ foo.bar|default:'foo }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteString { at: (19, 4).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete() {
        let template = "{{ foo.bar|default:_('foo' }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteTranslatedString { at: (19, 7).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string() {
        let template = "{{ foo.bar|default:_('foo }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteString { at: (21, 4).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_incomplete_string_double_quotes() {
        let template = "{{ foo.bar|default:_(\"foo }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::IncompleteString { at: (21, 4).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string() {
        let template = "{{ foo.bar|default:_( }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::MissingTranslatedString { at: (19, 2).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_translated_text_argument_missing_string_trailing_chars() {
        let template = "{{ foo.bar|default:_(foo) }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        let error = LexerError::MissingTranslatedString { at: (19, 6).into() };
        assert_eq!(tokens, vec![Err(error.into())]);
    }

    #[test]
    fn test_lex_string_argument_remainder() {
        let template = "{{ foo.bar|default:\"spam\"title }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (25, 5).into() }.into()
            )]
        );
    }

    #[test]
    fn test_lex_string_argument_remainder_before_filter() {
        let template = "{{ foo.bar|default:\"spam\"title|title }}";
        let variable = trim_variable(template);
        let (_token, lexer) = lex_variable(variable, START_TAG_LEN).unwrap().unwrap();
        let tokens: Vec<_> = lexer.collect();
        assert_eq!(
            tokens,
            vec![Err(
                LexerError::InvalidRemainder { at: (25, 5).into() }.into()
            )]
        );
    }
}
