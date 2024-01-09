//! Configuration loader trait and implementations.
//!
//! A configuration loader only loads contents of configurations for plugins. No parsing is done here.
//! The result is just a `Vec<(String, ConfigurationEntity)>` with plugin names (in lowercase) as first element
//! and [ConfigurationEntity] as values for each plugin.
//! A loader also should try to set contents format for each plugin. For example [mod@fs] loader (that loads
//! configurations from filesystem) guesses content formats from file extensions.
//!
//! Every configuration loader (every implementor of [ConfigurationLoader]) accepts a URL and maybe a
//! whitelist of plugin names. It can parse the URL to detect and validate its own options. For example [mod@env] (that
//! loads configuration from environment-variables) accepts a URL like `env://?prefix=MY_APP_NAME`.
//!
//! Also a Loader can be mark some errors skippable! For more information refer to documentation of the loader itself.
//!
//! Note that generally you do not need to implement [ConfigurationLoader], provided [mod@closure] lets you make your
//! own loader with just one [Fn] closure.

use crate::entity::ConfigurationEntity;
use serde::de::{Error, IntoDeserializer, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;
use url::Url;

pub mod closure;
#[cfg(feature = "env")]
pub mod env;
#[cfg(feature = "fs")]
pub mod fs;

/// Load error type.
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationLoadError {
    /// An entity could not be found.
    #[error("{loader} configuration loader could not found configuration `{url}`")]
    NotFound { loader: String, url: Url },
    /// Did not have enough permissions to read the contents.
    #[error("{loader} configuration loader has no access to load configuration from `{url}`")]
    NoAccess { loader: String, url: Url },
    /// Got timeout when reading the contents.
    #[error(
        "{loader} configuration loader reached timeout `{timeout_in_seconds}s` to load `{url}`"
    )]
    Timeout {
        loader: String,
        url: Url,
        timeout_in_seconds: usize,
    },
    /// The provided URL is invalid.
    #[error("{loader} configuration loader got invalid URL `{url}`")]
    InvalidUrl {
        loader: String,
        url: String,
        source: anyhow::Error,
    },
    /// Could not found URL scheme.
    #[error("Could not found configuration loader for scheme {scheme}")]
    UrlSchemeNotFound { scheme: String },
    /// Found more than one configuration with two different formats (extensions) for the same plugin.
    #[error("{loader} configuration loader found duplicate configurations `{url}/{plugin}.({format_1}|{format_2})`")]
    Duplicate {
        loader: String,
        url: Url,
        plugin: String,
        format_1: String,
        format_2: String,
    },
    /// Could not load the configuration.
    #[error("{loader} configuration loader could not {description} `{url}`")]
    Load {
        loader: String,
        url: Url,
        description: String,
        source: anyhow::Error,
    },
    #[error("Could not found a loader that supports URL scheme `{scheme}` in given URL `{url}`")]
    LoaderNotFound { scheme: String, url: Url },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Soft errors deserializer wrapper for URL query strings.
///
/// ### Example
/// ```
///
/// use plugx_config::{
///     loader::{SoftErrors, deserialize_query_string},
///     ext::{url::Url, serde::Deserialize},
/// };
///
/// // Define an enum for your own errors
/// #[derive(Debug, PartialEq, Deserialize)]
/// enum MySoftErrors {
///     NotFound,
///     Permission,
///     Empty,
/// }
///
/// // Define a struct for your own options
/// // Include your own errors inside your options
/// #[derive(Debug, PartialEq, Deserialize)]
/// struct MyOptions {
///     // The value should be string `all` or dot seperated values of `MySoftErrors`
///     skip_errors: SoftErrors<MySoftErrors>,
///     // Other options ...
/// }
///
/// // `deserialize_query_string` function needs loader name to generate a good descriptive error
/// let loader_name = "file-loader";
///
/// let url = Url::try_from("file://etc/config/file.toml?skip_errors=all").expect("Valid URL");
/// let options: MyOptions = deserialize_query_string(loader_name, &url).expect("Parse options");
/// assert_eq!(options, MyOptions{skip_errors: SoftErrors::new_all()});
/// assert!(options.skip_errors.skip_all());
///
/// let url = Url::try_from("file://etc/config/file.toml?skip_errors=NotFound.Permission").expect("Valid URL");
/// let options: MyOptions = deserialize_query_string(loader_name, &url).expect("Parse options");
/// let skip_errors = options.skip_errors;
/// assert!(skip_errors.contains(&MySoftErrors::NotFound));
/// assert!(skip_errors.contains(&MySoftErrors::Permission));
/// assert!(!skip_errors.contains(&MySoftErrors::Empty));
/// assert!(!skip_errors.skip_all());
/// assert_eq!(
///     skip_errors.maybe_soft_error_list(),
///       Some(&Vec::from([MySoftErrors::NotFound, MySoftErrors::Permission]))
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SoftErrors<T> {
    All,
    List(Vec<T>),
}

struct SoftErrorsVisitor<T> {
    _marker: PhantomData<T>,
}

/// A trait to load configurations for one or more plugins.
pub trait ConfigurationLoader: Send + Sync + Debug {
    /// Name of the loader (for logging purposes).
    fn name(&self) -> String;

    /// List of URL schemes that this loader supports.
    ///
    /// Different URL may be assigned to this loader by their scheme value.
    fn scheme_list(&self) -> Vec<String>;

    /// Main method that actually loads configurations.
    ///
    /// * Checks the `url` and detects its own options from it.
    /// * Checks whitelist to load just provided plugins configurations.
    /// * Attempts to load configurations.
    /// * Tries to set format for each [ConfigurationEntity].
    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, ConfigurationEntity)>, ConfigurationLoadError>;
}

#[cfg(feature = "qs")]
/// Checks query-string part of URL and tries to deserialize it to provided type. (`qs` Cargo feature)
///
/// For usage example see [SoftErrors].
pub fn deserialize_query_string<T: serde::de::DeserializeOwned>(
    loader_name: impl AsRef<str>,
    url: &Url,
) -> Result<T, ConfigurationLoadError> {
    serde_qs::from_str(url.query().unwrap_or_default()).map_err(|error| {
        ConfigurationLoadError::InvalidUrl {
            loader: loader_name.as_ref().to_string(),
            source: error.into(),
            url: url.to_string(),
        }
    })
}

impl<'de, T: Deserialize<'de>> SoftErrors<T> {
    pub fn new_all() -> Self {
        Self::All
    }

    pub fn new_list() -> Self {
        Self::List(Vec::with_capacity(0))
    }

    pub fn skip_all(&self) -> bool {
        matches!(self, Self::All)
    }

    pub fn add_soft_error(&mut self, soft_error: T) {
        if let Self::List(soft_errors) = self {
            soft_errors.push(soft_error);
        }
    }

    pub fn with_soft_error(mut self, soft_error: T) -> Self {
        self.add_soft_error(soft_error);
        self
    }

    pub fn maybe_soft_error_list(&self) -> Option<&Vec<T>> {
        if let Self::List(soft_errors) = self {
            Some(soft_errors)
        } else {
            None
        }
    }

    pub fn maybe_soft_error_list_mut(&mut self) -> Option<&mut Vec<T>> {
        if let Self::List(soft_errors) = self {
            Some(soft_errors)
        } else {
            None
        }
    }
}

impl<'de, T: Deserialize<'de> + PartialEq> SoftErrors<T> {
    pub fn contains(&self, soft_error: &T) -> bool {
        if let Self::List(soft_errors) = self {
            soft_errors.contains(soft_error)
        } else {
            true
        }
    }
}

impl<'de, T: Deserialize<'de>> Default for SoftErrors<T> {
    fn default() -> Self {
        Self::new_list()
    }
}

impl<'de, T> Visitor<'de> for SoftErrorsVisitor<T>
where
    T: Deserialize<'de>,
{
    type Value = SoftErrors<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("`all` or dot separated soft errors for configuration loader")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let parts: Vec<_> = v
            .split('.')
            .filter(|item| *item != "")
            .map(String::from)
            .collect();
        if parts.contains(&"all".to_string()) {
            Ok(SoftErrors::All)
        } else {
            Ok(SoftErrors::List(Vec::deserialize(
                parts.into_deserializer(),
            )?))
        }
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(v)
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(v.as_str())
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for SoftErrors<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(SoftErrorsVisitor {
            _marker: PhantomData,
        })
    }
}
