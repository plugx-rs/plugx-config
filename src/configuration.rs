use crate::{
    entity::ConfigurationEntity,
    error::{ConfigurationError, ConfigurationLoadError},
    loader::ConfigurationLoader,
    parser::{self, ConfigurationParser},
};
use anyhow::anyhow;
use plugx_input::{
    definition::InputDefinition, position::InputPosition, validation::InputValidateError, Input,
};
use std::{
    collections::HashMap,
    env::{self, VarError},
    sync::{Arc, RwLock},
};
use url::Url;

#[derive(Debug)]
pub struct Configuration {
    parser_list: Vec<Box<dyn ConfigurationParser>>,
    loader_list: Vec<Box<dyn ConfigurationLoader>>,
    #[allow(clippy::type_complexity)]
    url_list: Vec<(Url, Option<Arc<RwLock<Box<dyn ConfigurationLoader>>>>)>,
    states: HashMap<String, Vec<ConfigurationEntity>>,
    merged: HashMap<String, Input>,
    maybe_whitelist: Option<Vec<String>>,
    forget_loaded: bool,
    forget_parsed: bool,
}

impl Default for Configuration {
    fn default() -> Self {
        let mut new = Self::new();
        new.parser_list = vec![
            #[cfg(feature = "env-parser")]
            Box::<parser::env::ConfigurationParserEnv>::default(),
            #[cfg(feature = "json")]
            Box::<parser::json::ConfigurationParserJson>::default(),
            #[cfg(feature = "toml")]
            Box::<parser::toml::ConfigurationParserToml>::default(),
            #[cfg(feature = "yaml")]
            Box::<parser::yaml::ConfigurationParserYaml>::default(),
        ];
        new
    }
}

impl Configuration {
    pub fn new() -> Self {
        Self {
            parser_list: Default::default(),
            loader_list: Default::default(),
            url_list: Default::default(),
            states: Default::default(),
            merged: Default::default(),
            maybe_whitelist: Default::default(),
            forget_loaded: Default::default(),
            forget_parsed: Default::default(),
        }
    }
}

impl Configuration {
    pub fn with_url<S>(mut self, url: S) -> Self
    where
        S: Into<Url>,
    {
        self.add_url(url);
        self
    }

    pub fn add_url<S>(&mut self, url: S)
    where
        S: Into<Url>,
    {
        let url = url.into();
        if !self.has_url(url.clone()) {
            self.url_list.push((url, None));
        }
    }

    pub fn has_url<S>(&mut self, url: S) -> bool
    where
        S: Into<Url>,
    {
        let url = url.into();
        self.url_list
            .iter()
            .find(|(inner_url, _)| inner_url == &url)
            .map(|_| true)
            .unwrap_or_default()
    }

    pub fn with_url_and_loader<S, L>(self, url: S, loader: L) -> Self
    where
        S: Into<Url>,
        L: ConfigurationLoader + 'static,
    {
        self.with_url_and_boxed_loader(url, Box::new(loader))
    }

    pub fn add_url_and_loader<S, L>(&mut self, url: S, loader: L)
    where
        S: Into<Url>,
        L: ConfigurationLoader + 'static,
    {
        self.add_url_and_boxed_loader(url, Box::new(loader))
    }

    pub fn with_url_and_boxed_loader<S>(
        mut self,
        url: S,
        loader: Box<dyn ConfigurationLoader>,
    ) -> Self
    where
        S: Into<Url>,
    {
        self.add_url_and_boxed_loader(url, loader);
        self
    }

    pub fn add_url_and_boxed_loader<S>(&mut self, url: S, loader: Box<dyn ConfigurationLoader>)
    where
        S: Into<Url>,
    {
        let url = url.into();
        if !self.has_url(url.clone()) {
            self.url_list
                .push((url, Some(Arc::new(RwLock::new(loader)))));
        }
    }

    pub fn remove_url_and_loader<S>(&mut self, url: S) -> bool
    where
        S: Into<Url>,
    {
        let url = url.into();
        self.url_list
            .iter_mut()
            .position(|(inner_url, _)| inner_url == &url)
            .map(|index| {
                self.url_list.remove(index);
                true
            })
            .unwrap_or_default()
    }

    pub fn take_boxed_loader<S>(&mut self, url: S) -> Option<Box<dyn ConfigurationLoader>>
    where
        S: Into<Url>,
    {
        let url = url.into();
        if let Some(index) = self
            .url_list
            .iter_mut()
            .position(|(inner_url, _)| inner_url == &url)
        {
            let (_, maybe_loader) = self.url_list.remove(index);
            maybe_loader
                .map(|loader| {
                    Arc::into_inner(loader)
                        .map(|loader| loader.into_inner().ok())
                        .unwrap_or_default()
                })
                .unwrap_or_default()
        } else {
            None
        }
    }

    pub fn get_boxed_loader<S>(&self, url: S) -> Option<Arc<RwLock<Box<dyn ConfigurationLoader>>>>
    where
        S: Into<Url>,
    {
        let url = url.into();
        self.url_list
            .iter()
            .find(|(inner_url, _)| inner_url == &url)
            .map(|(_, loader)| loader.clone())
            .unwrap_or_default()
    }

    pub fn with_generic_loader<L>(self, loader: L) -> Self
    where
        L: ConfigurationLoader + 'static,
    {
        self.with_generic_boxed_loader(Box::new(loader))
    }

    pub fn add_generic_loader<L>(&mut self, loader: L)
    where
        L: ConfigurationLoader + 'static,
    {
        self.add_generic_boxed_loader(Box::new(loader))
    }

    pub fn with_generic_boxed_loader(mut self, loader: Box<dyn ConfigurationLoader>) -> Self {
        self.add_generic_boxed_loader(loader);
        self
    }

    pub fn add_generic_boxed_loader(&mut self, loader: Box<dyn ConfigurationLoader>) {
        self.loader_list.push(loader);
    }
}

impl Configuration {
    pub fn with_parser<P>(mut self, parser: P) -> Self
    where
        P: ConfigurationParser + 'static,
    {
        self.add_parser(parser);
        self
    }

    pub fn add_parser<P>(&mut self, parser: P)
    where
        P: ConfigurationParser + 'static,
    {
        self.add_boxed_parser(Box::new(parser));
    }

    pub fn with_boxed_parser(mut self, parser: Box<dyn ConfigurationParser>) -> Self {
        self.add_boxed_parser(parser);
        self
    }

    pub fn add_boxed_parser(&mut self, parser: Box<dyn ConfigurationParser>) {
        self.parser_list.push(parser);
    }
}

impl Configuration {
    pub fn load_whitelist_from_env<K: AsRef<str>>(
        &mut self,
        key: K,
    ) -> Result<(), ConfigurationError> {
        let whitelist = env::var(key.as_ref())
            .map(|value| value.trim().to_lowercase())
            .and_then(|value| {
                if value.is_empty() {
                    Err(VarError::NotPresent)
                } else {
                    Ok(value.split([' ', ',']).map(String::from).collect())
                }
            })
            .map_err(|error| {
                ConfigurationError::Other(anyhow!("Invalid key or the value is not set: {}", error))
            })?;
        self.set_whitelist(whitelist);
        Ok(())
    }

    pub fn set_whitelist_from_env<K: AsRef<str>>(
        mut self,
        key: K,
    ) -> Result<Self, ConfigurationError> {
        self.load_whitelist_from_env(key)?;
        Ok(self)
    }

    pub fn set_whitelist<P: AsRef<str>>(&mut self, whitelist: Vec<P>) {
        self.maybe_whitelist = Some(
            whitelist
                .into_iter()
                .map(|plugin_name| plugin_name.as_ref().to_lowercase())
                .collect(),
        );
    }

    pub fn with_whitelist<P: AsRef<str>>(mut self, whitelist: Vec<P>) -> Self {
        self.set_whitelist(whitelist);
        self
    }
}

impl Configuration {
    pub fn configuration(&self) -> &HashMap<String, Input> {
        &self.merged
    }

    pub fn configuration_mut(&mut self) -> &mut HashMap<String, Input> {
        &mut self.merged
    }
}

impl Configuration {
    pub fn with_forget_loaded(mut self, flag: bool) -> Self {
        self.set_forget_loaded(flag);
        self
    }

    pub fn set_forget_loaded(&mut self, flag: bool) {
        self.forget_loaded = flag;
    }

    pub fn forget_loaded(&mut self) {
        self.states.iter_mut().for_each(|(_, list)| {
            list.iter_mut()
                .for_each(|configuration| *configuration.maybe_contents_mut() = None)
        })
    }

    pub fn with_forget_parsed(mut self, flag: bool) -> Self {
        self.set_forget_parsed(flag);
        self
    }

    pub fn set_forget_parsed(&mut self, flag: bool) {
        self.forget_parsed = flag;
    }

    pub fn forget_parsed(&mut self) {
        self.states.iter_mut().for_each(|(_, list)| {
            list.iter_mut()
                .for_each(|configuration| *configuration.maybe_parsed_contents_mut() = None)
        })
    }
}
impl Configuration {
    pub fn try_load(&mut self, skip_retryable: bool) -> Result<(), ConfigurationLoadError> {
        let maybe_whitelist = self.maybe_whitelist.as_ref();
        self.url_list
            .iter_mut()
            .try_for_each(|(url, maybe_loader)| {
                let load_result = if let Some(loader) = maybe_loader {
                    let loader = loader
                        .try_write()
                        .map_err(|_| ConfigurationLoadError::AcquireLock { url: url.clone() })?;
                    loader.try_load(url, maybe_whitelist.map(|vector| vector.as_slice()))
                } else if let Some(loader) = self
                    .loader_list
                    .iter_mut()
                    .find(|loader| loader.scheme_list().contains(&url.scheme().to_string()))
                {
                    loader.try_load(url, maybe_whitelist.map(|vector| vector.as_slice()))
                } else {
                    return Err(ConfigurationLoadError::UrlSchemeNotFound {
                        scheme: url.scheme().to_string(),
                    });
                };
                load_result
                    .or_else(|error| {
                        if skip_retryable && error.is_skippable() {
                            Ok(HashMap::new())
                        } else {
                            Err(error)
                        }
                    })
                    .map(|result| {
                        result.into_iter().for_each(|(plugin_name, configuration)| {
                            if let Some(configuration_list) = self.states.get_mut(&plugin_name) {
                                configuration_list.push(configuration);
                            } else {
                                self.states.insert(plugin_name, [configuration].to_vec());
                            }
                        });
                    })
            })
    }

    pub fn try_parse(&mut self) -> Result<(), ConfigurationError> {
        self.states
            .iter_mut()
            .try_for_each(|(plugin_name, configuration_list)| {
                configuration_list.iter_mut().try_for_each(|configuration| {
                    let parsed =
                        configuration
                            .parse_contents(&self.parser_list)
                            .map_err(|error| ConfigurationError::Parse {
                                plugin_name: plugin_name.to_string(),
                                url: configuration.url().clone(),
                                source: error,
                            })?;
                    configuration.set_parsed_contents(parsed);
                    Ok(())
                })
            })
            .map(|result| {
                if self.forget_loaded {
                    self.forget_loaded()
                };
                result
            })
    }

    pub fn merge(&mut self) {
        self.states
            .iter()
            .for_each(|(plugin_name, configuration_list)| {
                let mut first = Input::new_map();
                configuration_list
                    .iter()
                    .filter(|configuration| configuration.maybe_parsed_contents().is_some())
                    .for_each(|configuration| {
                        plugx_input::merge::merge_with_positions(
                            &mut first,
                            plugx_input::position::new().new_with_key(plugin_name),
                            configuration.maybe_parsed_contents().unwrap(),
                            plugx_input::position::new().new_with_key(configuration.url().as_str()),
                        )
                    });
                self.merged.insert(plugin_name.to_string(), first);
            });
        if self.forget_parsed {
            self.forget_parsed()
        }
    }

    pub fn try_validate(
        &mut self,
        definitions: &HashMap<String, InputDefinition>,
    ) -> Result<(), InputValidateError> {
        self.merged
            .iter_mut()
            .try_for_each(|(plugin_name, merged_configuration)| {
                if let Some(plugin_definitions) = definitions.get(plugin_name) {
                    plugx_input::validation::validate(
                        merged_configuration,
                        plugin_definitions,
                        Some(InputPosition::new().new_with_key(plugin_name)),
                    )
                } else {
                    Ok(())
                }
            })
    }

    pub fn try_load_parse_merge(&mut self, skip_retryable: bool) -> Result<(), ConfigurationError> {
        self.try_load(skip_retryable)
            .map_err(|source| ConfigurationError::Load { source })?;
        self.try_parse()?;
        self.merge();
        Ok(())
    }

    pub fn try_load_parse_merge_validate(
        &mut self,
        skip_retryable: bool,
        definitions: &HashMap<String, InputDefinition>,
    ) -> Result<(), ConfigurationError> {
        self.try_load_parse_merge(skip_retryable)?;
        self.try_validate(definitions)
            .map_err(|source| ConfigurationError::Validate { source })
    }
}
