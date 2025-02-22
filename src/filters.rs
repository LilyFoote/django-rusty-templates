use crate::render::{Context, IntoBorrowedContent, IntoOwnedContent, Render, TemplateResult};
use crate::types::Argument;
use crate::{render::Content, types::TemplateString};
use pyo3::prelude::*;

#[derive(Debug)]
pub enum FilterType {
    Add(AddFilter),
    AddSlashes(AddSlashesFilter),
    Capfirst(CapfirstFilter),
    Default(DefaultFilter),
    External(ExternalFilter),
    Lower(LowerFilter),
}

pub trait ResolveFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py>;
}

#[derive(Debug)]
pub struct AddSlashesFilter;

impl ResolveFilter for AddSlashesFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(content) => content
                .render(context)?
                .replace(r"\", r"\\")
                .replace("\"", "\\\"")
                .replace("'", r"\'")
                .into_content(),
            None => "".into_content(),
        };
        Ok(content)
    }
}

#[derive(Debug)]
pub struct AddFilter {
    pub argument: Argument,
}

impl AddFilter {
    pub fn new(argument: Argument) -> Self {
        Self { argument: argument }
    }
}

impl ResolveFilter for AddFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let variable = match variable {
            Some(left) => left,
            None => return Ok(None),
        };
        let right = self
            .argument
            .resolve(py, template, context)?
            .expect("missing argument in context should already have raised");
        match (variable.to_bigint(), right.to_bigint()) {
            (Some(variable), Some(right)) => return Ok(Some(Content::Int(variable + right))),
            _ => {
                let variable = variable.to_py(py);
                let right = right.to_py(py);
                match variable.add(right) {
                    Ok(sum) => return Ok(Some(Content::Py(sum))),
                    Err(_) => return Ok(None),
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct CapfirstFilter;

impl ResolveFilter for CapfirstFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(content) => {
                let content_string = content.render(context)?.into_owned();
                let mut chars = content_string.chars();
                let first_char = match chars.next() {
                    Some(c) => c.to_uppercase(),
                    None => return Ok("".into_content()),
                };
                let string: String = first_char.chain(chars).collect();
                string.into_content()
            }
            None => "".into_content(),
        };
        Ok(content)
    }
}

#[derive(Debug)]
pub struct DefaultFilter {
    pub argument: Argument,
}

impl DefaultFilter {
    pub fn new(argument: Argument) -> Self {
        Self { argument: argument }
    }
}

impl ResolveFilter for DefaultFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(left) => Some(left),
            None => self.argument.resolve(py, template, context)?,
        };
        Ok(content)
    }
}

#[derive(Debug)]
pub struct ExternalFilter {
    pub filter: Py<PyAny>,
    pub argument: Option<Argument>,
}

impl ExternalFilter {
    pub fn new(filter: Py<PyAny>, argument: Option<Argument>) -> Self {
        Self {
            filter: filter,
            argument: argument,
        }
    }
}

impl ResolveFilter for ExternalFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        py: Python<'py>,
        template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let arg = match &self.argument {
            Some(arg) => arg.resolve(py, template, context)?,
            None => None,
        };
        let filter = self.filter.bind(py);
        let value = match arg {
            Some(arg) => filter.call1((variable, arg))?,
            None => filter.call1((variable,))?,
        };
        Ok(Some(Content::Py(value)))
    }
}

#[derive(Debug)]
pub struct LowerFilter;

impl ResolveFilter for LowerFilter {
    fn resolve<'t, 'py>(
        &self,
        variable: Option<Content<'t, 'py>>,
        _py: Python<'py>,
        _template: TemplateString<'t>,
        context: &mut Context,
    ) -> TemplateResult<'t, 'py> {
        let content = match variable {
            Some(content) => content.render(context)?.to_lowercase().into_content(),
            None => "".into_content(),
        };
        Ok(content)
    }
}
