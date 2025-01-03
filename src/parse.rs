use std::collections::HashMap;

use miette::{Diagnostic, SourceSpan};
use num_bigint::BigInt;
use pyo3::prelude::*;
use thiserror::Error;

use crate::lex::core::{Lexer, TokenType};
use crate::lex::tag::{lex_tag, TagLexerError, TagParts};
use crate::lex::url::{UrlLexer, UrlLexerError, UrlToken, UrlTokenType};
use crate::lex::variable::{
    lex_variable, Argument as ArgumentToken, ArgumentType as ArgumentTokenType, VariableLexerError,
};
use crate::lex::START_TAG_LEN;
use crate::types::{CloneRef, TemplateString};

#[cfg(test)]
use crate::types::PyEq;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Variable {
    at: (usize, usize),
}

impl<'t> Variable {
    pub fn new(at: (usize, usize)) -> Self {
        Self { at }
    }

    pub fn parts(&self, template: TemplateString<'t>) -> impl Iterator<Item = &'t str> {
        let variable = template.content(self.at);
        variable.split(".")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Text {
    pub at: (usize, usize),
}

impl Text {
    pub fn new(at: (usize, usize)) -> Self {
        Self { at }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ArgumentType {
    Variable(Variable),
    Text(Text),
    TranslatedText(Text),
    Int(BigInt),
    Float(f64),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Argument {
    pub at: (usize, usize),
    pub argument_type: ArgumentType,
}

impl ArgumentToken {
    fn parse(&self, template: TemplateString<'_>) -> Result<Argument, ParseError> {
        Ok(Argument {
            at: self.at,
            argument_type: match self.argument_type {
                ArgumentTokenType::Variable => ArgumentType::Variable(Variable::new(self.at)),
                ArgumentTokenType::Text => ArgumentType::Text(Text::new(self.content_at())),
                ArgumentTokenType::Numeric => match template.content(self.at).parse::<BigInt>() {
                    Ok(n) => ArgumentType::Int(n),
                    Err(_) => match template.content(self.at).parse::<f64>() {
                        Ok(f) => ArgumentType::Float(f),
                        Err(_) => return Err(ParseError::InvalidNumber { at: self.at.into() }),
                    },
                },
                ArgumentTokenType::TranslatedText => {
                    ArgumentType::TranslatedText(Text::new(self.content_at()))
                }
            },
        })
    }
}

#[derive(Debug, PartialEq)]
pub enum TagElement {
    Int(BigInt),
    Float(f64),
    Text(Text),
    TranslatedText(Text),
    Variable(Variable),
    Filter(Box<Filter>),
}

impl CloneRef for TagElement {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            Self::Int(int) => Self::Int(int.clone()),
            Self::Float(float) => Self::Float(*float),
            Self::Text(text) => Self::Text(*text),
            Self::TranslatedText(text) => Self::TranslatedText(*text),
            Self::Variable(variable) => Self::Variable(*variable),
            Self::Filter(filter) => Self::Filter(Box::new(filter.clone_ref(py))),
        }
    }
}

#[cfg(test)]
impl PyEq for TagElement {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::TranslatedText(a), Self::TranslatedText(b)) => a == b,
            (Self::Variable(a), Self::Variable(b)) => a == b,
            (Self::Filter(a), Self::Filter(b)) => a.py_eq(b, py),
            _ => false,
        }
    }
}

#[derive(Debug)]
pub enum FilterType {
    Default(Argument),
    External(Py<PyAny>, Option<Argument>),
    Lower,
}

impl PartialEq for FilterType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Default(a), Self::Default(b)) => a == b,
            (Self::Lower, Self::Lower) => true,
            (Self::External(_, _), Self::External(_, _)) => false, // Can't compare PyAny to PyAny
            _ => false,
        }
    }
}

impl CloneRef for FilterType {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            Self::Default(arg) => Self::Default(arg.clone()),
            Self::External(filter, arg) => Self::External(filter.clone_ref(py), arg.clone()),
            Self::Lower => Self::Lower,
        }
    }
}

#[cfg(test)]
impl PyEq for FilterType {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        match (self, other) {
            (Self::Default(a), Self::Default(b)) => a == b,
            (Self::External(a1, a2), Self::External(b1, b2)) => {
                a2 == b2
                    && a1
                        .bind(py)
                        .eq(b1.bind(py))
                        .expect("__eq__ should not raise")
            }
            (Self::Lower, Self::Lower) => true,
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Filter {
    pub at: (usize, usize),
    pub left: TagElement,
    pub filter: FilterType,
}

impl Filter {
    pub fn new(
        parser: &Parser,
        at: (usize, usize),
        left: TagElement,
        right: Option<Argument>,
    ) -> Result<Self, ParseError> {
        let filter = match parser.template.content(at) {
            "default" => match right {
                Some(right) => FilterType::Default(right),
                None => return Err(ParseError::MissingArgument { at: at.into() }),
            },
            "lower" => match right {
                Some(right) => {
                    return Err(ParseError::UnexpectedArgument {
                        at: right.at.into(),
                    })
                }
                None => FilterType::Lower,
            },
            external => {
                let external = match parser.external_filters.get(external) {
                    Some(external) => external.clone().unbind(),
                    None => {
                        return Err(ParseError::InvalidFilter {
                            at: at.into(),
                            filter: external.to_string(),
                        })
                    }
                };
                FilterType::External(external, right)
            }
        };
        Ok(Self { at, left, filter })
    }
}

impl CloneRef for Filter {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        Self {
            at: self.at,
            left: self.left.clone_ref(py),
            filter: self.filter.clone_ref(py),
        }
    }
}

#[cfg(test)]
impl PyEq for Filter {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        self.at == other.at
            && self.left.py_eq(&other.left, py)
            && self.filter.py_eq(&other.filter, py)
    }
}

impl UrlToken {
    fn parse(&self, parser: &Parser) -> Result<TagElement, ParseError> {
        let content_at = self.content_at();
        let (start, _len) = content_at;
        let content = parser.template.content(content_at);
        match self.token_type {
            UrlTokenType::Numeric => match content.parse::<BigInt>() {
                Ok(n) => Ok(TagElement::Int(n)),
                Err(_) => match content.parse::<f64>() {
                    Ok(f) => Ok(TagElement::Float(f)),
                    Err(_) => Err(ParseError::InvalidNumber { at: self.at.into() }),
                },
            },
            UrlTokenType::Text => Ok(TagElement::Text(Text::new(content_at))),
            UrlTokenType::TranslatedText => Ok(TagElement::TranslatedText(Text::new(content_at))),
            UrlTokenType::Variable => parser.parse_variable(content, content_at, start),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Url {
    pub view_name: TagElement,
    pub args: Vec<TagElement>,
    pub kwargs: Vec<(String, TagElement)>,
    pub variable: Option<String>,
}

impl CloneRef for Url {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        Self {
            view_name: self.view_name.clone_ref(py),
            args: self.args.clone_ref(py),
            kwargs: self.kwargs.clone_ref(py),
            variable: self.variable.clone(),
        }
    }
}

#[cfg(test)]
impl PyEq for Url {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        self.variable == other.variable
            && self.view_name.py_eq(&other.view_name, py)
            && self.args.py_eq(&other.args, py)
            && self.kwargs.py_eq(&other.kwargs, py)
    }
}

#[derive(Debug, PartialEq)]
pub enum Tag {
    Url(Url),
}

impl CloneRef for Tag {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            Self::Url(url) => Self::Url(url.clone_ref(py)),
        }
    }
}

#[cfg(test)]
impl PyEq for Tag {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        match (self, other) {
            (Self::Url(a), Self::Url(b)) => a.py_eq(b, py),
            _ => false,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum TokenTree {
    Text(Text),
    TranslatedText(Text),
    Tag(Tag),
    Variable(Variable),
    Filter(Box<Filter>),
}

impl CloneRef for TokenTree {
    fn clone_ref(&self, py: Python<'_>) -> Self {
        match self {
            Self::Text(text) => Self::Text(*text),
            Self::TranslatedText(text) => Self::TranslatedText(*text),
            Self::Tag(tag) => Self::Tag(tag.clone_ref(py)),
            Self::Variable(variable) => Self::Variable(*variable),
            Self::Filter(filter) => Self::Filter(Box::new(filter.clone_ref(py))),
        }
    }
}

#[cfg(test)]
impl PyEq for TokenTree {
    fn py_eq(&self, other: &Self, py: Python<'_>) -> bool {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::TranslatedText(a), Self::TranslatedText(b)) => a == b,
            (Self::Tag(a), Self::Tag(b)) => a.py_eq(b, py),
            (Self::Variable(a), Self::Variable(b)) => a == b,
            (Self::Filter(a), Self::Filter(b)) => a.py_eq(b, py),
            _ => false,
        }
    }
}

impl From<TagElement> for TokenTree {
    fn from(tag_element: TagElement) -> Self {
        match tag_element {
            TagElement::Text(text) => Self::Text(text),
            TagElement::TranslatedText(text) => Self::TranslatedText(text),
            TagElement::Variable(variable) => Self::Variable(variable),
            TagElement::Filter(filter) => Self::Filter(filter),
            TagElement::Int(_) => todo!(),
            TagElement::Float(_) => todo!(),
        }
    }
}

#[derive(Error, Debug, Diagnostic, PartialEq, Eq)]
pub enum ParseError {
    #[error("Empty block tag")]
    EmptyTag {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Empty variable tag")]
    EmptyVariable {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected an argument")]
    MissingArgument {
        #[label("here")]
        at: SourceSpan,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    BlockError(#[from] TagLexerError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    UrlLexerError(#[from] UrlLexerError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    VariableError(#[from] VariableLexerError),
    #[error("Invalid filter: '{filter}'")]
    InvalidFilter {
        filter: String,
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Invalid numeric literal")]
    InvalidNumber {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Cannot mix arguments and keyword arguments")]
    MixedArgsKwargs {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("'url' view name must be a string or variable, not a number")]
    NumericUrlName {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("Expected an argument")]
    UnexpectedArgument {
        #[label("here")]
        at: SourceSpan,
    },
    #[error("'url' takes at least one argument, a URL pattern name")]
    UrlTagNoArguments {
        #[label("here")]
        at: SourceSpan,
    },
}

#[derive(Error, Debug)]
pub enum PyParseError {
    #[error(transparent)]
    PyErr(#[from] PyErr),
    #[error(transparent)]
    ParseError(#[from] ParseError),
}

impl PyParseError {
    pub fn try_into_parse_error(self) -> Result<ParseError, PyErr> {
        match self {
            Self::ParseError(err) => Ok(err),
            Self::PyErr(err) => Err(err),
        }
    }

    #[cfg(test)]
    pub fn unwrap_parse_error(self) -> ParseError {
        match self {
            Self::ParseError(err) => err,
            Self::PyErr(err) => panic!("{err:?}"),
        }
    }
}

pub struct Parser<'t, 'py> {
    py: Python<'py>,
    template: TemplateString<'t>,
    lexer: Lexer<'t>,
    external_filters: HashMap<String, Bound<'py, PyAny>>,
}

impl<'t, 'py> Parser<'t, 'py> {
    pub fn new(py: Python<'py>, template: TemplateString<'t>) -> Self {
        Self {
            py,
            template,
            lexer: Lexer::new(template),
            external_filters: HashMap::new(),
        }
    }

    #[cfg(test)]
    fn new_with_filters(
        py: Python<'py>,
        template: TemplateString<'t>,
        external_filters: HashMap<String, Bound<'py, PyAny>>,
    ) -> Self {
        Self {
            py,
            template,
            lexer: Lexer::new(template),
            external_filters,
        }
    }

    pub fn parse(&mut self) -> Result<Vec<TokenTree>, PyParseError> {
        let mut nodes = Vec::new();
        while let Some(token) = self.lexer.next() {
            nodes.push(match token.token_type {
                TokenType::Text => TokenTree::Text(Text::new(token.at)),
                TokenType::Comment => continue,
                TokenType::Variable => self
                    .parse_variable(
                        token.content(self.template),
                        token.at,
                        token.at.0 + START_TAG_LEN,
                    )?
                    .into(),
                TokenType::Tag => self.parse_tag(token.content(self.template), token.at)?,
            })
        }
        Ok(nodes)
    }

    fn parse_variable(
        &self,
        variable: &str,
        at: (usize, usize),
        start: usize,
    ) -> Result<TagElement, ParseError> {
        let (variable_token, filter_lexer) = match lex_variable(variable, start)? {
            None => return Err(ParseError::EmptyVariable { at: at.into() }),
            Some(t) => t,
        };
        let mut var = TagElement::Variable(Variable::new(variable_token.at));
        for filter_token in filter_lexer {
            let filter_token = filter_token?;
            let argument = match filter_token.argument {
                None => None,
                Some(ref a) => Some(a.parse(self.template)?),
            };
            let filter = Filter::new(self, filter_token.at, var, argument)?;
            var = TagElement::Filter(Box::new(filter));
        }
        Ok(var)
    }

    fn parse_tag(&mut self, tag: &'t str, at: (usize, usize)) -> Result<TokenTree, PyParseError> {
        let maybe_tag = match lex_tag(tag, at.0 + START_TAG_LEN) {
            Ok(maybe_tag) => maybe_tag,
            Err(e) => {
                let parse_error: ParseError = e.into();
                return Err(parse_error.into());
            }
        };
        let (tag, parts) = match maybe_tag {
            None => return Err(ParseError::EmptyTag { at: at.into() }.into()),
            Some(t) => t,
        };
        Ok(match self.template.content(tag.at) {
            "url" => self.parse_url(at, parts)?,
            _ => todo!(),
        })
    }

    fn parse_url(&mut self, at: (usize, usize), parts: TagParts) -> Result<TokenTree, ParseError> {
        let mut lexer = UrlLexer::new(self.template, parts);
        let view_name = match lexer.next() {
            Some(view_token) => view_token?.parse(self)?,
            None => return Err(ParseError::UrlTagNoArguments { at: at.into() }),
        };

        let mut tokens = vec![];
        for token in lexer {
            tokens.push(token?);
        }
        let mut rev = tokens.iter().rev();
        let variable = match (rev.next(), rev.next()) {
            (
                Some(UrlToken {
                    at: last,
                    token_type: UrlTokenType::Variable,
                    ..
                }),
                Some(UrlToken {
                    at: prev,
                    token_type: UrlTokenType::Variable,
                    ..
                }),
            ) => {
                let prev = self.template.content(*prev);
                if prev == "as" {
                    Some(self.template.content(*last).to_string())
                } else {
                    None
                }
            }
            _ => None,
        };
        if variable.is_some() {
            tokens.truncate(tokens.len() - 2)
        }
        let mut args = vec![];
        let mut kwargs = vec![];
        for token in tokens {
            let element = token.parse(self)?;
            match token.kwarg {
                None => args.push(element),
                Some(at) => {
                    let kwarg = self.template.content(at).to_string();
                    kwargs.push((kwarg, element));
                }
            }
        }
        if !args.is_empty() && !kwargs.is_empty() {
            return Err(ParseError::MixedArgsKwargs { at: at.into() });
        }
        let url = Url {
            view_name,
            args,
            kwargs,
            variable,
        };
        Ok(TokenTree::Tag(Tag::Url(url)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::lex::common::LexerError;

    #[test]
    fn test_empty_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();
            assert_eq!(nodes, vec![]);
        })
    }

    #[test]
    fn test_text() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "Some text";
            let template_string = TemplateString(template);
            let mut parser = Parser::new(py, template_string);
            let nodes = parser.parse().unwrap();
            let text = Text::new((0, template.len()));
            assert_eq!(nodes, vec![TokenTree::Text(text)]);
            assert_eq!(template_string.content(text.at), template);
        })
    }

    #[test]
    fn test_comment() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{# A commment #}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();
            assert_eq!(nodes, vec![]);
        })
    }

    #[test]
    fn test_empty_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{{ }}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::EmptyVariable { at: (0, 5).into() });
        })
    }

    #[test]
    fn test_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = TemplateString("{{ foo }}");
            let mut parser = Parser::new(py, template);
            let nodes = parser.parse().unwrap();
            let variable = Variable { at: (3, 3) };
            assert_eq!(nodes, vec![TokenTree::Variable(variable)]);
            assert_eq!(variable.parts(template).collect::<Vec<_>>(), vec!["foo"]);
        })
    }

    #[test]
    fn test_variable_attribute() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = TemplateString("{{ foo.bar.baz }}");
            let mut parser = Parser::new(py, template);
            let nodes = parser.parse().unwrap();
            let variable = Variable { at: (3, 11) };
            assert_eq!(nodes, vec![TokenTree::Variable(variable)]);
            assert_eq!(
                variable.parts(template).collect::<Vec<_>>(),
                vec!["foo", "bar", "baz"]
            );
        })
    }

    #[test]
    fn test_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = TemplateString("{{ foo|bar }}");
            let mut parser = Parser::new_with_filters(py, template, filters);
            let nodes = parser.parse().unwrap();

            let foo = Variable { at: (3, 3) };
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: TagElement::Variable(foo),
                filter: FilterType::External(py.None(), None),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
            assert_eq!(foo.parts(template).collect::<Vec<_>>(), vec!["foo"]);
        })
    }

    #[test]
    fn test_unknown_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = TemplateString("{{ foo|bar }}");
            let mut parser = Parser::new(py, template);
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(
                error,
                ParseError::InvalidFilter {
                    filter: "bar".to_string(),
                    at: (7, 3).into()
                }
            );
        })
    }

    #[test]
    fn test_filter_multiple() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{{ foo|bar|baz }}";
            let filters = HashMap::from([
                ("bar".to_string(), py.None().bind(py).clone()),
                ("baz".to_string(), py.None().bind(py).clone()),
            ]);
            let mut parser = Parser::new_with_filters(py, template.into(), filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let bar = TagElement::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(py.None(), None),
            }));
            let baz = TokenTree::Filter(Box::new(Filter {
                at: (11, 3),
                left: bar,
                filter: FilterType::External(py.None(), None),
            }));
            assert!(nodes.py_eq(&vec![baz], py));
        })
    }

    #[test]
    fn test_filter_argument() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = TemplateString("{{ foo|bar:baz }}");
            let mut parser = Parser::new_with_filters(py, template, filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let baz = Variable { at: (11, 3) };
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(
                    py.None(),
                    Some(Argument {
                        at: (11, 3),
                        argument_type: ArgumentType::Variable(baz),
                    }),
                ),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
            assert_eq!(baz.parts(template).collect::<Vec<_>>(), vec!["baz"]);
        })
    }

    #[test]
    fn test_filter_argument_text() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = TemplateString("{{ foo|bar:'baz' }}");
            let mut parser = Parser::new_with_filters(py, template, filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let baz = Text::new((12, 3));
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(
                    py.None(),
                    Some(Argument {
                        at: (11, 5),
                        argument_type: ArgumentType::Text(baz),
                    }),
                ),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
            assert_eq!(template.content(baz.at), "baz");
        })
    }

    #[test]
    fn test_filter_argument_translated_text() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = TemplateString("{{ foo|bar:_('baz') }}");
            let mut parser = Parser::new_with_filters(py, template, filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let baz = Text::new((14, 3));
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(
                    py.None(),
                    Some(Argument {
                        at: (11, 8),
                        argument_type: ArgumentType::TranslatedText(baz),
                    }),
                ),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
            assert_eq!(template.content(baz.at), "baz");
        })
    }

    #[test]
    fn test_filter_argument_float() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = "{{ foo|bar:5.2e3 }}";
            let mut parser = Parser::new_with_filters(py, template.into(), filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let num = Argument {
                at: (11, 5),
                argument_type: ArgumentType::Float(5.2e3),
            };
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(py.None(), Some(num)),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
        })
    }

    #[test]
    fn test_filter_argument_int() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = "{{ foo|bar:99 }}";
            let mut parser = Parser::new_with_filters(py, template.into(), filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let num = Argument {
                at: (11, 2),
                argument_type: ArgumentType::Int(99.into()),
            };
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(py.None(), Some(num)),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
        })
    }

    #[test]
    fn test_filter_argument_bigint() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let filters = HashMap::from([("bar".to_string(), py.None().bind(py).clone())]);
            let template = "{{ foo|bar:99999999999999999 }}";
            let mut parser = Parser::new_with_filters(py, template.into(), filters);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let num = Argument {
                at: (11, 17),
                argument_type: ArgumentType::Int("99999999999999999".parse::<BigInt>().unwrap()),
            };
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 3),
                left: foo,
                filter: FilterType::External(py.None(), Some(num)),
            }));
            assert!(nodes.py_eq(&vec![bar], py));
        })
    }

    #[test]
    fn test_filter_argument_invalid_number() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{{ foo|bar:9.9.9 }}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::InvalidNumber { at: (11, 5).into() });
        })
    }

    #[test]
    fn test_filter_default() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = TemplateString("{{ foo|default:baz }}");
            let mut parser = Parser::new(py, template);
            let nodes = parser.parse().unwrap();

            let foo = TagElement::Variable(Variable { at: (3, 3) });
            let baz = Variable { at: (15, 3) };
            let bar = TokenTree::Filter(Box::new(Filter {
                at: (7, 7),
                left: foo,
                filter: FilterType::Default(Argument {
                    at: (15, 3),
                    argument_type: ArgumentType::Variable(baz),
                }),
            }));
            assert_eq!(nodes, vec![bar]);
            assert_eq!(baz.parts(template).collect::<Vec<_>>(), vec!["baz"]);
        })
    }

    #[test]
    fn test_filter_default_missing_argument() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{{ foo|default|baz }}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::MissingArgument { at: (7, 7).into() });
        })
    }

    #[test]
    fn test_filter_lower_unexpected_argument() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{{ foo|lower:baz }}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::UnexpectedArgument { at: (13, 3).into() });
        })
    }

    #[test]
    fn test_variable_lexer_error() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{{ _foo }}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(
                error,
                ParseError::VariableError(
                    LexerError::InvalidVariableName { at: (3, 4).into() }.into()
                )
            );
        })
    }

    #[test]
    fn test_parse_empty_tag() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{%  %}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::EmptyTag { at: (0, 6).into() });
        })
    }

    #[test]
    fn test_block_error() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url'foo' %}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(
                error,
                ParseError::BlockError(TagLexerError::InvalidTagName { at: (3, 8).into() })
            );
        })
    }

    #[test]
    fn test_parse_url_tag() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url 'some-url-name' %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Text(Text { at: (8, 13) }),
                args: vec![],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_view_name_translated() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url _('some-url-name') %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::TranslatedText(Text { at: (10, 13) }),
                args: vec![],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_view_name_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Variable(Variable { at: (7, 14) }),
                args: vec![],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_view_name_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name|default:'home' %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let some_view_name = TagElement::Variable(Variable { at: (7, 14) });
            let home = Text { at: (31, 4) };
            let default = Box::new(Filter {
                at: (22, 7),
                left: some_view_name,
                filter: FilterType::Default(Argument {
                    at: (30, 6),
                    argument_type: ArgumentType::Text(home),
                }),
            });
            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Filter(default),
                args: vec![],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_no_arguments() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url %}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::UrlTagNoArguments { at: (0, 9).into() });
        })
    }

    #[test]
    fn test_parse_url_view_name_integer() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url 64 %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Int(64.into()),
                args: vec![],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_arguments() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name 'foo' bar|default:'home' 64 5.7 _(\"spam\") %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Variable(Variable { at: (7, 14) }),
                args: vec![
                    TagElement::Text(Text { at: (23, 3) }),
                    TagElement::Filter(Box::new(Filter {
                        at: (32, 7),
                        left: TagElement::Variable(Variable { at: (28, 3) }),
                        filter: FilterType::Default(Argument {
                            at: (40, 6),
                            argument_type: ArgumentType::Text(Text { at: (41, 4) }),
                        }),
                    })),
                    TagElement::Int(64.into()),
                    TagElement::Float(5.7),
                    TagElement::TranslatedText(Text { at: (57, 4) }),
                ],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_kwargs() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name foo='foo' extra=-64 %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Variable(Variable { at: (7, 14) }),
                args: vec![],
                kwargs: vec![
                    ("foo".to_string(), TagElement::Text(Text { at: (27, 3) })),
                    ("extra".to_string(), TagElement::Int((-64).into())),
                ],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_arguments_as_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name 'foo' as some_url %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Variable(Variable { at: (7, 14) }),
                args: vec![TagElement::Text(Text { at: (23, 3) })],
                kwargs: vec![],
                variable: Some("some_url".to_string()),
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_kwargs_as_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name foo='foo' as some_url %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Variable(Variable { at: (7, 14) }),
                args: vec![],
                kwargs: vec![("foo".to_string(), TagElement::Text(Text { at: (27, 3) }))],
                variable: Some("some_url".to_string()),
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_arguments_last_variables() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name 'foo' arg arg2 %}";
            let mut parser = Parser::new(py, template.into());
            let nodes = parser.parse().unwrap();

            let url = TokenTree::Tag(Tag::Url(Url {
                view_name: TagElement::Variable(Variable { at: (7, 14) }),
                args: vec![
                    TagElement::Text(Text { at: (23, 3) }),
                    TagElement::Variable(Variable { at: (28, 3) }),
                    TagElement::Variable(Variable { at: (32, 4) }),
                ],
                kwargs: vec![],
                variable: None,
            }));

            assert_eq!(nodes, vec![url]);
        })
    }

    #[test]
    fn test_parse_url_tag_mixed_args_kwargs() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url some_view_name 'foo' arg name=arg2 %}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(
                error,
                ParseError::MixedArgsKwargs {
                    at: (0, template.len()).into()
                }
            );
        })
    }

    #[test]
    fn test_parse_url_tag_invalid_number() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let template = "{% url foo 9.9.9 %}";
            let mut parser = Parser::new(py, template.into());
            let error = parser.parse().unwrap_err().unwrap_parse_error();
            assert_eq!(error, ParseError::InvalidNumber { at: (11, 5).into() });
        })
    }
}
