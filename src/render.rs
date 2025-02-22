use std::borrow::Cow;
use std::collections::HashMap;

use html_escape::encode_quoted_attribute;
use num_bigint::{BigInt, ToBigInt};
use pyo3::exceptions::PyAttributeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyInt, PyList, PyString, PyType};

use crate::error::{PyRenderError, RenderError};
use crate::filters::FilterType;
use crate::filters::ResolveFilter;
use crate::parse::{Filter, Tag, TagElement, TokenTree, Url};
use crate::template::django_rusty_templates::NoReverseMatch;
use crate::types::Argument;
use crate::types::ArgumentType;
use crate::types::TemplateString;
use crate::types::Text;
use crate::types::Variable;
use crate::utils::PyResultMethods;

pub struct Context {
    pub request: Option<Py<PyAny>>,
    pub context: HashMap<String, Py<PyAny>>,
    pub autoescape: bool,
}

pub type TemplateResult<'t, 'py> = Result<Option<Content<'t, 'py>>, PyRenderError>;

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
    match value
        .getattr(intern!(py, "__html__"))
        .ok_or_isinstance_of::<PyAttributeError>(py)?
    {
        Ok(html) => html.call0()?.extract::<String>(),
        Err(_) => Ok(encode_quoted_attribute(&value.str()?.extract::<String>()?).to_string()),
    }
}

impl<'t, 'py> Content<'t, 'py> {
    pub fn render(self, context: &Context) -> PyResult<Cow<'t, str>> {
        let content = match self {
            Self::Py(content) => render_python(content, context)?,
            Self::String(content) => return Ok(content),
            Self::Float(content) => content.to_string(),
            Self::Int(content) => content.to_string(),
        };
        Ok(Cow::Owned(content))
    }

    pub fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Self::Int(left) => Some(left.clone()),
            Self::String(left) => match left.parse::<BigInt>() {
                Ok(left) => Some(left),
                Err(_) => None,
            },
            Self::Float(left) => left.trunc().to_bigint(),
            Self::Py(left) => match left.extract::<BigInt>() {
                Ok(left) => Some(left),
                Err(_) => {
                    let int = PyType::new::<PyInt>(left.py());
                    match int.call1((left,)) {
                        Ok(left) => Some(
                            left.extract::<BigInt>()
                                .expect("Python integers are BigInt compatible"),
                        ),
                        Err(_) => None,
                    }
                }
            },
        }
    }

    pub fn to_py(&self, py: Python<'py>) -> Bound<'py, PyAny> {
        match self {
            Self::Py(object) => object.clone(),
            Self::Int(i) => i
                .into_pyobject(py)
                .expect("A BigInt can always be converted to a Python int.")
                .into_any(),
            Self::Float(f) => f
                .into_pyobject(py)
                .expect("An f64 can always be converted to a Python float.")
                .into_any(),
            Self::String(s) => s
                .into_pyobject(py)
                .expect("A string can always be converted to a Python str.")
                .into_any(),
        }
    }
}

pub trait IntoOwnedContent<'t, 'py> {
    fn into_content(self) -> Option<Content<'t, 'py>>;
}

pub trait IntoBorrowedContent<'a, 't, 'py>
where
    'a: 't,
{
    fn into_content(&'a self) -> Option<Content<'t, 'py>>;
}

impl<'a, 't, 'py> IntoBorrowedContent<'a, 't, 'py> for str
where
    'a: 't,
{
    fn into_content(&'a self) -> Option<Content<'t, 'py>> {
        Some(Content::String(Cow::Borrowed(&self)))
    }
}

impl<'t, 'py> IntoOwnedContent<'t, 'py> for String {
    fn into_content(self) -> Option<Content<'t, 'py>> {
        Some(Content::String(Cow::Owned(self)))
    }
}

pub trait Render {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py>;

    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> Result<Cow<'t, str>, PyRenderError> {
        match self.resolve(py, template, context)? {
            Some(content) => Ok(content.render(context)?),
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
    ) -> TemplateResult<'t, 'py> {
        let mut parts = self.parts(template);
        let (first, mut object_at) = parts.next().expect("Variable names cannot be empty");
        let mut variable = match context.context.get(first) {
            Some(variable) => variable.bind(py).clone(),
            None => return Ok(None),
        };

        for (part, key_at) in parts {
            variable = match variable.get_item(part) {
                Ok(variable) => variable,
                Err(_) => match variable.getattr(part) {
                    Ok(variable) => variable,
                    Err(_) => {
                        let int = match part.parse::<usize>() {
                            Ok(int) => int,
                            Err(_) => {
                                return Err(RenderError::VariableDoesNotExist {
                                    key: part.to_string(),
                                    object: variable.str()?.to_string(),
                                    key_at: key_at.into(),
                                    object_at: Some(object_at.into()),
                                }
                                .into())
                            }
                        };
                        match variable.get_item(int) {
                            Ok(variable) => variable,
                            Err(_) => todo!(),
                        }
                    }
                },
            };
            object_at.1 += key_at.1 + 1;
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
    ) -> TemplateResult<'t, 'py> {
        let left = self.left.resolve(py, template, context)?;
        let result = match &self.filter {
            FilterType::Add(filter) => filter.resolve(left, py, template, context),
            FilterType::AddSlashes(filter) => filter.resolve(left, py, template, context),
            FilterType::Capfirst(filter) => filter.resolve(left, py, template, context),
            FilterType::Default(filter) => filter.resolve(left, py, template, context),
            FilterType::External(filter) => filter.resolve(left, py, template, context),
            FilterType::Lower(filter) => filter.resolve(left, py, template, context),
        };
        result
    }
}

fn current_app(py: Python, request: &Option<Py<PyAny>>) -> PyResult<Py<PyAny>> {
    let none = py.None();
    let request = match request {
        None => return Ok(none),
        Some(request) => request,
    };
    if let Ok(current_app) = request
        .getattr(py, "current_app")
        .ok_or_isinstance_of::<PyAttributeError>(py)?
    {
        return Ok(current_app);
    }
    match request
        .getattr(py, "resolver_match")
        .ok_or_isinstance_of::<PyAttributeError>(py)?
    {
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
    ) -> TemplateResult<'t, 'py> {
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
            reverse.call1((
                view_name,
                py.None(),
                py_args.to_tuple(),
                py.None(),
                current_app,
            ))
        } else {
            let kwargs = PyDict::new(py);
            for (key, value) in &self.kwargs {
                kwargs.set_item(key, value.resolve(py, template, context)?)?;
            }
            reverse.call1((view_name, py.None(), py.None(), kwargs, current_app))
        };
        match &self.variable {
            None => Ok(Some(Content::Py(url?))),
            Some(variable) => match url.ok_or_isinstance_of::<NoReverseMatch>(py)? {
                Ok(url) => {
                    context.context.insert(variable.clone(), url.unbind());
                    Ok(None)
                }
                Err(_) => Ok(None),
            },
        }
    }
}

impl Render for Tag {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        match self {
            Self::Load => Ok(None),
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
    ) -> TemplateResult<'t, 'py> {
        Ok(Some(Content::String(Cow::Borrowed(
            template.content(self.at),
        ))))
    }
}

impl Render for Argument {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        Ok(Some(match &self.argument_type {
            ArgumentType::Text(text) => return text.resolve(py, template, context),
            ArgumentType::TranslatedText(_text) => todo!(),
            ArgumentType::Variable(variable) => match variable.resolve(py, template, context)? {
                Some(content) => content,
                None => {
                    let key = template.content(variable.at).to_string();
                    let context: HashMap<&String, &Bound<'py, PyAny>> = context
                        .context
                        .iter()
                        .map(|(k, v)| (k, v.bind(py)))
                        .collect();
                    let object = format!("{:?}", context);
                    return Err(RenderError::VariableDoesNotExist {
                        key,
                        object,
                        key_at: variable.at.into(),
                        object_at: None,
                    }
                    .into());
                }
            },
            ArgumentType::Float(number) => Content::Float(*number),
            ArgumentType::Int(number) => Content::Int(number.clone()),
        }))
    }
}

impl Render for TagElement {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
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
    ) -> TemplateResult<'t, 'py> {
        match self {
            Self::Text(text) => text.resolve(py, template, context),
            Self::TranslatedText(_text) => todo!(),
            Self::Tag(tag) => tag.resolve(py, template, context),
            Self::Variable(variable) => variable.resolve(py, template, context),
            Self::Filter(filter) => filter.resolve(py, template, context),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::{AddSlashesFilter, DefaultFilter, LowerFilter};
    use crate::template::django_rusty_templates::{EngineData, Template};
    use crate::types::{Argument, ArgumentType};

    use pyo3::types::{PyDict, PyList, PyString};

    use crate::types::Text;

    #[test]
    fn test_render_variable() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "Lily").into_any();
            let context = HashMap::from([("name".to_string(), name.unbind())]);
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
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
            )
            .unwrap();

            let context = locals.extract().unwrap();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:'Bryony' }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 8),
                    argument_type: ArgumentType::Text(Text::new((17, 6))),
                })),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "Lily");
        })
    }

    #[test]
    fn test_render_filter_addslashes_single() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let name = PyString::new(py, "'hello'").into_any();
            let context = HashMap::from([("quotes".to_string(), name.unbind())]);
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ quotes|addslashes }}");
            let variable = Variable::new((3, 6));
            let filter = Filter {
                at: (10, 10),
                left: TagElement::Variable(variable),
                filter: FilterType::AddSlashes(AddSlashesFilter),
            };

            let rendered = filter.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, r"\'hello\'");
        })
    }

    #[test]
    fn test_render_filter_capfirst() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let template_string = "{{ var|capfirst }}".to_string();
            let context = PyDict::new(py);
            context.set_item("var", "hello world").unwrap();
            let template = Template::new_from_string(py, template_string, &engine).unwrap();
            let result = template.render(py, Some(context), None).unwrap();

            assert_eq!(result, "Hello world");

            let context = PyDict::new(py);
            context.set_item("var", "").unwrap();
            let template_string = "{{ var|capfirst }}".to_string();
            let template = Template::new_from_string(py, template_string, &engine).unwrap();
            let result = template.render(py, Some(context), None).unwrap();

            assert_eq!(result, "");

            let context = PyDict::new(py);
            context.set_item("bar", "").unwrap();
            let template_string = "{{ var|capfirst }}".to_string();
            let template = Template::new_from_string(py, template_string, &engine).unwrap();
            let result = template.render(py, Some(context), None).unwrap();

            assert_eq!(result, "");

            let template_string = "{{ var|capfirst:invalid }}".to_string();
            let error = Template::new_from_string(py, template_string, &engine).unwrap_err();

            let error_string = format!("{error}");
            assert!(error_string.contains("capfirst filter does not take an argument"));
        })
    }

    #[test]
    fn test_render_filter_default() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let context = HashMap::new();
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:'Bryony' }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 8),
                    argument_type: ArgumentType::Text(Text::new((17, 6))),
                })),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ count|default:12}}");
            let variable = Variable::new((3, 5));
            let filter = Filter {
                at: (9, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (17, 2),
                    argument_type: ArgumentType::Int(12.into()),
                })),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ count|default:3.5}}");
            let variable = Variable::new((3, 5));
            let filter = Filter {
                at: (9, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (17, 3),
                    argument_type: ArgumentType::Float(3.5),
                })),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:me}}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 2),
                    argument_type: ArgumentType::Variable(Variable::new((16, 2))),
                })),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|lower }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 5),
                left: TagElement::Variable(variable),
                filter: FilterType::Lower(LowerFilter),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|lower }}");
            let variable = Variable::new((3, 4));
            let filter = Filter {
                at: (8, 5),
                left: TagElement::Variable(variable),
                filter: FilterType::Lower(LowerFilter),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: false,
            };
            let template = TemplateString("{{ name|default:'Bryony'|lower }}");
            let variable = Variable::new((3, 4));
            let default = Filter {
                at: (8, 7),
                left: TagElement::Variable(variable),
                filter: FilterType::Default(DefaultFilter::new(Argument {
                    at: (16, 8),
                    argument_type: ArgumentType::Text(Text::new((17, 6))),
                })),
            };
            let lower = Filter {
                at: (25, 5),
                left: TagElement::Filter(Box::new(default)),
                filter: FilterType::Lower(LowerFilter),
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
            let mut context = Context {
                context,
                request: None,
                autoescape: true,
            };
            let template = TemplateString("{{ html }}");
            let html = Variable::new((3, 4));

            let rendered = html.render(py, template, &mut context).unwrap();
            assert_eq!(rendered, "&lt;p&gt;Hello World!&lt;/p&gt;");
        })
    }
}
