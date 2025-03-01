use crate::lex::common::{lex_numeric, lex_text, lex_translated, lex_variable, LexerError};
use crate::lex::tag::TagParts;
use crate::types::TemplateString;

#[derive(Debug, PartialEq, Eq)]
pub enum IfConditionTokenType {
    Numeric,
    Text,
    TranslatedText,
    Variable,
    Equal,
    NotEqual,
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
}

#[derive(Debug, PartialEq, Eq)]
pub struct IfConditionToken {
    at: (usize, usize),
    token_type: IfConditionTokenType,
}

pub struct IfConditionLexer<'t> {
    rest: &'t str,
    byte: usize,
}

impl<'t> IfConditionLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            rest: template.content(parts.at),
            byte: parts.at.0,
        }
    }

    fn lex_condition(&mut self) -> Result<IfConditionToken, LexerError> {
        let mut chars = self.rest.chars();
        let token = match chars.next().expect("self.rest is not empty") {
            '_' => {
                if let Some('(') = chars.next() {
                    self.lex_translated(&mut chars)?
                } else {
                    self.lex_variable()
                }
            }
            '"' => self.lex_text(&mut chars, '"')?,
            '\'' => self.lex_text(&mut chars, '\'')?,
            '0'..='9' | '-' => self.lex_numeric(),
            _ => self.lex_variable(),
        };
        self.lex_remainder()?;
        Ok(token)
    }

    fn lex_variable(&mut self) -> IfConditionToken {
        let (at, byte, rest) = lex_variable(self.byte, self.rest);
        self.rest = rest;
        self.byte = byte;
        IfConditionToken {
            token_type: IfConditionTokenType::Variable,
            at,
        }
    }

    fn lex_numeric(&mut self) -> IfConditionToken {
        let (at, byte, rest) = lex_numeric(self.byte, self.rest);
        self.rest = rest;
        self.byte = byte;
        IfConditionToken {
            at,
            token_type: IfConditionTokenType::Numeric,
        }
    }

    fn lex_text(
        &mut self,
        chars: &mut std::str::Chars,
        end: char,
    ) -> Result<IfConditionToken, LexerError> {
        match lex_text(self.byte, self.rest, chars, end) {
            Ok((at, byte, rest)) => {
                self.rest = rest;
                self.byte = byte;
                Ok(IfConditionToken {
                    token_type: IfConditionTokenType::Text,
                    at,
                })
            }
            Err(e) => {
                self.rest = "";
                Err(e)
            }
        }
    }

    fn lex_translated(
        &mut self,
        chars: &mut std::str::Chars,
    ) -> Result<IfConditionToken, LexerError> {
        match lex_translated(self.byte, self.rest, chars) {
            Ok((at, byte, rest)) => {
                self.rest = rest;
                self.byte = byte;
                Ok(IfConditionToken {
                    token_type: IfConditionTokenType::TranslatedText,
                    at,
                })
            }
            Err(e) => {
                self.rest = "";
                Err(e)
            }
        }
    }

    fn lex_remainder(&mut self) -> Result<(), LexerError> {
        let remainder = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());
        match remainder {
            0 => {
                let rest = self.rest.trim_start();
                self.byte += self.rest.len() - rest.len();
                self.rest = rest;
                Ok(())
            }
            n => {
                self.rest = "";
                let at = (self.byte, n).into();
                Err(LexerError::InvalidRemainder { at })
            }
        }
    }
}

impl Iterator for IfConditionLexer<'_> {
    type Item = Result<IfConditionToken, LexerError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let index = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());
        let (token_type, index) = match &self.rest[..index] {
            "==" => (IfConditionTokenType::Equal, index),
            "!=" => (IfConditionTokenType::NotEqual, index),
            "<" => (IfConditionTokenType::LessThan, index),
            ">" => (IfConditionTokenType::GreaterThan, index),
            "<=" => (IfConditionTokenType::LessThanEqual, index),
            ">=" => (IfConditionTokenType::GreaterThanEqual, index),
            _ => return Some(self.lex_condition()),
        };
        let at = (self.byte, index);

        let rest = &self.rest[index..];
        let next_index = rest
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(rest.len());
        self.byte += index + next_index;
        self.rest = &rest[next_index..];

        Some(Ok(IfConditionToken { at, token_type }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_variable() {
        let template = "{% if foo %}";
        let parts = TagParts { at: (6, 3) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let foo = IfConditionToken {
            at: (6, 3),
            token_type: IfConditionTokenType::Variable,
        };
        assert_eq!(tokens, vec![Ok(foo)]);
    }

    #[test]
    fn test_lex_numeric() {
        let template = "{% if 5.3 %}";
        let parts = TagParts { at: (6, 3) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let numeric = IfConditionToken {
            at: (6, 3),
            token_type: IfConditionTokenType::Numeric,
        };
        assert_eq!(tokens, vec![Ok(numeric)]);
    }

    #[test]
    fn test_lex_text() {
        let template = "{% if 'foo' %}";
        let parts = TagParts { at: (6, 5) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let text = IfConditionToken {
            at: (6, 5),
            token_type: IfConditionTokenType::Text,
        };
        assert_eq!(tokens, vec![Ok(text)]);
    }

    #[test]
    fn test_lex_text_double_quotes() {
        let template = "{% if \"foo\" %}";
        let parts = TagParts { at: (6, 5) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let text = IfConditionToken {
            at: (6, 5),
            token_type: IfConditionTokenType::Text,
        };
        assert_eq!(tokens, vec![Ok(text)]);
    }

    #[test]
    fn test_lex_translated() {
        let template = "{% if _('foo') %}";
        let parts = TagParts { at: (6, 8) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let text = IfConditionToken {
            at: (6, 8),
            token_type: IfConditionTokenType::TranslatedText,
        };
        assert_eq!(tokens, vec![Ok(text)]);
    }

    #[test]
    fn test_lex_equal() {
        let template = "{% if == %}";
        let parts = TagParts { at: (6, 2) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let equal = IfConditionToken {
            at: (6, 2),
            token_type: IfConditionTokenType::Equal,
        };
        assert_eq!(tokens, vec![Ok(equal)]);
    }

    #[test]
    fn test_lex_not_equal() {
        let template = "{% if != %}";
        let parts = TagParts { at: (6, 2) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let not_equal = IfConditionToken {
            at: (6, 2),
            token_type: IfConditionTokenType::NotEqual,
        };
        assert_eq!(tokens, vec![Ok(not_equal)]);
    }

    #[test]
    fn test_lex_less_than() {
        let template = "{% if < %}";
        let parts = TagParts { at: (6, 1) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let less_than = IfConditionToken {
            at: (6, 1),
            token_type: IfConditionTokenType::LessThan,
        };
        assert_eq!(tokens, vec![Ok(less_than)]);
    }

    #[test]
    fn test_lex_greater_than() {
        let template = "{% if > %}";
        let parts = TagParts { at: (6, 1) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let greater_than = IfConditionToken {
            at: (6, 1),
            token_type: IfConditionTokenType::GreaterThan,
        };
        assert_eq!(tokens, vec![Ok(greater_than)]);
    }

    #[test]
    fn test_lex_less_equal() {
        let template = "{% if <= %}";
        let parts = TagParts { at: (6, 2) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let less_equal = IfConditionToken {
            at: (6, 2),
            token_type: IfConditionTokenType::LessThanEqual,
        };
        assert_eq!(tokens, vec![Ok(less_equal)]);
    }

    #[test]
    fn test_lex_greater_equal() {
        let template = "{% if >= %}";
        let parts = TagParts { at: (6, 2) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let greater_equal = IfConditionToken {
            at: (6, 2),
            token_type: IfConditionTokenType::GreaterThanEqual,
        };
        assert_eq!(tokens, vec![Ok(greater_equal)]);
    }

    #[test]
    fn test_lex_complex_condition() {
        let template = "{% if foo.bar|default:'spam' and count >= 1.5 or enabled is not False %}";
        let parts = TagParts { at: (6, 63) };
        let lexer = IfConditionLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();

        let foobar = IfConditionToken {
            at: (6, 22),
            token_type: IfConditionTokenType::Variable,
        };
        let and = IfConditionToken {
            at: (29, 3),
            token_type: IfConditionTokenType::Variable,
        };
        let count = IfConditionToken {
            at: (33, 5),
            token_type: IfConditionTokenType::Variable,
        };
        let greater_equal = IfConditionToken {
            at: (39, 2),
            token_type: IfConditionTokenType::GreaterThanEqual,
        };
        let numeric = IfConditionToken {
            at: (42, 3),
            token_type: IfConditionTokenType::Numeric,
        };
        let or = IfConditionToken {
            at: (46, 2),
            token_type: IfConditionTokenType::Variable,
        };
        let enabled = IfConditionToken {
            at: (49, 7),
            token_type: IfConditionTokenType::Variable,
        };
        let is = IfConditionToken {
            at: (57, 2),
            token_type: IfConditionTokenType::Variable,
        };
        let not = IfConditionToken {
            at: (60, 3),
            token_type: IfConditionTokenType::Variable,
        };
        let falsey = IfConditionToken {
            at: (64, 5),
            token_type: IfConditionTokenType::Variable,
        };
        let condition = vec![
            Ok(foobar),
            Ok(and),
            Ok(count),
            Ok(greater_equal),
            Ok(numeric),
            Ok(or),
            Ok(enabled),
            Ok(is),
            Ok(not),
            Ok(falsey),
        ];
        assert_eq!(tokens, condition);
    }

    #[test]
    fn test_lex_invalid_remainder() {
        let template = "{% if 'foo'remainder %}";
        let parts = TagParts { at: (6, 14) };
        let mut lexer = IfConditionLexer::new(template.into(), parts);
        let error = lexer.next().unwrap().unwrap_err();
        assert_eq!(error, LexerError::InvalidRemainder { at: (11, 9).into() });
    }
}
