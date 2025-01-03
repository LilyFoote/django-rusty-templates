use std::borrow::Cow;
use std::collections::HashMap;

use html_escape::encode_quoted_attribute;
use num_bigint::BigInt;
use pyo3::exceptions::PyAttributeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyString};

use crate::parse::{Argument, ArgumentType, Filter, FilterType, Tag, TagElement, Text, TokenTree, Url, Variable};
use crate::template::django_rusty_templates::NoReverseMatch;
use crate::types::TemplateString;
use crate::utils::PyResultMethods;

pub struct Context {
    pub request: Option<Py<PyAny>>,
    pub context: HashMap<String, Py<PyAny>>,
    pub autoescape: bool,
}

#[derive(Debug, IntoPyObject)]
pub enum Content<'t, 'py> {
    Py(Bound<'py, PyAny>),
    String(Cow<'t, str>),
    Float(f64),
    Int(BigInt),
}

fn render_python(value: Bound<'_, PyAny>, context: &Context) -> PyResult<String> {
    if !context.autoescape {
        return value.str()?.extract::<String>();
    };
    let py = value.py();

    let value = match value.is_instance_of::<PyString>() {
        true => value,
        false => value.str()?.into_any(),
    };
    match value.getattr(intern!(py, "__html__")).ok_or_isinstance_of::<PyAttributeError>(py)? {
        Ok(html) => html.call0()?.extract::<String>(),
        Err(_) => Ok(encode_quoted_attribute(&value.str()?.extract::<String>()?).to_string()),
    }
}

impl<'t> Content<'t, '_> {
    fn render(self, context: &Context) -> PyResult<Cow<'t, str>> {
        let content = match self {
            Self::Py(content) => render_python(content, context)?,
            Self::String(content) => return Ok(content),
            Self::Float(content) => content.to_string(),
            Self::Int(content) => content.to_string(),
        };
        Ok(Cow::Owned(content))
    }
}

pub trait Render {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>>;

    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Cow<'t, str>> {
        match self.resolve(py, template, context)? {
            Some(content) => content.render(context),
            None => Ok(Cow::Borrowed("")),
        }
    }
}

impl Render for Variable {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let mut parts = self.parts(template);
        let first = parts.next().expect("Variable names cannot be empty");
        let mut variable = match context.context.get(first) {
            Some(variable) => variable.bind(py).clone(),
            None => return Ok(None),
        };
        for part in parts {
            variable = match variable.get_item(part) {
                Ok(variable) => variable,
                Err(_) => match variable.getattr(part) {
                    Ok(variable) => variable,
                    Err(e) => {
                        let int = match part.parse::<usize>() {
                            Ok(int) => int,
                            Err(_) => return Err(e),
                        };
                        match variable.get_item(int) {
                            Ok(variable) => variable,
                            Err(_) => todo!(),
                        }
                    }
                },
            }
        }
        Ok(Some(Content::Py(variable)))
    }
}

impl Render for Filter {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let left = self.left.resolve(py, template, context)?;
        Ok(match &self.filter {
            FilterType::Default(right) => match left {
                Some(left) => Some(left),
                None => right.resolve(py, template, context)?,
            },
            FilterType::External(filter, arg) => {
                let arg = match arg {
                    Some(arg) => arg.resolve(py, template, context)?,
                    None => None,
                };
                let value = filter.bind(py).call1((left, arg))?;
                Some(Content::Py(value))
            }
            FilterType::Lower => match left {
                Some(content) => Some(Content::String(Cow::Owned(content.render(context)?.to_lowercase()))),
                None => Some(Content::String(Cow::Borrowed(""))),
            }
        })
    }
}

fn current_app(py: Python, request: &Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    let none = py.None();
    let request = match request {
        None => return Ok(none),
        Some(request) => request,
    };
    if let Ok(current_app) = request.getattr(py, "current_app").ok_or_isinstance_of::<PyAttributeError>(py)? {
         return Ok(current_app);
    }
    match request.getattr(py, "resolver_match").ok_or_isinstance_of::<PyAttributeError>(py)? {
        Ok(resolver_match) => resolver_match.getattr(py, "namespace"),
        Err(_) => Ok(none),
    }
}

impl Render for Url {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        let view_name = match self.view_name.resolve(py, template, context)? {
            Some(view_name) => view_name,
            None => Content::String(Cow::Borrowed("")),
        };
        let urls = py.import("django.urls")?;
        let reverse = urls.getattr("reverse")?;

        let current_app = current_app(py, &context.request)?;
        let url = if self.kwargs.is_empty() {
            let py_args = PyList::empty(py);
            for arg in &self.args {
                py_args.append(arg.resolve(py, template, context)?)?;
            }
            reverse.call1((view_name, py.None(), py_args.to_tuple(), py.None(), current_app))
        } else {
            let kwargs = PyDict::new(py);
            for (key, value) in &self.kwargs {
                kwargs.set_item(key, value.resolve(py, template, context)?)?;
            }
            reverse.call1((view_name, py.None(), py.None(), kwargs, current_app))
        };
        match &self.variable {
            None => Ok(Some(Content::Py(url?))),
            Some(variable) => {
                match url.ok_or_isinstance_of::<NoReverseMatch>(py)? {
                    Ok(url) => {
                        context.context.insert(variable.clone(), url.unbind());
                        Ok(None)
                    }
                    Err(_) => Ok(None),
                }
            }
        }
    }
}

impl Render for Tag {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            Self::Url(url) => url.resolve(py, template, context),
        }
    }
}

impl Render for Text {
    fn resolve<'t, 'py>(
        &self,
        _py: Python<'py>,
        template: TemplateString<'t>,
        _context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        Ok(Some(Content::String(Cow::Borrowed(template.content(self.at)))))
    }
}

impl Render for TagElement {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            Self::Text(text) => text.resolve(py, template, context),
            Self::TranslatedText(_text) => todo!(),
            Self::Variable(variable) => variable.resolve(py, template, context),
            Self::Filter(filter) => filter.resolve(py, template, context),
            Self::Int(int) => Ok(Some(Content::Int(int.clone()))),
            Self::Float(float) => Ok(Some(Content::Float(*float))),
        }
    }
}

impl Render for TokenTree {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        match self {
            Self::Text(text) => text.resolve(py, template, context),
            Self::TranslatedText(_text) => todo!(),
            Self::Tag(tag) => tag.resolve(py, template, context),
            Self::Variable(variable) => variable.resolve(py, template, context),
            Self::Filter(filter) => filter.resolve(py, template, context),
        }
    }
}

impl Render for Argument {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> PyResult<Option<Content<'t, 'py>>> {
        Ok(Some(match &self.argument_type {
            ArgumentType::Text(text) => return text.resolve(py, template, context),
            ArgumentType::TranslatedText(_text) => todo!(),
            ArgumentType::Variable(variable) => return variable.resolve(py, template, context),
            ArgumentType::Float(number) => Content::Float(*number),
            ArgumentType::Int(number) => Content::Int(number.clone()),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use pyo3::types::{PyDict, PyList, PyString};

    use crate::parse::Text;

    #[test]
    fn test_render_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name.unbind())]);
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name }}");
            let variable = Variable::new((3, 4));

            let rendered = variable.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_dict_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let data = PyDict::new(py);
            let name = PyString::new(py, "Lily");
            data.set_item("name", name).unwrap();
            let context = HashMap::from([("data".to_string(), data.into_any().unbind())]);
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ data.name }}");
            let variable = Variable::new((3, 9));

            let rendered = variable.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_list_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily");
            let names = PyList::new(py, [name]).unwrap();
            let context = HashMap::from([("names".to_string(), names.into_any().unbind())]);
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ names.0 }}");
            let variable = Variable::new((3, 7));

            let rendered = variable.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_attribute_lookup() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let locals = PyDict::new(py);
            py.run(
                c"
class User:
    def __init__(self, name):
        self.name = name

user = User('Lily')
",
                None,
                Some(&locals),
            ).unwrap();

            let context = locals.extract().unwrap();
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ user.name }}");
            let variable = Variable::new((3, 9));

            let rendered = variable.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name.unbind())]);
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name|default:'Bryony' }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(Argument { at: (16, 8), argument_type: ArgumentType::Text(Text::new((17, 6)))}),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_default() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name|default:'Bryony' }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(Argument{ at: (16, 8), argument_type: ArgumentType::Text(Text::new((17, 6)))}),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Bryony");
        })
    }

    #[test]
    fn test_render_filter_default_integer() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ count|default:12}}");
            let variable = Variable::new((3, 5));
            let filter = Filter {
                at: (9, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(Argument { at: (17, 2), argument_type: ArgumentType::Int(12.into())}),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "12");
        })
    }

    #[test]
    fn test_render_filter_default_float() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ count|default:3.5}}");
            let variable = Variable::new((3, 5));
            let filter = Filter {
                at: (9, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(Argument{ at: (17, 3), argument_type: ArgumentType::Float(3.5)}),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "3.5");
        })
    }

    #[test]
    fn test_render_filter_default_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let me = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("me".to_string(), me.unbind())]);
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name|default:me}}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(Argument{ at: (16, 2), argument_type: ArgumentType::Variable(Variable::new((16, 2)))}),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_lower() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name.unbind())]);
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name|lower }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 5),
                left: TagElement::Variable(variable),
                filter: FilterType::Lower,
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "lily");
        })
    }

    #[test]
    fn test_render_filter_lower_missing_left() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name|lower }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 5),
                left: TagElement::Variable(variable),
                filter: FilterType::Lower,
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "");
        })
    }

    #[test]
    fn test_render_chained_filters() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context { context, request: None, autoescape: false };
            let template = TemplateString("{{ name|default:'Bryony'|lower }}");
            let variable = Variable::new((3, 4));
            let default = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(Argument { at: (16, 8), argument_type: ArgumentType::Text(Text::new((17, 6)))}),
            };
            let lower = Filter {
                at: (25, 5),
                left: TagElement::Filter(Box::new(default)),
                filter: FilterType::Lower,
            };

            let rendered = lower.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "bryony");
        })
    }

    #[test]
    fn test_render_html_autoescape() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let html = PyString::new(py, "<p>Hello World!</p>").into_any().unbind();
            let context = HashMap::from([("html".to_string(), html)]);
            let mut context = Context { context, request: None, autoescape: true };
            let template = TemplateString("{{ html }}");
            let html = Variable::new((3, 4));

            let rendered = html.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "&lt;p&gt;Hello World!&lt;/p&gt;");
        })
    }
}
