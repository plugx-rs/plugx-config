use crate::loader::BoxedLoaderModifierFn;
use crate::{
    entity::ConfigurationEntity,
    loader::{ConfigurationLoadError, ConfigurationLoader},
};
use std::{
    collections::HashMap,
    fmt::{Debug, Formatter},
};
use url::Url;

pub type BoxedLoaderFn = Box<
    dyn Fn(
            &Url,
            Option<&[String]>,
        ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError>
        + Send
        + Sync,
>;
pub type BoxedCheckUrlFn = Box<dyn Fn(&Url) -> Result<(), ConfigurationLoadError> + Send + Sync>;

pub struct ConfigurationLoaderFn {
    name: &'static str,
    loader: BoxedLoaderFn,
    maybe_modifier: Option<BoxedLoaderModifierFn>,
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
    pub fn new<S: AsRef<str>>(name: &'static str, loader: BoxedLoaderFn, scheme: S) -> Self {
        Self {
            name,
            loader,
            maybe_modifier: None,
            scheme_list: [scheme.as_ref().into()].into(),
        }
    }

    pub fn set_name(&mut self, name: &'static str) {
        self.name = name
    }

    pub fn with_name(mut self, name: &'static str) -> Self {
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
    fn set_modifier(&mut self, modifier: BoxedLoaderModifierFn) {
        self.maybe_modifier = Some(modifier)
    }

    fn maybe_get_modifier(&self) -> Option<&BoxedLoaderModifierFn> {
        self.maybe_modifier.as_ref()
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn scheme_list(&self) -> Vec<String> {
        self.scheme_list.clone()
    }

    fn try_load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
    ) -> Result<HashMap<String, ConfigurationEntity>, ConfigurationLoadError> {
        (self.loader)(url, maybe_whitelist)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::enable_logging;

    #[test]
    fn load() {
        enable_logging();
        // let mut l = ConfigurationLoaderEnv::new("FOO")
        //     .unwrap()
        //     .with_key_separator("_");
        // println!("{l:?}");
        // let loaded = l.try_load().unwrap();
        // println!("{loaded:#?}");
        // for (p, r) in loaded {
        //     println!("{p}: {:?}\n\n\n\n", r.deserialize());
        // }
    }
}
