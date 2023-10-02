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
    #[error("{loader} configuration loader found duplicate configurations `{url}({extension_1}|{extension_2})`")]
    Duplicate {
        loader: String,
        url: String,
        extension_1: String,
        extension_2: String,
    },
    #[error("{loader} configuration loader could not {description} `{url}`")]
    Load {
        loader: String,
        url: String,
        description: String,
        source: anyhow::Error,
        retryable: bool,
    },
    #[error("Could not acquire lock for configuration loader with url `{url}`")]
    AcquireLock { url: String },
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
pub fn parse_url<R: DeserializeOwned>(
    loader: &'static str,
    url: &mut Url,
) -> Result<R, ConfigurationLoadError> {
    serde_qs::from_str(url.query().unwrap_or_default())
        .map(|result| {
            url.set_query(None);
            result
        })
        .map_err(|error| ConfigurationLoadError::InvalidSource {
            loader: loader.to_string(),
            error: error.into(),
            url: url.to_string(),
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
