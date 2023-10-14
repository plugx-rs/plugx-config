//! File system configuration loader.
//!
//! This is only usable if you enabled `fs` Cargo feature.
//!
//! ### Example
//! ```rust
//! use std::{fs, collections::HashMap};
//! use tempdir::TempDir;
//! use plugx_config::loader::{ConfigurationLoader, fs::{ConfigurationLoaderFs, SkippbaleErrorKind}};
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
//! let modifier = |url: &Url, loaded: &mut HashMap<String, _>| {
//!     // Modify loaded configuration if needed
//!     Ok(())
//! };
//! loader.set_modifier(Box::new(modifier)); // Note that setting a modifier is optional
//!
//! // Load all configurations inside directory:
//! let loaded = loader.try_load(&url, None).unwrap();
//! assert!(loaded.contains_key("foo") && loaded.contains_key("bar") && loaded.contains_key("baz"));
//! let foo = loaded.get("foo").unwrap();
//! assert_eq!(foo.maybe_format(), Some(&"json".to_string()));
//! let bar = loaded.get("bar").unwrap();
//! assert_eq!(bar.maybe_contents(), Some(&"hello: world".to_string()));
//!
//! // Only load `foo` and `bar`:
//! let whitelist = ["foo".into(), "bar".into()].to_vec();
//! let loaded = loader.try_load(&url, Some(&whitelist)).unwrap();
//! assert!(!loaded.contains_key("baz"));
//!
//! // Load just one file:
//! let qux = tmp_dir.path().join("qux.env");
//! fs::write(&qux, "hello=\"world\"").unwrap();
//! let url = Url::try_from(format!("file://{}", qux.to_str().unwrap()).as_str()).unwrap();
//! let loaded = loader.try_load(&url, None).unwrap();
//! assert!(loaded.contains_key("qux"));
//! ```
//!
//! See [loader] documentation to known how loaders work.

use crate::{
    entity::ConfigurationEntity,
    loader::{self, BoxedLoaderModifierFn, ConfigurationLoadError, ConfigurationLoader},
};
use anyhow::anyhow;
use cfg_if::cfg_if;
use serde::Deserialize;
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
    fs, io,
    path::PathBuf,
};
use url::Url;

const NAME: &str = "File";
const SCHEME_LIST: &[&str] = &["fs", "file"];

/// Loads configurations from filesystem.
#[derive(Default)]
pub struct ConfigurationLoaderFs {
    options: ConfigurationLoaderFsOptions,
    maybe_modifier: Option<BoxedLoaderModifierFn>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct ConfigurationLoaderFsOptions {
    skippable: Vec<SkippbaleErrorKind>,
}

impl ConfigurationLoaderFsOptions {
    pub fn contains(&self, error: io::ErrorKind) -> bool {
        SkippbaleErrorKind::try_from(error)
            .map(|error| self.skippable.contains(&error))
            .unwrap_or_default()
    }
}

/// Supported skippable errors when loading filesystem contents.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SkippbaleErrorKind {
    WouldBlock,
    Interrupted,
    NotFound,
    PermissionDenied,
    TimedOut,
}

impl TryFrom<io::ErrorKind> for SkippbaleErrorKind {
    type Error = String;

    fn try_from(value: io::ErrorKind) -> Result<Self, Self::Error> {
        match value {
            io::ErrorKind::WouldBlock => Ok(Self::WouldBlock),
            io::ErrorKind::Interrupted => Ok(Self::Interrupted),
            io::ErrorKind::NotFound => Ok(Self::NotFound),
            io::ErrorKind::PermissionDenied => Ok(Self::PermissionDenied),
            io::ErrorKind::TimedOut => Ok(Self::TimedOut),
            _ => Err("Unhandled IO error".into()),
        }
    }
}

impl Debug for ConfigurationLoaderFs {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationLoaderEnv")
            .field("options", &self.options)
            .finish()
    }
}

#[doc(hidden)]
pub mod utils {
    use super::*;
    use std::path::Path;

    #[inline]
    pub fn get_plugin_name_and_format<P: AsRef<Path>>(path: P) -> Option<(String, String)> {
        get_plugin_name(&path).and_then(|name| get_format(&path).map(|format| (name, format)))
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
    ) -> Result<Vec<ConfigurationEntity>, ConfigurationLoadError> {
        let path = PathBuf::from(url.path());
        if path.is_dir() {
            let list = get_directory_file_list(path, maybe_whitelist).map_err(|error| {
                let skippable = options.contains(error.kind());
                ConfigurationLoadError::Load {
                    loader: NAME.to_string(),
                    url: url.to_string(),
                    description: "load directory file list".to_string(),
                    source: error.into(),
                    skippable,
                }
            })?;
            let mut plugins: HashMap<&String, &String> = HashMap::with_capacity(list.len());
            for (plugin_name, format, _) in list.iter() {
                if let Some(other_format) = plugins.get(plugin_name) {
                    return Err(ConfigurationLoadError::Duplicate {
                        loader: NAME.to_string(),
                        url: url.to_string(),
                        plugin: plugin_name.to_string(),
                        format_1: other_format.to_string(),
                        format_2: format.to_string(),
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
            if let Some((plugin_name, format)) = get_plugin_name_and_format(&path) {
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
            } else {
                Err(ConfigurationLoadError::InvalidUrl {
                    loader: NAME.to_string(),
                    url: url.to_string(),
                    error: anyhow!("Could not parse plugin name/format"),
                })
            }
        } else if path.exists() {
            Err(ConfigurationLoadError::InvalidUrl {
                loader: NAME.to_string(),
                url: url.to_string(),
                error: anyhow!("This is not pointing to a directory or regular file"),
            })
        } else if options.contains(io::ErrorKind::NotFound) {
            Err(ConfigurationLoadError::Load {
                loader: NAME.to_string(),
                url: url.to_string(),
                description: "find path".to_string(),
                source: anyhow!(io::Error::from(io::ErrorKind::NotFound)),
                skippable: true,
            })
        } else {
            Err(ConfigurationLoadError::NotFound {
                loader: NAME.to_string(),
                url: url.to_string(),
            })
        }
    }

    #[inline]
    pub fn get_directory_file_list(
        path: PathBuf,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<Vec<(String, String, PathBuf)>, io::Error> {
        Ok(fs::read_dir(path)?
            .filter_map(|maybe_entry| maybe_entry.ok())
            .map(|entry| entry.path())
            .filter_map(|path| {
                if let Some((plugin_name, format)) = get_plugin_name_and_format(&path) {
                    Some((plugin_name, format, path))
                } else {
                    cfg_if! {
                        if #[cfg(feature = "tracing")] {
                            tracing::warn!(path = ?path, "Could not parse plugin name/format");
                        } else if #[cfg(feature = "logging")] {
                            log::warn!("path={path:?} message=\"Could not parse plugin name/format\"");
                        }
                    }
                    None
                }
            })
            .filter(|(plugin_name, _, _)| {
                maybe_whitelist.map(|whitelist| whitelist.contains(plugin_name)).unwrap_or(true)
            })
            .filter_map(|(plugin_name, format, path)| {
                if path.is_file() {
                    Some((plugin_name, format, path))
                } else {
                    cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::warn!(path = ?path, "This is not a regular file");
                            } else if #[cfg(feature = "logging")] {
                                log::warn!("path={path:?} message=\"This is not a regular file\"");
                            }
                        }
                    None
                }
            })
            .collect())
    }

    #[inline]
    pub fn read_entity_contents(entity: &mut ConfigurationEntity) -> Result<(), io::Error> {
        fs::read_to_string(PathBuf::from(entity.url().path())).map(|contents| {
            entity.set_contents(contents);
        })
    }
}

impl ConfigurationLoaderFs {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_skippable_error(&mut self, error: SkippbaleErrorKind) {
        self.options.skippable.push(error)
    }

    pub fn with_skippable_error(mut self, error: SkippbaleErrorKind) -> Self {
        self.add_skippable_error(error);
        self
    }

    pub fn set_modifier(&mut self, modifier: BoxedLoaderModifierFn) {
        self.maybe_modifier = Some(modifier)
    }

    pub fn with_modifier(mut self, modifier: BoxedLoaderModifierFn) -> Self {
        self.set_modifier(modifier);
        self
    }

    fn get_options(
        &self,
        url: &Url,
    ) -> Result<ConfigurationLoaderFsOptions, ConfigurationLoadError> {
        loader::deserialize_query_string::<ConfigurationLoaderFsOptions>(NAME, url).map(
            |mut options| {
                options
                    .skippable
                    .append(&mut self.options.skippable.clone());
                options
            },
        )
    }
}

impl ConfigurationLoader for ConfigurationLoaderFs {
    fn name(&self) -> &'static str {
        NAME
    }

    fn scheme_list(&self) -> Vec<String> {
        SCHEME_LIST.iter().cloned().map(String::from).collect()
    }

    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let options = self.get_options(url)?;
        let mut entity_list = utils::get_entity_list(url, &options, maybe_whitelist)?;
        entity_list.iter_mut().try_for_each(|entity| {
            utils::read_entity_contents(entity).map_err(|error| {
                let skippable = options.contains(error.kind());
                ConfigurationLoadError::Load {
                    loader: NAME.to_string(),
                    url: entity.url().to_string(),
                    description: "read entity file".to_string(),
                    source: error.into(),
                    skippable,
                }
            })
        })?;
        let mut result = entity_list
            .into_iter()
            .map(|entity| (entity.plugin_name().clone(), entity))
            .collect();
        if let Some(ref modifier) = self.maybe_modifier {
            // TODO: logging
            modifier(url, &mut result)?;
        }
        Ok(result)
    }
}
