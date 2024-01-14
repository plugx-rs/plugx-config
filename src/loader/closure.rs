//! Custom configuration loader with [Fn].
//!
//! ### Example
//! ```rust
//! use plugx_config::{
//!     entity::ConfigurationEntity,
//!     loader::{ConfigurationLoader, closure::ConfigurationLoaderFn},
//!     ext::url::Url,
//! };
//!
//! let url_scheme = "xyz";
//! let loader_name = "my-custom-loader";
//! let loader_fn = move |url: &Url, maybe_whitelist: Option<&[String]>, skip_soft_errors: bool| {
//!     // TODO: check `url` and get my own options
//!     let mut result = Vec::new();
//!     // TODO: check whitelist
//!     // load configurations
//!     // for example I load configuration for plugin named `foo`:
//!     let entity = ConfigurationEntity::new(url.clone(), "foo", loader_name)
//!         // If you do not set format here, `Configuration` struct will try to guess it later:
//!         .with_format("yml")
//!         // If you do not set contents here, the default value will be `plugx_input::Input::empty_map()`
//!         .with_contents("hello: world");
//!     result.push(("foo".to_string(), entity));
//!     Ok(result)
//! };
//! let url = "xyz:///my/own/path?my_option=value".parse().unwrap();
//! let loader = ConfigurationLoaderFn::new(loader_name, Box::new(loader_fn), url_scheme);
//! let loaded = loader.load(&url, None, false).unwrap();
//! assert_eq!(loaded.len(), 1);
//! ```
//!
//! * See [crate::loader] documentation to known how loaders work.
//! * To detect your own options easily see [crate::loader::deserialize_query_string].
//! * To detect your soft errors and apply them see [crate::loader::SoftErrors].

use crate::{
    entity::ConfigurationEntity,
    loader::{ConfigurationLoadError, ConfigurationLoader},
};
use std::fmt::{Debug, Formatter};
use url::Url;

/// A `|&Url, Option<&[String]>, bool| -> Result<Vec<String, ConfigurationEntity>, ConfigurationLoadError>` [Fn]
pub type BoxedLoaderFn = Box<
    dyn Fn(
            &Url,
            Option<&[String]>,
            bool,
        ) -> Result<Vec<(String, ConfigurationEntity)>, ConfigurationLoadError>
        + Send
        + Sync,
>;

/// Builder struct.
pub struct ConfigurationLoaderFn {
    name: String,
    loader: BoxedLoaderFn,
    scheme_list: Vec<String>,
}

impl Debug for ConfigurationLoaderFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationLoaderFn")
            .field("name", &self.name)
            .field("scheme_list", &self.scheme_list)
            .finish()
    }
}

impl ConfigurationLoaderFn {
    pub fn new<S: AsRef<str>, N: AsRef<str>>(name: N, loader: BoxedLoaderFn, scheme: S) -> Self {
        Self {
            name: name.as_ref().to_string(),
            loader,
            scheme_list: [scheme.as_ref().into()].into(),
        }
    }

    pub fn set_name<N: AsRef<str>>(&mut self, name: N) {
        self.name = name.as_ref().to_string()
    }

    pub fn with_name<N: AsRef<str>>(mut self, name: N) -> Self {
        self.set_name(name);
        self
    }

    pub fn set_loader(&mut self, loader: BoxedLoaderFn) {
        self.loader = loader
    }

    pub fn with_loader(mut self, loader: BoxedLoaderFn) -> Self {
        self.set_loader(loader);
        self
    }

    pub fn set_scheme_list<S: AsRef<str>>(&mut self, scheme_list: Vec<S>) {
        self.scheme_list = scheme_list
            .into_iter()
            .map(|scheme| scheme.as_ref().to_string())
            .collect();
    }

    pub fn with_scheme_list<S: AsRef<str>>(mut self, scheme_list: Vec<S>) -> Self {
        self.set_scheme_list(scheme_list);
        self
    }
}

impl ConfigurationLoader for ConfigurationLoaderFn {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn scheme_list(&self) -> Vec<String> {
        self.scheme_list.clone()
    }

    fn load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, ConfigurationEntity)>, ConfigurationLoadError> {
        (self.loader)(url, maybe_whitelist, skip_soft_errors)
    }
}
