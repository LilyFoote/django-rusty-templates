use crate::lex::tag::TagParts;
use crate::types::TemplateString;

#[derive(Debug, PartialEq)]
pub struct LoadToken {
    pub at: (usize, usize),
}

pub struct LoadLexer<'t> {
    rest: &'t str,
    byte: usize,
}

impl<'t> LoadLexer<'t> {
    pub fn new(template: TemplateString<'t>, parts: TagParts) -> Self {
        Self {
            rest: template.content(parts.at),
            byte: parts.at.0,
        }
    }
}

impl Iterator for LoadLexer<'_> {
    type Item = LoadToken;

    fn next(&mut self) -> Option<Self::Item> {
        if self.rest.is_empty() {
            return None;
        }

        let start = self.byte;
        let len = self
            .rest
            .find(char::is_whitespace)
            .unwrap_or(self.rest.len());

        let rest = &self.rest[len..];
        let next = rest
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(rest.len());
        self.rest = &rest[next..];
        self.byte = self.byte + len + next;

        let at = (start, len);
        Some(LoadToken { at })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lex_library() {
        let template = "{% load foo %}";
        let parts = TagParts { at: (8, 3) };
        let lexer = LoadLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let foo = LoadToken { at: (8, 3) };
        assert_eq!(tokens, [foo]);
    }

    #[test]
    fn test_lex_libraries() {
        let template = "{% load foo bar.eggs %}";
        let parts = TagParts { at: (8, 12) };
        let lexer = LoadLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let foo = LoadToken { at: (8, 3) };
        let bar_eggs = LoadToken { at: (12, 8) };
        assert_eq!(tokens, [foo, bar_eggs]);
    }

    #[test]
    fn test_lex_individual() {
        let template = "{% load foo bar from library %}";
        let parts = TagParts { at: (8, 20) };
        let lexer = LoadLexer::new(template.into(), parts);
        let tokens: Vec<_> = lexer.collect();
        let foo = LoadToken { at: (8, 3) };
        let bar = LoadToken { at: (12, 3) };
        let from = LoadToken { at: (16, 4) };
        let library = LoadToken { at: (21, 7) };
        assert_eq!(tokens, [foo, bar, from, library]);
    }
}
