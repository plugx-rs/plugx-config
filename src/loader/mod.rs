//! Configuration loader trait and implementations.
//!
//! A configuration loader only loads contents of one or more plugins. No parsing is done here.
//! The result is just a hashmap with plugin names (in lowercase) as keys and [ConfigurationEntity]
//! as values.
//! A loader also should try to set contents format for each plugin.
//! For example [fs] (that implements [ConfigurationLoader]) that loads
//! configurations from filesystem, guesses content formats from file extensions.
//!
//! Main method of [ConfigurationLoader] trait is `try_load` that accepts a URL and maybe a
//! whitelist of plugin names. It can parse the URL to detect and validate its own options.
//! For example [mod@env] that loads configuration from environment-variables
//! accepts a URL like `env://?prefix=MY_APP_NAME` and [fs] accepts a URL
//! like `file:///path/to/a/file.json?skippable[0]=notfound` (`skippable` is a list and should
//! contain error kinds that we want to skip if they happen).
//!
//! Note that generally you do not need to implement [ConfigurationLoader] for your own structs and
//! provided [closure] lets you implement your own loader with just one [Fn] closure.

use crate::entity::ConfigurationEntity;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, fmt::Debug};
use url::Url;

pub mod closure;
#[cfg(feature = "env")]
pub mod env;
#[cfg(feature = "fs")]
pub mod fs;

/// A modifier [Fn] that modifies loaded configurations (if needed).
pub type BoxedLoaderModifierFn = Box<
    dyn Fn(&Url, &mut HashMap<String, ConfigurationEntity>) -> Result<(), ConfigurationLoadError>
        + Send
        + Sync,
>;

/// Loaded error type.
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationLoadError {
    /// An entity could not be found.
    #[error("{loader} configuration loader could not found configuration `{url}`")]
    NotFound { loader: String, url: String },
    /// Did not have enough permissions to read the contents.
    #[error("{loader} configuration loader has no access to load configuration from `{url}`")]
    NoAccess { loader: String, url: String },
    /// Got timeout when reading the contents.
    #[error(
        "{loader} configuration loader reached timeout `{timeout_in_seconds}s` to load `{url}`"
    )]
    Timeout {
        loader: String,
        url: String,
        timeout_in_seconds: usize,
    },
    /// The provided URL is invalid.
    #[error("{loader} configuration loader got invalid source `{url}`")]
    InvalidUrl {
        loader: String,
        url: String,
        #[source]
        error: anyhow::Error,
    },
    /// Could not found URL scheme.
    #[error("Could not found configuration loader for scheme {scheme}")]
    UrlSchemeNotFound { scheme: String },
    /// Found more than one configuration with two different formats (extensions) for the same plugin.
    #[error("{loader} configuration loader found duplicate configurations `{url}/{plugin}.({format_1}|{format_2})`")]
    Duplicate {
        loader: String,
        url: String,
        plugin: String,
        format_1: String,
        format_2: String,
    },
    /// Could not load the configuration.
    ///
    /// note that `skippable` key is very important. You might want to detect your own options from
    /// provided [Url] and sometimes make some errors skippable based on you detected options.    
    #[error("{loader} configuration loader could not {description} `{url}`")]
    Load {
        loader: String,
        url: String,
        description: String,
        source: anyhow::Error,
        skippable: bool,
    },
    #[error("Could not acquire lock for configuration loader with url `{url}`")]
    AcquireLock { url: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// A trait to load configurations for one or more plugins.
pub trait ConfigurationLoader: Send + Sync + Debug {
    /// Name of the loader (for logging purposes).
    fn name(&self) -> &'static str;

    /// List of URL schemes that this loader supports.
    ///
    /// Different URL may be assigned to this loader by their scheme value.
    fn scheme_list(&self) -> Vec<String>;

    /// Main method that actually loads configurations.
    ///
    /// * Checks the `source` and detects its own options from it.
    /// * Checks whitelist to load just provided plugins configurations.
    /// * Attempts to load configurations.
    /// * Tries to set format for each [ConfigurationEntity].
    fn try_load(
        &self,
        source: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError>;
}

#[cfg(feature = "qs")]
/// Checks query-string part of URL and tries to deserialize it to provided type.
///
/// See supported syntax at [serde_qs].
/// This function is only usable if `qs` Cargo feature is enabled.
pub fn deserialize_query_string<T: DeserializeOwned>(
    loader_name: &'static str,
    url: &Url,
) -> Result<T, ConfigurationLoadError> {
    serde_qs::from_str(url.query().unwrap_or_default()).map_err(|error| {
        ConfigurationLoadError::InvalidUrl {
            loader: loader_name.to_string(),
            error: error.into(),
            url: url.to_string(),
        }
    })
}

impl ConfigurationLoadError {
    pub fn is_skippable(&self) -> bool {
        if let Self::Load { skippable, .. } = self {
            *skippable
        } else {
            false
        }
    }
}
