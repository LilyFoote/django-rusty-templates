use miette::{Diagnostic, SourceSpan};
use thiserror::Error;
use unicode_xid::UnicodeXID;

use crate::lex::common::{
    LexerError, lex_numeric, lex_text, lex_translated, lex_variable, text_content_at,
    translated_text_content_at,
};
use crate::lex::tag::TagParts;
use crate::types::TemplateString;

#[derive(Debug, PartialEq)]
pub enum UrlTokenType {
    Numeric,
    Text,
    TranslatedText,
    Variable,
}

#[derive(Debug, PartialEq)]
pub struct UrlToken {
    pub at: (usize, usize),
    pub token_type: UrlTokenType,
    pub kwarg: Option<(usize, usize)>,
}

impl UrlToken {
    pub fn content_at(&self) -> (usize, usize) {
        match self.token_type {
            UrlTokenType::Variable => self.at,
            UrlTokenType::Numeric => self.at,
            UrlTokenType::Text => text_content_at(self.at),
            UrlTokenType::TranslatedText => translated_text_content_at(self.at),
        }
    }
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum UrlLexerError {
    #[error(transparent)]
    #[diagnostic(transparent)]
    LexerError(#[from] LexerError),
    #[error("Incomplete keyword argument")]
    IncompleteKeywordArgument {
        #[label("here")]
        at: SourceSpan,
    },
}

pub struct UrlLexer<'t> {
    rest: &'t str,
    byte: usize,
}

impl<'t> UrlLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            rest: template.content(parts.at),
            byte: parts.at.0,
        }
    }

    fn lex_numeric(&mut self, kwarg: Option<(usize, usize)>) -> UrlToken {
        let (at, byte, rest) = lex_numeric(self.byte, self.rest);
        self.rest = rest;
        self.byte = byte;
        UrlToken {
            at,
            token_type: UrlTokenType::Numeric,
            kwarg,
        }
    }

    fn lex_text(
        &mut self,
        chars: &mut std::str::Chars,
        end: char,
        kwarg: Option<(usize, usize)>,
    ) -> Result<UrlToken, UrlLexerError> {
        match lex_text(self.byte, self.rest, chars, end) {
            Ok((at, byte, rest)) => {
                self.rest = rest;
                self.byte = byte;
                Ok(UrlToken {
                    token_type: UrlTokenType::Text,
                    at,
                    kwarg,
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
        kwarg: Option<(usize, usize)>,
    ) -> Result<UrlToken, UrlLexerError> {
        match lex_translated(self.byte, self.rest, chars) {
            Ok((at, byte, rest)) => {
                self.rest = rest;
                self.byte = byte;
                Ok(UrlToken {
                    token_type: UrlTokenType::TranslatedText,
                    at,
                    kwarg,
                })
            }
            Err(e) => {
                self.rest = "";
                Err(e.into())
            }
        }
    }

    fn lex_kwarg(&mut self) -> Option<(usize, usize)> {
        let index = self.rest.find('=')?;
        match self.rest.find(|c: char| !c.is_xid_continue()) {
            Some(n) if n < index => return None,
            _ => {}
        }
        let at = (self.byte, index);
        self.rest = &self.rest[index + 1..];
        self.byte += index + 1;
        Some(at)
    }

    fn lex_variable_or_filter(
        &mut self,
        kwarg: Option<(usize, usize)>,
    ) -> Result<UrlToken, UrlLexerError> {
        let (at, byte, rest) = lex_variable(self.byte, self.rest);
        self.rest = rest;
        self.byte = byte;
        Ok(UrlToken {
            token_type: UrlTokenType::Variable,
            at,
            kwarg,
        })
    }

    fn lex_remainder(
        &mut self,
        token: Result<UrlToken, UrlLexerError>,
    ) -> Result<UrlToken, UrlLexerError> {
        let remainder = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());
        match remainder {
            0 => {
                let rest = self.rest.trim_start();
                self.byte += self.rest.len() - rest.len();
                self.rest = rest;
                token
            }
            n => {
                self.rest = "";
                let at = (self.byte, n).into();
                let err = LexerError::InvalidRemainder { at };
                Err(err.into())
            }
        }
    }
}

impl Iterator for UrlLexer<'_> {
    type Item = Result<UrlToken, UrlLexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let kwarg = self.lex_kwarg();

        let mut chars = self.rest.chars();
        let next = match chars.next() {
            Some(next) if !next.is_whitespace() => next,
            _ => {
                self.rest = "";
                let at = kwarg.expect("kwarg is Some or we'd already have exited");
                let at = (at.0, at.1 + 1).into();
                return Some(Err(UrlLexerError::IncompleteKeywordArgument { at }));
            }
        };
        let token = match next {
            '_' => {
                if let Some('(') = chars.next() {
                    self.lex_translated(&mut chars, kwarg)
                } else {
                    self.lex_variable_or_filter(kwarg)
                }
            }
            '"' => self.lex_text(&mut chars, '"', kwarg),
            '\'' => self.lex_text(&mut chars, '\'', kwarg),
            '0'..='9' | '-' => Ok(self.lex_numeric(kwarg)),
            _ => self.lex_variable_or_filter(kwarg),
        };
        Some(self.lex_remainder(token))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_url_name_text() {
        let template = "{% url 'foo' %}";
        let parts = TagParts { at: (7, 5) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 5),
            token_type: UrlTokenType::Text,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_double_quotes() {
        let template = "{% url \"foo\" %}";
        let parts = TagParts { at: (7, 5) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 5),
            token_type: UrlTokenType::Text,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_incomplete() {
        let template = "{% url 'foo %}";
        let parts = TagParts { at: (7, 4) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::IncompleteString { at: (7, 4).into() }.into()
        );
    }

    #[test]
    fn test_lex_url_name_variable() {
        let template = "{% url foo %}";
        let parts = TagParts { at: (7, 3) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 3),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter() {
        let template = "{% url foo|default:'home' %}";
        let parts = TagParts { at: (7, 18) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 18),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter_inner_double_quote() {
        let template = "{% url foo|default:'home\"' %}";
        let parts = TagParts { at: (7, 19) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 19),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter_inner_single_quote() {
        let template = "{% url foo|default:\"home'\" %}";
        let parts = TagParts { at: (7, 19) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 19),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_filter_inner_whitespace() {
        let template = "{% url foo|default:'home url' %}";
        let parts = TagParts { at: (7, 22) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 22),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_leading_underscore() {
        let template = "{% url _foo %}";
        let parts = TagParts { at: (7, 4) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 4),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_translated() {
        let template = "{% url _('foo') %}";
        let parts = TagParts { at: (7, 8) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 8),
            token_type: UrlTokenType::TranslatedText,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_translated_incomplete() {
        let template = "{% url _('foo' %}";
        let parts = TagParts { at: (7, 7) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::IncompleteTranslatedString { at: (7, 7).into() }.into()
        );
    }

    #[test]
    fn test_lex_url_name_numeric() {
        let template = "{% url 5 %}";
        let parts = TagParts { at: (7, 1) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (7, 1),
            token_type: UrlTokenType::Numeric,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_kwarg() {
        let template = "{% url name='foo' %}";
        let parts = TagParts { at: (7, 10) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (12, 5),
            token_type: UrlTokenType::Text,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_text_kwarg_double_quotes() {
        let template = "{% url name=\"foo\" %}";
        let parts = TagParts { at: (7, 10) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (12, 5),
            token_type: UrlTokenType::Text,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_variable_kwarg() {
        let template = "{% url name=foo %}";
        let parts = TagParts { at: (7, 8) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (12, 3),
            token_type: UrlTokenType::Variable,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_leading_underscore_kwarg() {
        let template = "{% url name=_foo %}";
        let parts = TagParts { at: (7, 9) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (12, 4),
            token_type: UrlTokenType::Variable,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_translated_kwarg() {
        let template = "{% url name=_('foo') %}";
        let parts = TagParts { at: (7, 13) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (12, 8),
            token_type: UrlTokenType::TranslatedText,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url_name_numeric_kwarg() {
        let template = "{% url name=5 %}";
        let parts = TagParts { at: (7, 6) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let name = UrlToken {
            at: (12, 1),
            token_type: UrlTokenType::Numeric,
            kwarg: Some((7, 4)),
        };
        assert_eq!(tokens, vec![Ok(name)]);
    }

    #[test]
    fn test_lex_url() {
        let template = "{% url 'home' next %}";
        let parts = TagParts { at: (7, 11) };
        let lexer = UrlLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let home = UrlToken {
            at: (7, 6),
            token_type: UrlTokenType::Text,
            kwarg: None,
        };
        let next = UrlToken {
            at: (14, 4),
            token_type: UrlTokenType::Variable,
            kwarg: None,
        };
        assert_eq!(tokens, vec![Ok(home), Ok(next)]);
    }

    #[test]
    fn test_lex_url_incomplete_kwarg() {
        let template = "{% url name= %}";
        let parts = TagParts { at: (7, 5) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            UrlLexerError::IncompleteKeywordArgument { at: (7, 5).into() }
        );
    }

    #[test]
    fn test_lex_url_incomplete_kwarg_args() {
        let template = "{% url name= foo %}";
        let parts = TagParts { at: (7, 9) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            UrlLexerError::IncompleteKeywordArgument { at: (7, 5).into() }
        );
    }

    #[test]
    fn test_lex_url_invalid_remainder() {
        let template = "{% url 'foo'remainder %}";
        let parts = TagParts { at: (7, 14) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::InvalidRemainder { at: (12, 9).into() }.into()
        );
    }

    #[test]
    fn test_lex_url_kwarg_invalid_remainder() {
        let template = "{% url name='foo'=remainder %}";
        let parts = TagParts { at: (7, 20) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(
            error,
            LexerError::InvalidRemainder {
                at: (17, 10).into()
            }
            .into()
        );
    }

    #[test]
    fn test_lex_url_incomplete_kwarg_message() {
        let template = "{% url name= %}";
        let parts = TagParts { at: (7, 5) };
        let mut lexer = UrlLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error.to_string(), "Incomplete keyword argument");
    }
}
