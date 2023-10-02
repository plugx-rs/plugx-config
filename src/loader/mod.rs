use crate::entity::ConfigurationEntity;
use serde::de::DeserializeOwned;
use std::{collections::HashMap, fmt::Debug};
use url::Url;

pub mod closure;
#[cfg(feature = "env-loader")]
pub mod env;
#[cfg(feature = "fs")]
pub mod fs;

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationLoadError {
    #[error(
        "{loader} configuration loader could not found configuration `{configuration_source}`"
    )]
    NotFound {
        loader: String,
        configuration_source: String,
    },
    #[error("{loader} configuration loader has no access to load configuration from `{configuration_source}`")]
    NoAccess {
        loader: String,
        configuration_source: String,
    },
    #[error("{loader} configuration loader reached timeout `{timeout_in_seconds}s` to load `{configuration_source}`")]
    Timeout {
        loader: String,
        configuration_source: String,
        timeout_in_seconds: usize,
    },
    #[error("{loader} configuration loader got invalid source `{configuration_source}`")]
    InvalidSource {
        loader: String,
        configuration_source: String,
        #[source]
        error: anyhow::Error,
    },
    #[error("Invalid URL `{url}`")]
    InvalidUrl {
        url: String,
        #[source]
        error: anyhow::Error,
    },
    #[error("Could not found configuration loader for scheme {scheme}")]
    UrlSchemeNotFound { scheme: String },
    #[error("{loader} configuration loader found duplicate configurations `{configuration_source}({extension_1}|{extension_2})`")]
    Duplicate {
        loader: String,
        configuration_source: String,
        extension_1: String,
        extension_2: String,
    },
    #[error("{loader} configuration loader could not {description} `{configuration_source}`")]
    Load {
        loader: String,
        configuration_source: String,
        description: String,
        #[source]
        error: anyhow::Error,
        retryable: bool,
    },
    #[error("Could not acquire lock for configuration loader with source ")]
    AcquireLock { configuration_source: String },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
pub trait ConfigurationLoader: Send + Sync + Debug {
    fn name(&self) -> &'static str;
    fn scheme_list(&self) -> Vec<String>;
    fn try_load(
        &self,
        source: Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError>;
}

#[cfg(feature = "qs")]
pub fn parse_url<R: DeserializeOwned>(url: &mut Url) -> Result<R, ConfigurationLoadError> {
    serde_qs::from_str(url.query().unwrap_or_default())
        .map(|result| {
            url.set_query(None);
            result
        })
        .map_err(|error| ConfigurationLoadError::InvalidUrl {
            url: url.to_string(),
            error: error.into(),
        })
}

impl ConfigurationLoadError {
    pub fn is_retryable(&self) -> bool {
        if let Self::Load { retryable, .. } = self {
            *retryable
        } else {
            false
        }
    }
}
