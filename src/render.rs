pub mod common;
pub mod filters;
pub mod tags;
pub mod types;

use std::borrow::Cow;

use pyo3::prelude::*;

use crate::error::PyRenderError;
use crate::types::TemplateString;
use types::{Content, Context};

pub type ResolveResult<'t, 'py> = Result<Option<Content<'t, 'py>>, PyRenderError>;
pub type RenderResult<'t> = Result<Cow<'t, str>, PyRenderError>;

/// Trait for resolving a template element into content suitable for
/// further processing by another template element.
trait Resolve {
    fn resolve<'t, 'py>(
        &self,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> ResolveResult<'t, 'py>;
}

/// Trait for rendering a template element into content suitable for
/// output in the completely processed template.
pub trait Render {
    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> RenderResult<'t>;
}

/// Trait for evaluating an expression in a boolean context
pub trait Evaluate {
    fn evaluate(&self, py: Python<'_>, template: TemplateString<'_>, context: &mut Context)
        -> bool;
}

impl<T> Evaluate for Option<T>
where
    T: Evaluate,
{
    fn evaluate(
        &self,
        py: Python<'_>,
        template: TemplateString<'_>,
        context: &mut Context,
    ) -> bool {
        match self {
            Some(inner) => inner.evaluate(py, template, context),
            None => false,
        }
    }
}

/// All resolvable template elements can be rendered
impl<T> Render for T
where
    T: Resolve,
{
    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> RenderResult<'t> {
        match self.resolve(py, template, context)? {
            Some(content) => Ok(content.render(context)?),
            None => Ok(Cow::Borrowed("")),
        }
    }
}

impl<T> Render for Vec<T>
where
    T: Render,
{
    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> RenderResult<'t> {
        Ok(Cow::Owned(
            self.iter()
                .map(|node| node.render(py, template, context))
                .collect::<Result<Vec<_>, _>>()?
                .join(""),
        ))
    }
}

impl<T> Render for Option<T>
where
    T: Render,
{
    fn render<'t>(
        &self,
        py: Python<'_>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> RenderResult<'t> {
        Ok(match self {
            Some(inner) => inner.render(py, template, context)?,
            None => Cow::Borrowed(""),
        })
    }
}
