use std::borrow::Cow;

use num_bigint::BigInt;
use num_traits::cast::ToPrimitive;
use pyo3::exceptions::PyAttributeError;
use pyo3::prelude::*;
use pyo3::types::{PyBool, PyDict, PyList, PyNone};

use super::types::{Content, Context};
use super::{Evaluate, Render, RenderResult, Resolve, ResolveResult};
use crate::error::{PyRenderError, RenderError};
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
    ) -> ResolveResult<'t, 'py> {
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

impl Evaluate for Content<'_, '_> {
    fn evaluate(
        &self,
        _py: Python<'_>,
        _template: TemplateString<'_>,
        _context: &mut Context,
    ) -> bool {
        match self {
            Self::Py(obj) => obj.is_truthy().unwrap_or(false),
            Self::String(s) => !s.is_empty(),
            Self::HtmlSafe(s) => !s.is_empty(),
            Self::Float(f) => *f != 0.0,
            Self::Int(n) => *n != BigInt::ZERO,
        }
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
            (None, None) => false,
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

impl Evaluate for IfCondition {
    fn evaluate(
        &self,
        py: Python<'_>,
        template: TemplateString<'_>,
        context: &mut Context,
    ) -> bool {
        match self {
            Self::Variable(v) => match v.resolve(py, template, context) {
                Ok(r) => r.evaluate(py, template, context),
                Err(_) => false,
            },
            Self::And(inner) => {
                if !inner.0.evaluate(py, template, context) {
                    false
                } else {
                    inner.1.evaluate(py, template, context)
                }
            }
            Self::Or(inner) => {
                if inner.0.evaluate(py, template, context) {
                    true
                } else {
                    inner.1.evaluate(py, template, context)
                }
            }
            Self::Not(inner) => !inner.evaluate(py, template, context),
            Self::Equal(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    left.eq(&right)
                }
                (IfCondition::Variable(l), r) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.evaluate(py, template, context);
                    left.eq(&right)
                }
                (l, IfCondition::Variable(r)) => {
                    let left = l.evaluate(py, template, context);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    right.eq(&left)
                }
                (l, r) => l.evaluate(py, template, context) == r.evaluate(py, template, context),
            },
            Self::NotEqual(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = match l.resolve(py, template, context) {
                        Ok(left) => left,
                        Err(PyRenderError::RenderError(RenderError::ArgumentDoesNotExist {
                            ..
                        })) => return false,
                        _ => None,
                    };
                    let right = match r.resolve(py, template, context) {
                        Ok(left) => left,
                        Err(PyRenderError::RenderError(RenderError::ArgumentDoesNotExist {
                            ..
                        })) => return false,
                        _ => None,
                    };
                    left.ne(&right)
                }
                (IfCondition::Variable(l), r) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.evaluate(py, template, context);
                    left.ne(&right)
                }
                (l, IfCondition::Variable(r)) => {
                    let left = l.evaluate(py, template, context);
                    let right = match r.resolve(py, template, context) {
                        Ok(left) => left,
                        Err(PyRenderError::RenderError(RenderError::ArgumentDoesNotExist {
                            ..
                        })) => return false,
                        _ => None,
                    };
                    right.ne(&left)
                }
                (l, r) => l.evaluate(py, template, context) != r.evaluate(py, template, context),
            },
            Self::LessThan(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    left.lt(&right)
                }
                (IfCondition::Variable(l), r) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.evaluate(py, template, context);
                    left.lt(&right)
                }
                (l, IfCondition::Variable(r)) => {
                    let left = l.evaluate(py, template, context);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    right.gt(&left)
                }
                #[allow(clippy::bool_comparison)] // I find the suggestion harder to understand
                (l, r) => l.evaluate(py, template, context) < r.evaluate(py, template, context),
            },
            Self::GreaterThan(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    left.gt(&right)
                }
                (IfCondition::Variable(l), r) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.evaluate(py, template, context);
                    left.gt(&right)
                }
                (l, IfCondition::Variable(r)) => {
                    let left = l.evaluate(py, template, context);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    right.lt(&left)
                }
                #[allow(clippy::bool_comparison)] // I find the suggestion harder to understand
                (l, r) => l.evaluate(py, template, context) > r.evaluate(py, template, context),
            },
            Self::LessThanEqual(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    left.lte(&right)
                }
                (IfCondition::Variable(l), r) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.evaluate(py, template, context);
                    left.lte(&right)
                }
                (l, IfCondition::Variable(r)) => {
                    let left = l.evaluate(py, template, context);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    right.gte(&left)
                }
                (l, r) => l.evaluate(py, template, context) <= r.evaluate(py, template, context),
            },
            Self::GreaterThanEqual(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    left.gte(&right)
                }
                (IfCondition::Variable(l), r) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.evaluate(py, template, context);
                    left.gte(&right)
                }
                (l, IfCondition::Variable(r)) => {
                    let left = l.evaluate(py, template, context);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    right.lte(&left)
                }
                (l, r) => l.evaluate(py, template, context) >= r.evaluate(py, template, context),
            },
            Self::In(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let right = match r.resolve(py, template, context) {
                        Ok(Some(right)) => right,
                        _ => return false,
                    };
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    right.contains(left).unwrap_or(false)
                }
                (l, IfCondition::Variable(r)) => {
                    let right = match r.resolve(py, template, context) {
                        Ok(Some(right)) => right,
                        _ => return false,
                    };
                    let left = l.evaluate(py, template, context);
                    right.contains(left).unwrap_or(false)
                }
                _ => false,
            },
            Self::NotIn(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let right = match r.resolve(py, template, context) {
                        Ok(Some(right)) => right,
                        _ => return false,
                    };
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    match right.contains(left) {
                        Some(b) => !b,
                        None => false,
                    }
                }
                (l, IfCondition::Variable(r)) => {
                    let right = match r.resolve(py, template, context) {
                        Ok(Some(right)) => right,
                        _ => return false,
                    };
                    let left = l.evaluate(py, template, context);
                    match right.contains(left) {
                        Some(b) => !b,
                        None => false,
                    }
                }
                _ => false,
            },
            Self::Is(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    match (left, right) {
                        (Some(Content::Py(left)), Some(Content::Py(right))) => left.is(&right),
                        (Some(Content::Py(obj)), None) | (None, Some(Content::Py(obj))) => {
                            obj.is(PyNone::get(py).as_any())
                        }
                        (None, None) => true,
                        _ => false,
                    }
                }
                (l, IfCondition::Variable(r)) => {
                    let right = match r.resolve(py, template, context) {
                        Ok(Some(right)) => right,
                        _ => return false,
                    };
                    let left = l.evaluate(py, template, context);
                    match right {
                        Content::Py(right) => right.is(PyBool::new(py, left).as_any()),
                        _ => false,
                    }
                }
                _ => false,
            },
            Self::IsNot(inner) => match &**inner {
                (IfCondition::Variable(l), IfCondition::Variable(r)) => {
                    let left = l.resolve(py, template, context).unwrap_or(None);
                    let right = r.resolve(py, template, context).unwrap_or(None);
                    match (left, right) {
                        (Some(Content::Py(left)), Some(Content::Py(right))) => !left.is(&right),
                        (Some(Content::Py(obj)), None) | (None, Some(Content::Py(obj))) => {
                            !obj.is(PyNone::get(py).as_any())
                        }
                        (None, None) => false,
                        _ => true,
                    }
                }
                (l, IfCondition::Variable(r)) => {
                    let right = match r.resolve(py, template, context) {
                        Ok(Some(right)) => right,
                        _ => return true,
                    };
                    let left = l.evaluate(py, template, context);
                    match right {
                        Content::Py(right) => !right.is(PyBool::new(py, left).as_any()),
                        _ => true,
                    }
                }
                (IfCondition::Variable(l), r) => {
                    let left = match l.resolve(py, template, context) {
                        Ok(Some(left)) => left,
                        _ => return true,
                    };
                    let right = r.evaluate(py, template, context);
                    match left {
                        Content::Py(left) => !left.is(PyBool::new(py, right).as_any()),
                        _ => true,
                    }
                }
                (l, r) => {
                    let left = l.evaluate(py, template, context);
                    let right = r.evaluate(py, template, context);
                    left != right
                }
            },
        }
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
                if condition.evaluate(py, template, context) {
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
