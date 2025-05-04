use std::borrow::Cow;
use std::collections::HashMap;

use html_escape::encode_quoted_attribute;
use num_bigint::{BigInt, ToBigInt};
use pyo3::exceptions::PyAttributeError;
use pyo3::intern;
use pyo3::prelude::*;
use pyo3::types::{PyInt, PyString, PyType};

use crate::utils::PyResultMethods;

pub struct Context {
    pub request: Option<Py<PyAny>>,
    pub context: HashMap<String, Py<PyAny>>,
    pub autoescape: bool,
}

#[derive(Debug)]
pub enum ContentString<'t> {
    String(Cow<'t, str>),
    HtmlSafe(Cow<'t, str>),
    HtmlUnsafe(Cow<'t, str>),
}

#[allow(clippy::needless_lifetimes)] // https://github.com/rust-lang/rust-clippy/issues/13923
impl<'t, 'py> ContentString<'t> {
    pub fn content(self) -> Cow<'t, str> {
        match self {
            Self::String(content) => content,
            Self::HtmlSafe(content) => content,
            Self::HtmlUnsafe(content) => Cow::Owned(encode_quoted_attribute(&content).to_string()),
        }
    }

    pub fn map_content(self, f: impl FnOnce(Cow<'t, str>) -> Cow<'t, str>) -> Content<'t, 'py> {
        match self {
            Self::String(content) => Content::String(f(content)),
            Self::HtmlSafe(content) => Content::HtmlSafe(f(content)),
            Self::HtmlUnsafe(content) => Content::HtmlUnsafe(f(content)),
        }
    }
}

fn resolve_python<'t>(value: Bound<'_, PyAny>, context: &Context) -> PyResult<ContentString<'t>> {
    if !context.autoescape {
        return Ok(ContentString::String(
            value.str()?.extract::<String>()?.into(),
        ));
    };
    let py = value.py();

    let value = match value.is_instance_of::<PyString>() {
        true => value,
        false => value.str()?.into_any(),
    };
    Ok(
        match value
            .getattr(intern!(py, "__html__"))
            .ok_or_isinstance_of::<PyAttributeError>(py)?
        {
            Ok(html) => ContentString::HtmlSafe(html.call0()?.extract::<String>()?.into()),
            Err(_) => ContentString::HtmlUnsafe(value.str()?.extract::<String>()?.into()),
        },
    )
}

#[derive(Debug, IntoPyObject)]
pub enum Content<'t, 'py> {
    Py(Bound<'py, PyAny>),
    String(Cow<'t, str>),
    HtmlSafe(Cow<'t, str>),
    HtmlUnsafe(Cow<'t, str>),
    Float(f64),
    Int(BigInt),
}

impl<'t, 'py> Content<'t, 'py> {
    pub fn render(self, context: &Context) -> PyResult<Cow<'t, str>> {
        Ok(match self {
            Self::Py(content) => resolve_python(content, context)?.content(),
            Self::String(content) => content,
            Self::HtmlSafe(content) => content,
            Self::HtmlUnsafe(content) => Cow::Owned(encode_quoted_attribute(&content).to_string()),
            Self::Float(content) => content.to_string().into(),
            Self::Int(content) => content.to_string().into(),
        })
    }

    pub fn resolve_string(self, context: &Context) -> PyResult<ContentString<'t>> {
        Ok(match self {
            Self::String(content) => ContentString::String(content),
            Self::HtmlSafe(content) => ContentString::HtmlSafe(content),
            Self::HtmlUnsafe(content) => ContentString::HtmlUnsafe(content),
            Self::Float(content) => ContentString::String(content.to_string().into()),
            Self::Int(content) => ContentString::String(content.to_string().into()),
            Self::Py(content) => return resolve_python(content, context),
        })
    }

    pub fn to_bigint(&self) -> Option<BigInt> {
        match self {
            Self::Int(left) => Some(left.clone()),
            Self::String(left) => match left.parse::<BigInt>() {
                Ok(left) => Some(left),
                Err(_) => None,
            },
            Self::HtmlSafe(left) => match left.parse::<BigInt>() {
                Ok(left) => Some(left),
                Err(_) => None,
            },
            Self::HtmlUnsafe(left) => match left.parse::<BigInt>() {
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

    pub fn to_py(&self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
        Ok(match self {
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
            Self::HtmlSafe(s) => {
                let string = s
                    .into_pyobject(py)
                    .expect("A string can always be converted to a Python str.");
                let safestring = py.import(intern!(py, "django.utils.safestring"))?;
                let mark_safe = safestring.getattr(intern!(py, "mark_safe"))?;
                mark_safe.call1((string,))?
            }
            Self::HtmlUnsafe(s) => s
                .into_pyobject(py)
                .expect("A string can always be converted to a Python str.")
                .into_any(),
        })
    }
}
