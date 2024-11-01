use std::collections::HashMap;
use std::path::{Path, PathBuf};

use encoding_rs::Encoding;
use pyo3::exceptions::PyUnicodeError;
use pyo3::prelude::*;
use sugar_path::SugarPath;

use crate::template::django_rusty_templates::Template;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderError {
    pub tried: Vec<(String, String)>,
}

fn absolute(path: &Path) -> Option<PathBuf> {
    match path.as_os_str().is_empty() {
        false => std::path::absolute(path).ok(),
        true => std::env::current_dir().ok(),
    }
}

fn safe_join(directory: &Path, template_name: &str) -> Option<PathBuf> {
    let final_path = absolute(&directory.join(template_name))?.normalize();
    let directory = absolute(directory)?;
    if final_path.starts_with(directory) {
        Some(final_path)
    } else {
        None
    }
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

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/basic.txt");
            assert_eq!(template.filename.unwrap(), expected);
        })
    }

    #[test]
    fn test_filesystem_loader_missing_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let error = loader.get_template(py, "missing.txt").unwrap_err();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/missing.txt");
            assert_eq!(
                error,
                LoaderError {
                    tried: vec![(
                        expected.display().to_string(),
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

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/invalid.txt");
            assert_eq!(
                error.to_string(),
                format!("UnicodeError: Could not open {expected:?} with UTF-8 encoding.")
            );
        })
    }

    #[test]
    fn test_safe_join_absolute() {
        let path = PathBuf::from("/abc/");
        let joined = safe_join(&path, "def").unwrap();
        assert_eq!(joined, PathBuf::from("/abc/def"));
    }

    #[test]
    fn test_safe_join_relative() {
        let path = PathBuf::from("abc");
        let joined = safe_join(&path, "def").unwrap();
        let mut expected = std::env::current_dir().unwrap();
        expected.push("abc/def");
        assert_eq!(joined, expected);
    }

    #[test]
    fn test_safe_join_absolute_starts_with_sep() {
        let path = PathBuf::from("/abc/");
        let joined = safe_join(&path, "/def");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_relative_starts_with_sep() {
        let path = PathBuf::from("abc");
        let joined = safe_join(&path, "/def");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_absolute_parent() {
        let path = PathBuf::from("/abc/");
        let joined = safe_join(&path, "../def");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_relative_parent() {
        let path = PathBuf::from("abc");
        let joined = safe_join(&path, "../def");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_absolute_parent_starts_with_sep() {
        let path = PathBuf::from("/abc/");
        let joined = safe_join(&path, "/../def");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_relative_parent_starts_with_sep() {
        let path = PathBuf::from("abc");
        let joined = safe_join(&path, "/../def");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_django_example() {
        let path = PathBuf::from("/dir");
        let joined = safe_join(&path, "/../d");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_django_example_variant() {
        let path = PathBuf::from("/dir");
        let joined = safe_join(&path, "/../directory");
        assert_eq!(joined, None);
    }

    #[test]
    fn test_safe_join_empty_path() {
        let path = PathBuf::from("");
        let joined = safe_join(&path, "directory").unwrap();
        let mut expected = std::env::current_dir().unwrap();
        expected.push("directory");
        assert_eq!(joined, expected);
    }

    #[test]
    fn test_safe_join_empty_path_and_template_name() {
        let path = PathBuf::from("");
        let joined = safe_join(&path, "").unwrap();
        let expected = std::env::current_dir().unwrap();
        assert_eq!(joined, expected);
    }
}
