use std::borrow::Cow;

use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyNone};

use super::types::{Content, Context};
use super::{Evaluate, Render, RenderResult, Resolve, ResolveFailures, ResolveResult};
use crate::error::PyRenderError;
use crate::parse::{IfCondition, Tag, Url};
use crate::template::django_rusty_templates::NoReverseMatch;
use crate::types::TemplateString;
use crate::utils::PyResultMethods;

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

impl Resolve for Url {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
        failures: ResolveFailures,
    ) -> ResolveResult<'t, 'py> {
        let view_name = match self.view_name.resolve(py, template, context, failures)? {
            Some(view_name) => view_name,
            None => Content::String(Cow::Borrowed("")),
        };
        let urls = py.import("django.urls")?;
        let reverse = urls.getattr("reverse")?;

        let current_app = current_app(py, &context.request)?;
        let url = if self.kwargs.is_empty() {
            let py_args = PyList::empty(py);
            for arg in &self.args {
                py_args.append(arg.resolve(py, template, context, failures)?)?;
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
                kwargs.set_item(key, value.resolve(py, template, context, failures)?)?;
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

impl Evaluate for Content<'_, '_> {
    fn evaluate(
        &self,
        _py: Python<'_>,
        _template: TemplateString<'_>,
        _context: &mut Context,
    ) -> Option<bool> {
        Some(match self {
            Self::Py(obj) => obj.is_truthy().unwrap_or(false),
            Self::String(s) => !s.is_empty(),
            Self::HtmlSafe(s) => !s.is_empty(),
            Self::Float(f) => *f != 0.0,
            Self::Int(n) => *n != BigInt::ZERO,
        })
    }
}

trait PyCmp<T> {
    fn eq(&self, other: &T) -> bool;

    fn ne(&self, other: &T) -> bool {
        !self.eq(other)
    }

    fn lt(&self, other: &T) -> bool;

    fn gt(&self, other: &T) -> bool;

    fn lte(&self, other: &T) -> bool;

    fn gte(&self, other: &T) -> bool;
}

impl PyCmp<Content<'_, '_>> for Content<'_, '_> {
    fn eq(&self, other: &Content<'_, '_>) -> bool {
        match (self, other) {
            (Self::Py(obj), Content::Py(other)) => obj.eq(other).unwrap_or(false),
            (Self::Py(obj), Content::Float(other)) => obj.eq(other).unwrap_or(false),
            (Self::Py(obj), Content::Int(other)) => obj.eq(other).unwrap_or(false),
            (Self::Py(obj), Content::String(other)) => obj.eq(other).unwrap_or(false),
            (Self::Py(obj), Content::HtmlSafe(other)) => obj.eq(other).unwrap_or(false),
            (Self::Float(obj), Content::Py(other)) => other.eq(obj).unwrap_or(false),
            (Self::Int(obj), Content::Py(other)) => other.eq(obj).unwrap_or(false),
            (Self::String(obj), Content::Py(other)) => other.eq(obj).unwrap_or(false),
            (Self::HtmlSafe(obj), Content::Py(other)) => other.eq(obj).unwrap_or(false),
            (Self::Float(obj), Content::Float(other)) => obj == other,
            (Self::Int(obj), Content::Int(other)) => obj == other,
            (Self::Float(obj), Content::Int(other)) => {
                match other.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => false,
                    f64::NEG_INFINITY => false,
                    other => *obj == other,
                }
            }
            (Self::Int(obj), Content::Float(other)) => {
                match obj.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => false,
                    f64::NEG_INFINITY => false,
                    obj => obj == *other,
                }
            }
            (Self::String(obj), Content::String(other)) => obj == other,
            (Self::HtmlSafe(obj), Content::HtmlSafe(other)) => obj == other,
            (Self::String(obj), Content::HtmlSafe(other)) => obj == other,
            (Self::HtmlSafe(obj), Content::String(other)) => obj == other,
            _ => false,
        }
    }

    fn lt(&self, other: &Content<'_, '_>) -> bool {
        match (self, other) {
            (Self::Py(obj), Content::Py(other)) => obj.lt(other).unwrap_or(false),
            (Self::Py(obj), Content::Float(other)) => obj.lt(other).unwrap_or(false),
            (Self::Py(obj), Content::Int(other)) => obj.lt(other).unwrap_or(false),
            (Self::Py(obj), Content::String(other)) => obj.lt(other).unwrap_or(false),
            (Self::Py(obj), Content::HtmlSafe(other)) => obj.lt(other).unwrap_or(false),
            (Self::Float(obj), Content::Py(other)) => other.gt(obj).unwrap_or(false),
            (Self::Int(obj), Content::Py(other)) => other.gt(obj).unwrap_or(false),
            (Self::String(obj), Content::Py(other)) => other.gt(obj).unwrap_or(false),
            (Self::HtmlSafe(obj), Content::Py(other)) => other.gt(obj).unwrap_or(false),
            (Self::Float(obj), Content::Float(other)) => obj < other,
            (Self::Int(obj), Content::Int(other)) => obj < other,
            (Self::Float(obj), Content::Int(other)) => {
                match other.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => obj.is_finite() || *obj == f64::NEG_INFINITY,
                    f64::NEG_INFINITY => *obj == f64::NEG_INFINITY,
                    other => *obj < other,
                }
            }
            (Self::Int(obj), Content::Float(other)) => {
                match obj.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => *other == f64::INFINITY,
                    f64::NEG_INFINITY => other.is_finite() || *other == f64::INFINITY,
                    obj => obj < *other,
                }
            }
            (Self::String(obj), Content::String(other)) => obj < other,
            (Self::HtmlSafe(obj), Content::HtmlSafe(other)) => obj < other,
            (Self::String(obj), Content::HtmlSafe(other)) => obj < other,
            (Self::HtmlSafe(obj), Content::String(other)) => obj < other,
            _ => false,
        }
    }

    fn gt(&self, other: &Content<'_, '_>) -> bool {
        match (self, other) {
            (Self::Py(obj), Content::Py(other)) => obj.gt(other).unwrap_or(false),
            (Self::Py(obj), Content::Float(other)) => obj.gt(other).unwrap_or(false),
            (Self::Py(obj), Content::Int(other)) => obj.gt(other).unwrap_or(false),
            (Self::Py(obj), Content::String(other)) => obj.gt(other).unwrap_or(false),
            (Self::Py(obj), Content::HtmlSafe(other)) => obj.gt(other).unwrap_or(false),
            (Self::Float(obj), Content::Py(other)) => other.lt(obj).unwrap_or(false),
            (Self::Int(obj), Content::Py(other)) => other.lt(obj).unwrap_or(false),
            (Self::String(obj), Content::Py(other)) => other.lt(obj).unwrap_or(false),
            (Self::HtmlSafe(obj), Content::Py(other)) => other.lt(obj).unwrap_or(false),
            (Self::Float(obj), Content::Float(other)) => obj > other,
            (Self::Int(obj), Content::Int(other)) => obj > other,
            (Self::Float(obj), Content::Int(other)) => {
                match other.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => *obj == f64::INFINITY,
                    f64::NEG_INFINITY => obj.is_finite() || *obj == f64::INFINITY,
                    other => *obj > other,
                }
            }
            (Self::Int(obj), Content::Float(other)) => {
                match obj.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => other.is_finite() || *other == f64::NEG_INFINITY,
                    f64::NEG_INFINITY => *other == f64::NEG_INFINITY,
                    obj => obj > *other,
                }
            }
            (Self::String(obj), Content::String(other)) => obj > other,
            (Self::HtmlSafe(obj), Content::HtmlSafe(other)) => obj > other,
            (Self::String(obj), Content::HtmlSafe(other)) => obj > other,
            (Self::HtmlSafe(obj), Content::String(other)) => obj > other,
            _ => false,
        }
    }

    fn lte(&self, other: &Content<'_, '_>) -> bool {
        match (self, other) {
            (Self::Py(obj), Content::Py(other)) => obj.le(other).unwrap_or(false),
            (Self::Py(obj), Content::Float(other)) => obj.le(other).unwrap_or(false),
            (Self::Py(obj), Content::Int(other)) => obj.le(other).unwrap_or(false),
            (Self::Py(obj), Content::String(other)) => obj.le(other).unwrap_or(false),
            (Self::Py(obj), Content::HtmlSafe(other)) => obj.le(other).unwrap_or(false),
            (Self::Float(obj), Content::Py(other)) => other.ge(obj).unwrap_or(false),
            (Self::Int(obj), Content::Py(other)) => other.ge(obj).unwrap_or(false),
            (Self::String(obj), Content::Py(other)) => other.ge(obj).unwrap_or(false),
            (Self::HtmlSafe(obj), Content::Py(other)) => other.ge(obj).unwrap_or(false),
            (Self::Float(obj), Content::Float(other)) => obj <= other,
            (Self::Int(obj), Content::Int(other)) => obj <= other,
            (Self::Float(obj), Content::Int(other)) => {
                match other.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => obj.is_finite() || *obj == f64::NEG_INFINITY,
                    f64::NEG_INFINITY => *obj == f64::NEG_INFINITY,
                    other => *obj <= other,
                }
            }
            (Self::Int(obj), Content::Float(other)) => {
                match obj.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => *other == f64::INFINITY,
                    f64::NEG_INFINITY => other.is_finite() || *other == f64::INFINITY,
                    obj => obj <= *other,
                }
            }
            (Self::String(obj), Content::String(other)) => obj <= other,
            (Self::HtmlSafe(obj), Content::HtmlSafe(other)) => obj <= other,
            (Self::String(obj), Content::HtmlSafe(other)) => obj <= other,
            (Self::HtmlSafe(obj), Content::String(other)) => obj <= other,
            _ => false,
        }
    }

    fn gte(&self, other: &Content<'_, '_>) -> bool {
        match (self, other) {
            (Self::Py(obj), Content::Py(other)) => obj.ge(other).unwrap_or(false),
            (Self::Py(obj), Content::Float(other)) => obj.ge(other).unwrap_or(false),
            (Self::Py(obj), Content::Int(other)) => obj.ge(other).unwrap_or(false),
            (Self::Py(obj), Content::String(other)) => obj.ge(other).unwrap_or(false),
            (Self::Py(obj), Content::HtmlSafe(other)) => obj.ge(other).unwrap_or(false),
            (Self::Float(obj), Content::Py(other)) => other.le(obj).unwrap_or(false),
            (Self::Int(obj), Content::Py(other)) => other.le(obj).unwrap_or(false),
            (Self::String(obj), Content::Py(other)) => other.le(obj).unwrap_or(false),
            (Self::HtmlSafe(obj), Content::Py(other)) => other.le(obj).unwrap_or(false),
            (Self::Float(obj), Content::Float(other)) => obj >= other,
            (Self::Int(obj), Content::Int(other)) => obj >= other,
            (Self::Float(obj), Content::Int(other)) => {
                match other.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => *obj == f64::INFINITY,
                    f64::NEG_INFINITY => obj.is_finite() || *obj == f64::INFINITY,
                    other => *obj >= other,
                }
            }
            (Self::Int(obj), Content::Float(other)) => {
                match obj.to_f64().expect("BigInt to f64 is always possible") {
                    f64::INFINITY => other.is_finite() || *other == f64::NEG_INFINITY,
                    f64::NEG_INFINITY => *other == f64::NEG_INFINITY,
                    obj => obj >= *other,
                }
            }
            (Self::String(obj), Content::String(other)) => obj >= other,
            (Self::HtmlSafe(obj), Content::HtmlSafe(other)) => obj >= other,
            (Self::String(obj), Content::HtmlSafe(other)) => obj >= other,
            (Self::HtmlSafe(obj), Content::String(other)) => obj >= other,
            _ => false,
        }
    }
}

impl PyCmp<Option<Content<'_, '_>>> for Option<Content<'_, '_>> {
    fn eq(&self, other: &Option<Content<'_, '_>>) -> bool {
        match (self, other) {
            (None, None) => true,
            (Some(obj), Some(other)) => obj.eq(other),
            (Some(obj), None) | (None, Some(obj)) => match obj {
                Content::Py(obj) => obj.eq(PyNone::get(obj.py())).unwrap_or(false),
                _ => false,
            },
        }
    }

    fn lt(&self, other: &Option<Content<'_, '_>>) -> bool {
        match (self, other) {
            (Some(obj), Some(other)) => obj.lt(other),
            _ => false,
        }
    }

    fn gt(&self, other: &Option<Content<'_, '_>>) -> bool {
        match (self, other) {
            (Some(obj), Some(other)) => obj.gt(other),
            _ => false,
        }
    }

    fn lte(&self, other: &Option<Content<'_, '_>>) -> bool {
        match (self, other) {
            (Some(obj), Some(other)) => obj.lte(other),
            _ => false,
        }
    }

    fn gte(&self, other: &Option<Content<'_, '_>>) -> bool {
        match (self, other) {
            (Some(obj), Some(other)) => obj.gte(other),
            _ => false,
        }
    }
}

impl PyCmp<bool> for Option<Content<'_, '_>> {
    fn eq(&self, other: &bool) -> bool {
        match self {
            Some(Content::Py(obj)) => obj.eq(other).unwrap_or(false),
            Some(Content::Float(f)) => {
                *f == match other {
                    true => 1.0,
                    false => 0.0,
                }
            }
            Some(Content::Int(n)) => {
                *n == match other {
                    true => 1.into(),
                    false => 0.into(),
                }
            }
            _ => false,
        }
    }

    fn lt(&self, other: &bool) -> bool {
        match self {
            Some(Content::Py(obj)) => obj.lt(other).unwrap_or(false),
            Some(Content::Float(f)) => {
                *f < match other {
                    true => 1.0,
                    false => 0.0,
                }
            }
            Some(Content::Int(n)) => {
                *n < match other {
                    true => 1.into(),
                    false => 0.into(),
                }
            }
            _ => false,
        }
    }

    fn gt(&self, other: &bool) -> bool {
        match self {
            Some(Content::Py(obj)) => obj.gt(other).unwrap_or(false),
            Some(Content::Float(f)) => {
                *f > match other {
                    true => 1.0,
                    false => 0.0,
                }
            }
            Some(Content::Int(n)) => {
                *n > match other {
                    true => 1.into(),
                    false => 0.into(),
                }
            }
            _ => false,
        }
    }

    fn lte(&self, other: &bool) -> bool {
        match self {
            Some(Content::Py(obj)) => obj.le(other).unwrap_or(false),
            Some(Content::Float(f)) => {
                *f <= match other {
                    true => 1.0,
                    false => 0.0,
                }
            }
            Some(Content::Int(n)) => {
                *n <= match other {
                    true => 1.into(),
                    false => 0.into(),
                }
            }
            _ => false,
        }
    }

    fn gte(&self, other: &bool) -> bool {
        match self {
            Some(Content::Py(obj)) => obj.ge(other).unwrap_or(false),
            Some(Content::Float(f)) => {
                *f >= match other {
                    true => 1.0,
                    false => 0.0,
                }
            }
            Some(Content::Int(n)) => {
                *n >= match other {
                    true => 1.into(),
                    false => 0.into(),
                }
            }
            _ => false,
        }
    }
}

trait Contains<T> {
    fn contains(&self, other: T) -> Option<bool>;
}

impl Contains<Option<Content<'_, '_>>> for Content<'_, '_> {
    fn contains(&self, other: Option<Content<'_, '_>>) -> Option<bool> {
        match other {
            None => match self {
                Self::Py(obj) => obj.contains(PyNone::get(obj.py())).ok(),
                _ => None,
            },
            Some(Content::Py(other)) => {
                let obj = self.to_py(other.py()).ok()?;
                obj.contains(other).ok()
            }
            Some(Content::String(other)) | Some(Content::HtmlSafe(other)) => match self {
                Self::String(obj) | Self::HtmlSafe(obj) => Some(obj.contains(other.as_ref())),
                Self::Int(_) | Self::Float(_) => None,
                Self::Py(obj) => obj.contains(other).ok(),
            },
            Some(Content::Int(n)) => match self {
                Self::Py(obj) => obj.contains(n).ok(),
                _ => None,
            },
            Some(Content::Float(f)) => match self {
                Self::Py(obj) => obj.contains(f).ok(),
                _ => None,
            },
        }
    }
}

impl Contains<bool> for Content<'_, '_> {
    fn contains(&self, other: bool) -> Option<bool> {
        match self {
            Self::Py(obj) => obj.contains(other).ok(),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Resolved<'t, 'py> {
    Content(Option<Content<'t, 'py>>),
    Evaluate(bool),
    None,
}

trait ResolveTuple<'t, 'py> {
    fn resolve(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> Result<(Resolved<'t, 'py>, Resolved<'t, 'py>), PyRenderError>;
}

impl<'t, 'py> ResolveTuple<'t, 'py> for (IfCondition, IfCondition) {
    fn resolve(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> Result<(Resolved<'t, 'py>, Resolved<'t, 'py>), PyRenderError> {
        const IGNORE: ResolveFailures = ResolveFailures::IgnoreVariableDoesNotExist;
        Ok(match self {
            (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                let left = l.resolve(py, template, context, IGNORE)?;
                let right = r.resolve(py, template, context, IGNORE)?;
                (Resolved::Content(left), Resolved::Content(right))
            }
            (IfCondition::Variable(l), r) => {
                let left = l.resolve(py, template, context, IGNORE)?;
                let right = r.evaluate(py, template, context);
                match right {
                    Some(right) => (Resolved::Content(left), Resolved::Evaluate(right)),
                    None => (Resolved::Content(left), Resolved::None),
                }
            }
            (l, IfCondition::Variable(r)) => {
                let left = l.evaluate(py, template, context);
                let right = r.resolve(py, template, context, IGNORE)?;
                match left {
                    Some(left) => (Resolved::Evaluate(left), Resolved::Content(right)),
                    None => (Resolved::None, Resolved::Content(right)),
                }
            }
            (l, r) => {
                let left = l.evaluate(py, template, context);
                let right = r.evaluate(py, template, context);
                match (left, right) {
                    (Some(left), Some(right)) => {
                        (Resolved::Evaluate(left), Resolved::Evaluate(right))
                    }
                    (Some(left), None) => (Resolved::Evaluate(left), Resolved::None),
                    (None, Some(right)) => (Resolved::None, Resolved::Evaluate(right)),
                    (None, None) => (Resolved::None, Resolved::None),
                }
            }
        })
    }
}

impl Evaluate for IfCondition {
    fn evaluate(
        &self,
        py: Python<'_>,
        template: TemplateString<'_>,
        context: &mut Context,
    ) -> Option<bool> {
        Some(match self {
            Self::Variable(v) => v.evaluate(py, template, context)?,
            Self::And(inner) => {
                let left = inner.0.evaluate(py, template, context).unwrap_or(false);
                let right = inner.1.evaluate(py, template, context).unwrap_or(false);
                if !left { false } else { right }
            }
            Self::Or(inner) => {
                let left = inner.0.evaluate(py, template, context);
                let right = inner.1.evaluate(py, template, context);
                if left? { true } else { right.unwrap_or(false) }
            }
            Self::Not(inner) => !inner.evaluate(py, template, context)?,
            Self::Equal(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => l.eq(&r),
                    (Resolved::Evaluate(l), Resolved::Content(r)) => r.eq(&l),
                    (Resolved::Content(l), Resolved::Evaluate(r)) => l.eq(&r),
                    (Resolved::Content(l), Resolved::None) => l.eq(&false),
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l.eq(&r),
                    (Resolved::Evaluate(l), Resolved::None) => !l,
                    (Resolved::None, _) => unreachable!(),
                }
            }
            Self::NotEqual(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => l.ne(&r),
                    (Resolved::Evaluate(l), Resolved::Content(r)) => r.ne(&l),
                    (Resolved::Content(l), Resolved::Evaluate(r)) => l.ne(&r),
                    (Resolved::Content(l), Resolved::None) => l.ne(&false),
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l.ne(&r),
                    (Resolved::Evaluate(l), Resolved::None) => l,
                    (Resolved::None, _) => unreachable!(),
                }
            }
            Self::LessThan(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => l.lt(&r),
                    (Resolved::Evaluate(l), Resolved::Content(r)) => r.gt(&l),
                    (Resolved::Content(l), Resolved::Evaluate(r)) => l.lt(&r),
                    (Resolved::Content(l), Resolved::None) => l.lt(&false),
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l < r,
                    (Resolved::Evaluate(_), Resolved::None) => false,
                    (Resolved::None, _) => unreachable!(),
                }
            }
            Self::GreaterThan(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => l.gt(&r),
                    (Resolved::Evaluate(l), Resolved::Content(r)) => r.lt(&l),
                    (Resolved::Content(l), Resolved::Evaluate(r)) => l.gt(&r),
                    (Resolved::Content(l), Resolved::None) => l.gt(&false),
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l > r,
                    (Resolved::Evaluate(l), Resolved::None) => l,
                    (Resolved::None, _) => unreachable!(),
                }
            }
            Self::LessThanEqual(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => l.lte(&r),
                    (Resolved::Evaluate(l), Resolved::Content(r)) => r.gte(&l),
                    (Resolved::Content(l), Resolved::Evaluate(r)) => l.lte(&r),
                    (Resolved::Content(l), Resolved::None) => l.lte(&false),
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l <= r,
                    (Resolved::Evaluate(l), Resolved::None) => !l,
                    (Resolved::None, _) => unreachable!(),
                }
            }
            Self::GreaterThanEqual(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => l.gte(&r),
                    (Resolved::Evaluate(l), Resolved::Content(r)) => r.lte(&l),
                    (Resolved::Content(l), Resolved::Evaluate(r)) => l.gte(&r),
                    (Resolved::Content(l), Resolved::None) => l.gte(&false),
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l >= r,
                    (Resolved::Evaluate(_), Resolved::None) => true,
                    (Resolved::None, _) => unreachable!(),
                }
            }
            Self::In(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(Some(r))) => {
                        r.contains(l).unwrap_or(false)
                    }
                    (Resolved::Evaluate(l), Resolved::Content(Some(r))) => {
                        r.contains(l).unwrap_or(false)
                    }
                    _ => false,
                }
            }
            Self::NotIn(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(Some(r))) => {
                        !(r.contains(l).unwrap_or(true))
                    }
                    (Resolved::Evaluate(l), Resolved::Content(Some(r))) => {
                        !(r.contains(l).unwrap_or(true))
                    }
                    _ => false,
                }
            }
            Self::Is(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => match (l, r) {
                        (Some(Content::Py(left)), Some(Content::Py(right))) => left.is(&right),
                        (Some(Content::Py(obj)), None) | (None, Some(Content::Py(obj))) => {
                            obj.is(PyNone::get(py).as_any())
                        }
                        (None, None) => true,
                        _ => false,
                    },
                    (Resolved::Evaluate(l), Resolved::Content(r)) => match r {
                        None => false,
                        Some(Content::Py(right)) => right.is(PyBool::new(py, l).as_any()),
                        _ => false,
                    },
                    _ => false,
                }
            }
            Self::IsNot(inner) => {
                let inner = match inner.resolve(py, template, context) {
                    Ok(inner) => inner,
                    Err(_) => return Some(false),
                };
                match inner {
                    (Resolved::Content(l), Resolved::Content(r)) => match (l, r) {
                        (Some(Content::Py(left)), Some(Content::Py(right))) => !left.is(&right),
                        (Some(Content::Py(obj)), None) | (None, Some(Content::Py(obj))) => {
                            !obj.is(PyNone::get(py).as_any())
                        }
                        (None, None) => false,
                        _ => true,
                    },
                    (Resolved::Evaluate(l), Resolved::Content(r)) => match r {
                        Some(Content::Py(right)) => !right.is(PyBool::new(py, l).as_any()),
                        _ => true,
                    },
                    (Resolved::Content(l), Resolved::Evaluate(r)) => match l {
                        Some(Content::Py(left)) => !left.is(PyBool::new(py, r).as_any()),
                        _ => true,
                    },
                    (Resolved::Evaluate(l), Resolved::Evaluate(r)) => l != r,
                    _ => todo!(),
                }
            }
        })
    }
}

impl Render for Tag {
    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> RenderResult<'t> {
        Ok(match self {
            Self::Autoescape { enabled, nodes } => {
                let autoescape = context.autoescape;
                context.autoescape = enabled.into();

                let mut rendered = vec![];
                for node in nodes {
                    rendered.push(node.render(py, template, context)?)
                }

                context.autoescape = autoescape;
                Cow::Owned(rendered.join(""))
            }
            Self::If {
                condition,
                truthy,
                falsey,
            } => {
                if condition.evaluate(py, template, context).unwrap_or(false) {
                    truthy.render(py, template, context)?
                } else {
                    falsey.render(py, template, context)?
                }
            }
            Self::Load => Cow::Borrowed(""),
            Self::Url(url) => url.render(py, template, context)?,
        })
    }
}
