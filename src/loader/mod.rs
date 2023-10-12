use crate::entity::ConfigurationEntity;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, fmt::Debug};
use url::Url;

pub mod closure;
#[cfg(feature = "env")]
pub mod env;
#[cfg(feature = "fs")]
pub mod fs;

pub type BoxedLoaderModifierFn = Box<
    dyn Fn(&Url, &mut HashMap<String, ConfigurationEntity>) -> Result<(), ConfigurationLoadError>
        + Send
        + Sync,
>;

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationLoadError {
    #[error("{loader} configuration loader could not found configuration `{url}`")]
    NotFound { loader: String, url: String },
    #[error("{loader} configuration loader has no access to load configuration from `{url}`")]
    NoAccess { loader: String, url: String },
    #[error(
        "{loader} configuration loader reached timeout `{timeout_in_seconds}s` to load `{url}`"
    )]
    Timeout {
        loader: String,
        url: String,
        timeout_in_seconds: usize,
    },
    #[error("{loader} configuration loader got invalid source `{url}`")]
    InvalidSource {
        loader: String,
        url: String,
        #[source]
        error: anyhow::Error,
    },
    #[error("Could not found configuration loader for scheme {scheme}")]
    UrlSchemeNotFound { scheme: String },
    #[error("{loader} configuration loader found duplicate configurations `{url}/{plugin}.({format_1}|{format_2})`")]
    Duplicate {
        loader: String,
        url: String,
        plugin: String,
        format_1: String,
        format_2: String,
    },
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
pub trait ConfigurationLoader: Send + Sync + Debug {
    fn set_modifier(&mut self, _modifier: BoxedLoaderModifierFn) {}

    fn maybe_get_modifier(&self) -> Option<&BoxedLoaderModifierFn> {
        None
    }

    fn name(&self) -> &'static str;

    fn scheme_list(&self) -> Vec<String>;

    fn try_load(
        &self,
        source: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError>;

    fn try_load_and_maybe_modify(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        let mut loaded = self.try_load(url, maybe_whitelist)?;
        if let Some(modifier) = self.maybe_get_modifier() {
            // TODO: logging
            modifier(url, &mut loaded)?
        };
        Ok(loaded)
    }
}

#[cfg(feature = "qs")]
pub fn deserialize_query_string<T: DeserializeOwned>(
    loader_name: &'static str,
    url: &Url,
) -> Result<T, ConfigurationLoadError> {
    serde_qs::from_str(url.query().unwrap_or_default()).map_err(|error| {
        ConfigurationLoadError::InvalidSource {
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
