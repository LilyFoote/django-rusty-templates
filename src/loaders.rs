use std::collections::HashMap;
use std::path::{Path, PathBuf};

use cached::proc_macro::cached;
use encoding_rs::Encoding;
use pyo3::exceptions::PyUnicodeError;
use pyo3::prelude::*;
use sugar_path::SugarPath;

use crate::template::django_rusty_templates::{EngineData, Template};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoaderError {
    pub tried: Vec<(String, String)>,
}

fn abspath(path: &Path) -> Option<PathBuf> {
    match path.as_os_str().is_empty() {
        false => std::path::absolute(path).map(|p| p.normalize()).ok(),
        true => std::env::current_dir().ok(),
    }
}

fn safe_join(directory: &Path, template_name: &str) -> Option<PathBuf> {
    let final_path = abspath(&directory.join(template_name))?;
    let directory = abspath(directory)?;
    if final_path.starts_with(directory) {
        Some(final_path)
    } else {
        None
    }
}

#[cached(
    size = 128,          // Cache size
    result = true,       // Cache Result type
    key = "String",      // Use owned String as key
    convert = r##"{ dirname.to_string() }"## // Convert &str to String
)]
fn get_app_template_dirs(py: Python<'_>, dirname: &str) -> Result<Vec<String>, PyErr> {
    // Import django.apps
    let apps_module = PyModule::import(py, "django.apps")?;

    // Get the 'apps' object
    let apps = apps_module.getattr("apps")?;

    // Call get_app_configs() on apps
    let app_configs = apps.call_method0("get_app_configs")?;

    // Convert to iterator and process each app_config
    let app_configs_iter = app_configs.try_iter()?;

    let mut template_dirs = Vec::new();
    for app_config_result in app_configs_iter {
        match app_config_result {
            Ok(app_config) => {
                let path = app_config.getattr("path")?;
                if !path.is_none() {
                    let path_str: String = path.extract()?;
                    let template_path = PathBuf::from(path_str).join(dirname);
                    if template_path.is_dir() {
                        template_dirs.push(template_path.to_string_lossy().into_owned());
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }

    Ok(template_dirs)
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
        engine: &EngineData,
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
            return Ok(Template::new(py, &contents, path, engine));
        }
        Err(LoaderError { tried })
    }
}

pub struct AppDirsLoader {
    encoding: &'static Encoding,
}

impl AppDirsLoader {
    pub fn new(encoding: &'static Encoding) -> Self {
        Self { encoding }
    }

    fn get_template(
        &self,
        py: Python<'_>,
        template_name: &str,
        engine: &EngineData,
    ) -> Result<PyResult<Template>, LoaderError> {
        let dirs_result = get_app_template_dirs(py, "templates");
        let dirs = match dirs_result {
            Ok(dirs) => dirs,
            Err(e) => Vec::new(),
        };

        let filesystem_loader = FileSystemLoader::new(dirs, self.encoding);

        filesystem_loader.get_template(py, template_name, engine)
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
        engine: &EngineData,
    ) -> Result<PyResult<Template>, LoaderError> {
        match self.cache.get(template_name) {
            Some(Ok(template)) => Ok(Ok((*template).clone())),
            Some(Err(e)) => Err(e.clone()),
            None => {
                let mut tried = Vec::new();
                for loader in &mut self.loaders {
                    match loader.get_template(py, template_name, engine) {
                        Ok(Ok(template)) => {
                            self.cache
                                .insert(template_name.to_string(), Ok(template.clone()));
                            return Ok(Ok(template));
                        }
                        Ok(Err(e)) => return Ok(Err(e)),
                        Err(mut e) => tried.append(&mut e.tried),
                    }
                }
                let error = LoaderError { tried };
                self.cache
                    .insert(template_name.to_string(), Err(error.clone()));
                Err(error)
            }
        }
    }
}

pub struct LocMemLoader {
    templates: HashMap<String, String>,
}

impl LocMemLoader {
    pub fn new(templates: HashMap<String, String>) -> Self {
        Self { templates }
    }

    fn get_template(
        &self,
        py: Python<'_>,
        template_name: &str,
        engine: &EngineData,
    ) -> Result<PyResult<Template>, LoaderError> {
        if let Some(contents) = self.templates.get(template_name) {
            Ok(Template::new(
                py,
                contents,
                PathBuf::from(template_name),
                engine,
            ))
        } else {
            Err(LoaderError {
                tried: vec![(
                    template_name.to_string(),
                    "Source does not exist".to_string(),
                )],
            })
        }
    }
}

pub struct ExternalLoader {}

impl ExternalLoader {
    fn get_template(
        &self,
        _py: Python<'_>,
        _template_name: &str,
        _engine: &EngineData,
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
        engine: &EngineData,
    ) -> Result<PyResult<Template>, LoaderError> {
        match self {
            Self::FileSystem(loader) => loader.get_template(py, template_name, engine),
            Self::AppDirs(loader) => loader.get_template(py, template_name, engine),
            Self::Cached(loader) => loader.get_template(py, template_name, engine),
            Self::LocMem(loader) => loader.get_template(py, template_name, engine),
            Self::External(loader) => loader.get_template(py, template_name, engine),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use quickcheck::quickcheck;

    fn setup_django(py: Python<'_>) {
        // Import the os module and set the DJANGO_SETTINGS_MODULE environment variable
        let os_module = PyModule::import(py, "os").unwrap();
        let environ = os_module.getattr("environ").unwrap();
        environ
            .call_method(
                "setdefault",
                ("DJANGO_SETTINGS_MODULE", "tests.settings"),
                None,
            )
            .unwrap();

        // Import the django module and call django.setup()
        let django_module = PyModule::import(py, "django").unwrap();
        django_module.call_method0("setup").unwrap();
    }

    #[test]
    fn test_filesystem_loader() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let template = loader
                .get_template(py, "basic.txt", &engine)
                .unwrap()
                .unwrap();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/basic.txt");
            assert_eq!(template.filename.unwrap(), expected);
        })
    }

    #[test]
    fn test_filesystem_loader_missing_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let error = loader.get_template(py, "missing.txt", &engine).unwrap_err();

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
            let engine = EngineData::empty();
            let loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);
            let error = loader
                .get_template(py, "invalid.txt", &engine)
                .unwrap()
                .unwrap_err();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/invalid.txt");
            assert_eq!(
                error.to_string(),
                format!("UnicodeError: Could not open {expected:?} with UTF-8 encoding.")
            );
        })
    }

    #[test]
    fn test_cached_loader() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Helper to check cache contents
            let verify_cache = |cache: &HashMap<String, Result<Template, LoaderError>>,
                                key: &str,
                                expected_path: &Path| {
                if let Some(Ok(cached_template)) = cache.get(key) {
                    assert_eq!(cached_template.filename.as_ref().unwrap(), expected_path);
                } else {
                    panic!("Expected '{}' to be in cache.", key);
                }
            };

            let engine = EngineData::empty();

            // Create a FileSystemLoader for the CachedLoader
            let filesystem_loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);

            // Wrap the FileSystemLoader in a CachedLoader
            let mut cached_loader = CachedLoader::new(vec![Loader::FileSystem(filesystem_loader)]);

            // Load a template via the CachedLoader
            let template = cached_loader
                .get_template(py, "basic.txt", &engine)
                .expect("Failed to load template")
                .expect("Template file could not be read");

            // Verify the template filename
            let mut expected_path =
                std::env::current_dir().expect("Failed to get current directory");
            expected_path.push("tests/templates/basic.txt");
            assert_eq!(template.filename.unwrap(), expected_path);

            // Verify the cache state after first load
            assert_eq!(cached_loader.cache.len(), 1);
            verify_cache(&cached_loader.cache, "basic.txt", &expected_path);

            // Load the same template again via the CachedLoader
            let template = cached_loader
                .get_template(py, "basic.txt", &engine)
                .expect("Failed to load template")
                .expect("Template file could not be read");

            // Verify the template filename again
            assert_eq!(template.filename.unwrap(), expected_path);

            // Verify the cache state remains consistent
            assert_eq!(cached_loader.cache.len(), 1);
            verify_cache(&cached_loader.cache, "basic.txt", &expected_path);
        });
    }

    #[test]
    fn test_cached_loader_missing_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let filesystem_loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);

            let mut cached_loader = CachedLoader::new(vec![Loader::FileSystem(filesystem_loader)]);
            let error = cached_loader
                .get_template(py, "missing.txt", &engine)
                .unwrap_err();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/missing.txt");
            let expected_err = LoaderError {
                tried: vec![(
                    expected.display().to_string(),
                    "Source does not exist".to_string(),
                )],
            };
            assert_eq!(error, expected_err);

            let cache = &cached_loader.cache;
            assert_eq!(
                cache.get("missing.txt").unwrap().as_ref().unwrap_err(),
                &expected_err
            );

            let error = cached_loader
                .get_template(py, "missing.txt", &engine)
                .unwrap_err();
            assert_eq!(error, expected_err);
        })
    }

    #[test]
    fn test_cached_loader_invalid_encoding() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let filesystem_loader =
                FileSystemLoader::new(vec!["tests/templates".to_string()], encoding_rs::UTF_8);

            let mut cached_loader = CachedLoader::new(vec![Loader::FileSystem(filesystem_loader)]);
            let error = cached_loader
                .get_template(py, "invalid.txt", &engine)
                .unwrap()
                .unwrap_err();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/invalid.txt");
            assert_eq!(
                error.to_string(),
                format!("UnicodeError: Could not open {expected:?} with UTF-8 encoding.")
            );
        })
    }

    #[test]
    fn test_locmem_loader() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let mut templates: HashMap<String, String> = HashMap::new();
            templates.insert("index.html".to_string(), "index".to_string());

            let loader = LocMemLoader::new(templates);

            let template = loader
                .get_template(py, "index.html", &engine)
                .unwrap()
                .unwrap();
            assert_eq!(template.template, "index".to_string());
            assert_eq!(template.filename.unwrap(), PathBuf::from("index.html"));
        });
    }

    #[test]
    fn test_locmem_loader_missing_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            let engine = EngineData::empty();
            let templates: HashMap<String, String> = HashMap::new();

            let loader = LocMemLoader::new(templates);

            let error = loader.get_template(py, "index.html", &engine).unwrap_err();
            assert_eq!(
                error,
                LoaderError {
                    tried: vec![(
                        "index.html".to_string(),
                        "Source does not exist".to_string(),
                    )],
                },
            );
        })
    }

    #[test]
    fn test_appdirs_loader() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Setup Django
            setup_django(py);

            let engine = EngineData::empty();
            let loader = AppDirsLoader::new(encoding_rs::UTF_8);
            let template = loader
                .get_template(py, "basic.txt", &engine)
                .unwrap()
                .unwrap();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/basic.txt");
            assert_eq!(template.filename.unwrap(), expected);
        })
    }

    #[test]
    fn test_appdirs_loader_missing_template() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Setup Django
            setup_django(py);

            let engine = EngineData::empty();
            let loader = AppDirsLoader::new(encoding_rs::UTF_8);
            let error = loader.get_template(py, "missing.txt", &engine).unwrap_err();

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
    fn test_appdirs_loader_invalid_encoding() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Setup Django
            setup_django(py);

            let engine = EngineData::empty();
            let loader = AppDirsLoader::new(encoding_rs::UTF_8);
            let error = loader
                .get_template(py, "invalid.txt", &engine)
                .unwrap()
                .unwrap_err();

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates/invalid.txt");
            assert_eq!(
                error.to_string(),
                format!("UnicodeError: Could not open {expected:?} with UTF-8 encoding.")
            );
        })
    }

    #[test]
    fn test_get_app_template_dirs() {
        pyo3::prepare_freethreaded_python();

        Python::with_gil(|py| {
            // Setup Django
            setup_django(py);

            let dirs = get_app_template_dirs(py, "templates").unwrap();
            assert_eq!(dirs.len(), 1);

            let mut expected = std::env::current_dir().unwrap();
            expected.push("tests/templates");
            assert_eq!(dirs[0], expected.to_str().unwrap());
        });
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

    #[test]
    fn test_safe_join_parent_and_empty_template_name() {
        let path = PathBuf::from("..");
        let joined = safe_join(&path, "").unwrap();
        let mut expected = std::env::current_dir().unwrap();
        expected.push("..");
        assert_eq!(joined, expected.normalize());
    }

    #[test]
    fn test_safe_join_matches_django_safe_join() {
        pyo3::prepare_freethreaded_python();

        fn matches(path: PathBuf, template_name: String) -> bool {
            Python::with_gil(|py| {
                let utils_os = PyModule::import(py, "django.utils._os").unwrap();
                let django_safe_join = utils_os.getattr("safe_join").unwrap();

                let joined = django_safe_join
                    .call1((&path, &template_name))
                    .map(|joined| joined.extract().unwrap_or_default())
                    .ok();
                joined == safe_join(&path, &template_name)
            })
        }
        quickcheck(matches as fn(PathBuf, String) -> bool)
    }
}
