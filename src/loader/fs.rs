//! File system configuration loader (`fs` feature).
//!
//! * Supported schema: `fs` and `file`  
//! *
//!
//! ### Example
//! ```rust
//! use std::{fs, collections::HashMap};
//! use tempdir::TempDir;
//! use plugx_config::loader::{ConfigurationLoader, fs::{ConfigurationLoaderFs, SoftErrorsFs}};
//! use url::Url;
//!
//! // Create a temporary directory containing `foo.json`, `bar.yaml`, and `baz.toml`:
//! let tmp_dir = TempDir::new("fs-example").unwrap();
//! let foo = tmp_dir.path().join("foo.json");
//! fs::write(&foo, "{\"hello\": \"world\"}").unwrap();
//! let bar = tmp_dir.path().join("bar.yaml");
//! fs::write(&bar, "hello: world").unwrap();
//! let baz = tmp_dir.path().join("baz.toml");
//! fs::write(&baz, "hello = \"world\"").unwrap();
//! let url = Url::try_from(format!("file://{}", tmp_dir.path().to_str().unwrap()).as_str()).unwrap();
//!
//! let mut loader = ConfigurationLoaderFs::new();
//! // You could set some skippable errors here.
//! // For example if you're loading contents of one file that may potentially not exists:
//! // loader.add_skippable_error(SkippbaleErrorKind::NotFound)
//!
//! // Load all configurations inside directory:
//! let loaded = loader.load(&url, None, false).unwrap();
//! assert_eq!(loaded.len(), 3);
//! let (_, foo) = loaded.iter().find(|(plugin_name, _)| plugin_name == "foo").expect("`foo` plugin config");
//! assert_eq!(foo.maybe_format(), Some(&"json".to_string()));
//! let (_, bar) = loaded.iter().find(|(plugin_name, _)| plugin_name == "bar").expect("`bar` plugin config");
//! assert_eq!(bar.maybe_contents(), Some(&"hello: world".to_string()));
//!
//! // Only load `foo` and `bar`:
//! let whitelist = ["foo".into(), "bar".into()].to_vec();
//! let loaded = loader.load(&url, Some(&whitelist), false).unwrap();
//! assert_eq!(loaded.len(), 2);
//!
//! // Load just one file:
//! let qux = tmp_dir.path().join("qux.env");
//! fs::write(&qux, "hello=\"world\"").unwrap();
//! let url = Url::try_from(format!("file://{}", qux.to_str().unwrap()).as_str()).unwrap();
//! let loaded = loader.load(&url, None, false).unwrap();
//! assert_eq!(loaded.len(), 1);
//! ```
//!
//! See [loader] documentation to known how loaders work.

use crate::{
    entity::ConfigurationEntity,
    loader::{self, ConfigurationLoadError, ConfigurationLoader, SoftErrors},
};
use anyhow::anyhow;
use cfg_if::cfg_if;
use serde::Deserialize;
use std::{
    collections::HashMap,
    env::current_dir,
    fmt::Debug,
    fs, io,
    path::{Path, PathBuf},
};
use url::Url;

pub const NAME: &str = "File";
pub const SCHEME_LIST: &[&str] = &["fs", "file"];

/// Loads configurations from filesystem.
#[derive(Default, Clone, Debug)]
pub struct ConfigurationLoaderFs {
    options: ConfigurationLoaderFsOptions,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct ConfigurationLoaderFsOptions {
    strip_slash: Option<bool>,
    soft_errors: SoftErrors<SoftErrorsFs>,
}

impl ConfigurationLoaderFsOptions {
    pub fn contains(&self, error: io::ErrorKind) -> bool {
        SoftErrorsFs::try_from(error)
            .map(|error| self.soft_errors.contains(&error))
            .unwrap_or_default()
    }
}

/// Supported soft errors when loading filesystem contents.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum SoftErrorsFs {
    NotFound,
    PermissionDenied,
}

impl TryFrom<io::ErrorKind> for SoftErrorsFs {
    type Error = String;

    fn try_from(value: io::ErrorKind) -> Result<Self, Self::Error> {
        match value {
            io::ErrorKind::NotFound => Ok(Self::NotFound),
            io::ErrorKind::PermissionDenied => Ok(Self::PermissionDenied),
            _ => Err("Unhandled IO error".into()),
        }
    }
}

#[doc(hidden)]
impl ConfigurationLoaderFs {
    #[inline]
    pub fn get_plugin_name_and_format<P: AsRef<Path>>(path: P) -> Option<(String, String)> {
        Self::get_plugin_name(&path)
            .and_then(|name| Self::get_format(&path).map(|format| (name, format)))
    }

    #[inline]
    pub fn get_plugin_name<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .file_stem()
            .and_then(|name| name.to_str())
            .map(|name| name.to_lowercase())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
    }

    #[inline]
    pub fn get_format<P: AsRef<Path>>(path: P) -> Option<String> {
        path.as_ref()
            .extension()
            .and_then(|format| format.to_str())
            .map(|format| format.to_lowercase())
            .and_then(|format| {
                if format.is_empty() {
                    None
                } else {
                    Some(format)
                }
            })
    }

    #[inline]
    pub(super) fn get_entity_list(
        url: &Url,
        options: &ConfigurationLoaderFsOptions,
        maybe_whitelist: Option<&[String]>,
        skip_soft_errors: bool,
    ) -> Result<Vec<ConfigurationEntity>, ConfigurationLoadError> {
        let path = Self::url_to_path(url, options).map_err(|_| {
            ConfigurationLoadError::Other(anyhow!("Could not detect current working directory"))
        })?;
        if path.is_dir() {
            let list = match Self::get_directory_file_list(&path, maybe_whitelist) {
                Ok(list) => list,
                Err(error) => {
                    return if skip_soft_errors
                        && (options.soft_errors.skip_all() || options.contains(error.kind()))
                    {
                        cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::info!(path=?path, skip_error=true, "Could not load directory contents");
                            } else if #[cfg(feature = "logging")] {
                                log::info!("msg=\"Could not load directory contents\" path={path:?} skip_error=true");
                            }
                        }
                        Ok(Vec::new())
                    } else {
                        Err(ConfigurationLoadError::Load {
                            loader: NAME.to_string(),
                            url: url.clone(),
                            description: "load directory file list".to_string().into(),
                            source: error.into(),
                        })
                    }
                }
            };
            let mut plugins: HashMap<&String, &String> = HashMap::with_capacity(list.len());
            for (plugin_name, format, _) in list.iter() {
                if let Some(other_format) = plugins.get(plugin_name) {
                    let mut url = url.clone();
                    url.set_query(None);
                    return Err(ConfigurationLoadError::Duplicate {
                        loader: NAME.to_string().into(),
                        url,
                        plugin: plugin_name.to_string().into(),
                        format_1: other_format.to_string().into(),
                        format_2: format.to_string().into(),
                    });
                } else {
                    plugins.insert(plugin_name, format);
                }
            }
            Ok(list
                .into_iter()
                .map(|(plugin_name, format, path)| {
                    let mut url = url.clone();
                    url.set_path(path.to_str().expect("It was &str!"));
                    ConfigurationEntity::new(url, plugin_name, NAME).with_format(format)
                })
                .collect())
        } else if path.is_file() {
            if let Some((plugin_name, format)) = Self::get_plugin_name_and_format(&path) {
                if maybe_whitelist
                    .map(|whitelist| whitelist.contains(&plugin_name))
                    .unwrap_or(true)
                {
                    let entity = ConfigurationEntity::new(url.clone(), plugin_name, NAME)
                        .with_format(format);
                    Ok([entity].into())
                } else {
                    Ok(Vec::new())
                }
            } else if skip_soft_errors && options.soft_errors.skip_all() {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::info!(url=%url, skip_error=true, "Could not parse plugin name/format");
                    } else if #[cfg(feature = "logging")] {
                        log::info!(
                            "msg=\"Could not parse plugin name/format\" url={:?} skip_error=true",
                            url.to_string()
                        );
                    }
                }
                Ok(Vec::new())
            } else {
                Err(ConfigurationLoadError::InvalidUrl {
                    loader: NAME.to_string(),
                    url: url.to_string(),
                    source: anyhow!("Could not parse plugin name/format"),
                })
            }
        } else if path.exists() {
            if skip_soft_errors && options.soft_errors.skip_all() {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::info!(url=%url, skip_error=true, "URL is not pointing to a directory or regular file");
                    } else if #[cfg(feature = "logging")] {
                        log::info!(
                            "msg=\"URL is not pointing to a directory or regular file\" url={:?} skip_error=true",
                            url.to_string()
                        );
                    }
                }
                Ok(Vec::new())
            } else {
                Err(ConfigurationLoadError::InvalidUrl {
                    loader: NAME.to_string(),
                    url: url.to_string(),
                    source: anyhow!("URL is not pointing to a directory or regular file"),
                })
            }
        } else if skip_soft_errors && options.contains(io::ErrorKind::NotFound) {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::info!(url=%url, skip_error=true, "Could not find path");
                } else if #[cfg(feature = "logging")] {
                    log::info!(
                        "msg=\"Could not find path\" url={:?} skip_error=true",
                        url.to_string()
                    );
                }
            }
            Ok(Vec::new())
        } else {
            Err(ConfigurationLoadError::NotFound {
                loader: NAME.to_string(),
                url: url.clone(),
            })
        }
    }

    #[inline]
    pub fn get_directory_file_list<P: AsRef<Path>>(
        path: P,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<Vec<(String, String, PathBuf)>, io::Error> {
        Ok(fs::read_dir(path)?
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| {
                if let Some((plugin_name, format)) = Self::get_plugin_name_and_format(&path) {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::trace!(plugin=plugin_name, path=?path, "Detected configuration file");
                        } else if #[cfg(feature = "logging")] {
                            log::trace!("msg=\"Detected configuration file\" plugin={plugin_name:?} path={path:?}");
                        }
                    }
                    Some((plugin_name, format, path))
                } else {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::warn!(path=?path, "Could not parse plugin name/format");
                        } else if #[cfg(feature = "logging")] {
                            log::warn!("msg=\"Could not parse plugin name/format\" path={path:?}");
                        }
                    }
                    None
                }
            })
            .filter(|(plugin_name, _, _)| {
                maybe_whitelist
                    .map(|whitelist| whitelist.contains(plugin_name))
                    .unwrap_or(true)
            })
            .filter_map(|(plugin_name, format, path)| {
                if path.is_file() {
                    Some((plugin_name, format, path))
                } else {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::warn!(path=?path, "Path is not pointing to a regular file");
                        } else if #[cfg(feature = "logging")] {
                            log::warn!("msg=\"Path is not pointing to a regular file\" path={path:?}");
                        }
                    }
                    None
                }
            })
            .collect())
    }

    #[inline]
    pub fn read_entity_contents(
        entity: &mut ConfigurationEntity,
        options: &ConfigurationLoaderFsOptions,
    ) -> Result<(), io::Error> {
        fs::read_to_string(Self::url_to_path(entity.url(), options)?).map(|contents| {
            entity.set_contents(contents);
        })
    }

    #[inline]
    pub fn url_to_path(
        url: &Url,
        options: &ConfigurationLoaderFsOptions,
    ) -> Result<PathBuf, io::Error> {
        let url_path = if url.path() == "/" {
            let cwd = current_dir()?;
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(url=%url, cwd=?cwd,"set current working directory");
                } else if #[cfg(feature = "logging")] {
                    log::debug!("msg=\"Read file\" url=\"{url}\", cwd={cwd:?}");
                }
            }
            cwd
        } else if options.strip_slash.unwrap_or(false) && url.path().starts_with('/') {
            PathBuf::from(
                url.path()
                    .strip_prefix('/')
                    .expect("URL path with length > 1"),
            )
        } else {
            PathBuf::from(url.path())
        };
        let mut path = PathBuf::new();
        url_path.components().for_each(|component| {
            path.push(component);
        });
        dbg!(&url, &url_path, &path);
        Ok(path)
    }
}

impl ConfigurationLoaderFs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_soft_error(&mut self, error: SoftErrorsFs) {
        self.options.soft_errors.add_soft_error(error)
    }

    pub fn with_soft_error(mut self, error: SoftErrorsFs) -> Self {
        self.add_soft_error(error);
        self
    }

    fn get_options(
        &self,
        url: &Url,
    ) -> Result<ConfigurationLoaderFsOptions, ConfigurationLoadError> {
        loader::deserialize_query_string::<ConfigurationLoaderFsOptions>(NAME, url).map(
            |mut options| {
                if let Some(soft_errors) = self.options.soft_errors.maybe_soft_error_list() {
                    soft_errors
                        .iter()
                        .for_each(|soft_error| options.soft_errors.add_soft_error(*soft_error))
                }
                options
            },
        )
    }
}

impl ConfigurationLoader for ConfigurationLoaderFs {
    fn name(&self) -> String {
        NAME.into()
    }

    /// In this case "fs" and "file".
    fn scheme_list(&self) -> Vec<String> {
        SCHEME_LIST.iter().cloned().map(String::from).collect()
    }

    fn load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, ConfigurationEntity)>, ConfigurationLoadError> {
        let options = self.get_options(url)?;
        let mut entity_list =
            Self::get_entity_list(url, &options, maybe_whitelist, skip_soft_errors)?;
        entity_list.iter_mut().try_for_each(|entity| {
            match Self::read_entity_contents(entity, &options) {
                Ok(_) => {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::trace!(
                                url=%entity.url(),
                                contents=entity
                                    .maybe_contents()
                                    .expect("Contents is set inside `utils::read_entity_contents`"),
                                "Read configuration file"
                            );
                        } else if #[cfg(feature = "logging")] {
                            log::trace!(
                                "msg=\"Read configuration file\" url={:?} contents={:?}",
                                entity.url().to_string(),
                                entity.maybe_contents().expect("Contents is set inside `utils::read_entity_contents`")
                            );
                        }
                    }
                    Ok(())
                },
                Err(error) => {
                    if skip_soft_errors && (options.soft_errors.skip_all() || options.contains(error.kind())) {
                        cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::info!(
                                    path=entity.url().path(),
                                    skip_error=true,
                                    "Could not read contents of file"
                                );
                            } else if #[cfg(feature = "logging")] {
                                log::info!(
                                    "msg=\"Could not read contents of file\" path={:?} skip_error=true",
                                    entity.url().path()
                                );
                            }
                        }
                        Ok(())
                    } else {
                        Err(ConfigurationLoadError::Load {
                            loader: NAME.to_string(),
                            url: entity.url().clone(),
                            description: "read contents of file".to_string().into(),
                            source: error.into(),
                        })
                    }
                }
            }
        })?;
        let result = entity_list
            .into_iter()
            // Maybe we have skipped soft errors in above:
            .filter(|entity| entity.maybe_contents().is_some())
            .map(|entity| (entity.plugin_name().clone(), entity))
            .collect();
        Ok(result)
    }
}
