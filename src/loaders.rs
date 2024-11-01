use std::collections::HashMap;
use std::path::{Path, PathBuf};

use encoding_rs::Encoding;
use pyo3::exceptions::PyUnicodeError;
use pyo3::prelude::*;

use crate::template::django_rusty_templates::Template;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderError {
    pub tried: Vec<(String, String)>,
}

fn safe_join(directory: &Path, template_name: &str) -> Option<PathBuf> {
    // TODO: check safety invariants
    // https://github.com/django/django/blob/4c3897bb154a3d3a94e5f7e146d0b8bf41e27d81/django/utils/_os.py#L9
    Some(directory.join(template_name))
}

pub struct FileSystemLoader {
    dirs: Vec<PathBuf>,
    encoding: &'static Encoding,
}

impl FileSystemLoader {
    pub fn new(dirs: Vec<String>, encoding: &'static Encoding) -> Self {
        Self {
            dirs: dirs.iter().map(PathBuf::from).collect(),
            encoding,
        }
    }

    fn get_template(
        &self,
        py: Python<'_>,
        template_name: &str,
    ) -> Result<PyResult<Template>, LoaderError> {
        let mut tried = Vec::new();
        for template_dir in &self.dirs {
            let path = match safe_join(template_dir, template_name) {
                Some(path) => path,
                None => continue,
            };
            let bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => {
                    tried.push((
                        path.display().to_string(),
                        "Source does not exist".to_string(),
                    ));
                    continue;
                }
            };
            let (contents, encoding, malformed) = self.encoding.decode(&bytes);
            if malformed {
                return Ok(Err(PyUnicodeError::new_err(format!(
                    "Could not open {path:?} with {} encoding.",
                    encoding.name()
                ))));
            }
            return Ok(Template::new(&contents, path));
        }
        Err(LoaderError { tried })
    }
}

pub struct AppDirsLoader {}

impl AppDirsLoader {
    fn get_template(
        &self,
        py: Python<'_>,
        template_name: &str,
    ) -> Result<PyResult<Template>, LoaderError> {
        todo!()
    }
}

pub struct CachedLoader {
    cache: HashMap<String, Result<Template, LoaderError>>,
    pub loaders: Vec<Loader>,
}

impl CachedLoader {
    pub fn new(loaders: Vec<Loader>) -> Self {
        Self {
            loaders,
            cache: HashMap::new(),
        }
    }

    fn get_template(
        &mut self,
        py: Python<'_>,
        template_name: &str,
    ) -> Result<PyResult<Template>, LoaderError> {
        match self.cache.get(template_name) {
            Some(Ok(template)) => Ok(Ok(template.clone())),
            Some(Err(e)) => Err(e.clone()),
            None => {
                let mut tried = Vec::new();
                for loader in &mut self.loaders {
                    match loader.get_template(py, template_name) {
                        Ok(Ok(template)) => {
                            self.cache
                                .insert(template_name.to_string(), Ok(template.clone()));
                            return Ok(Ok(template));
                        }
                        Ok(Err(e)) => return Ok(Err(e)),
                        Err(mut e) => tried.append(&mut e.tried),
                    }
                }
                Err(LoaderError { tried })
            }
        }
    }
}

pub struct LocMemLoader {}

impl LocMemLoader {
    fn get_template(
        &self,
        py: Python<'_>,
        template_name: &str,
    ) -> Result<PyResult<Template>, LoaderError> {
        todo!()
    }
}

pub struct ExternalLoader {}

impl ExternalLoader {
    fn get_template(
        &self,
        py: Python<'_>,
        template_name: &str,
    ) -> Result<PyResult<Template>, LoaderError> {
        todo!()
    }
}

pub enum Loader {
    FileSystem(FileSystemLoader),
    AppDirs(AppDirsLoader),
    Cached(CachedLoader),
    LocMem(LocMemLoader),
    External(ExternalLoader),
}

impl Loader {
    pub fn get_template(
        &mut self,
        py: Python<'_>,
        template_name: &str,
    ) -> Result<PyResult<Template>, LoaderError> {
        match self {
            Self::FileSystem(loader) => loader.get_template(py, template_name),
            Self::AppDirs(loader) => loader.get_template(py, template_name),
            Self::Cached(loader) => loader.get_template(py, template_name),
            Self::LocMem(loader) => loader.get_template(py, template_name),
            Self::External(loader) => loader.get_template(py, template_name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filesystem_loader() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let template = loader.get_template(py, "basic.txt").unwrap().unwrap();

            assert_eq!(
                template.filename.unwrap(),
                PathBuf::from("tests/templates/basic.txt")
            );
        })
    }

    #[test]
    fn test_filesystem_loader_missing_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let error = loader.get_template(py, "missing.txt").unwrap_err();

            assert_eq!(
                error,
                LoaderError {
                    tried: vec![(
                        "tests/templates/missing.txt".to_string(),
                        "Source does not exist".to_string(),
                    )],
                },
            );
        })
    }

    #[test]
    fn test_filesystem_loader_invalid_encoding() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let error = loader.get_template(py, "invalid.txt").unwrap().unwrap_err();

            assert_eq!(
                error.to_string(),
                "UnicodeError: Could not open \"tests/templates/invalid.txt\" with UTF-8 encoding."
            );
        })
    }
}
