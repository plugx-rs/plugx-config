use crate::{
    entity::ConfigurationEntity,
    error::{ConfigurationError, ConfigurationLoadError},
    loader::ConfigurationLoader,
    parser::{self, ConfigurationParser},
};
use plugx_input::{
    definition::InputDefinition, position::InputPosition, validation::InputValidateError, Input,
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use url::Url;

#[derive(Debug)]
pub struct Configuration {
    parser_list: Vec<Box<dyn ConfigurationParser>>,
    loader_list: Vec<Box<dyn ConfigurationLoader>>,
    #[allow(clippy::type_complexity)]
    source_list: Vec<(Url, Option<Arc<RwLock<Box<dyn ConfigurationLoader>>>>)>,
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
            source_list: Default::default(),
            states: Default::default(),
            merged: Default::default(),
            maybe_whitelist: Default::default(),
            forget_loaded: Default::default(),
            forget_parsed: Default::default(),
        }
    }
}

impl Configuration {
    pub fn with_source<S>(mut self, source: S) -> Self
    where
        S: Into<Url>,
    {
        self.add_source(source);
        self
    }

    pub fn add_source<S>(&mut self, source: S)
    where
        S: Into<Url>,
    {
        let source = source.into();
        if !self.has_source(source.clone()) {
            self.source_list.push((source, None));
        }
    }

    pub fn has_source<S>(&mut self, source: S) -> bool
    where
        S: Into<Url>,
    {
        let source = source.into();
        self.source_list
            .iter()
            .find(|(inner_source, _)| inner_source == &source)
            .map(|_| true)
            .unwrap_or_default()
    }

    pub fn with_source_and_loader<S, L>(self, source: S, loader: L) -> Self
    where
        S: Into<Url>,
        L: ConfigurationLoader + 'static,
    {
        self.with_source_and_boxed_loader(source, Box::new(loader))
    }

    pub fn add_source_and_loader<S, L>(&mut self, source: S, loader: L)
    where
        S: Into<Url>,
        L: ConfigurationLoader + 'static,
    {
        self.add_source_and_boxed_loader(source, Box::new(loader))
    }

    pub fn with_source_and_boxed_loader<S>(
        mut self,
        source: S,
        loader: Box<dyn ConfigurationLoader>,
    ) -> Self
    where
        S: Into<Url>,
    {
        self.add_source_and_boxed_loader(source, loader);
        self
    }

    pub fn add_source_and_boxed_loader<S>(
        &mut self,
        source: S,
        loader: Box<dyn ConfigurationLoader>,
    ) where
        S: Into<Url>,
    {
        let source = source.into();
        if !self.has_source(source.clone()) {
            self.source_list
                .push((source, Some(Arc::new(RwLock::new(loader)))));
        }
    }

    pub fn remove_source_and_loader<S>(&mut self, source: S) -> bool
    where
        S: Into<Url>,
    {
        let source = source.into();
        self.source_list
            .iter_mut()
            .position(|(inner_source, _)| inner_source == &source)
            .map(|index| {
                self.source_list.remove(index);
                true
            })
            .unwrap_or_default()
    }

    pub fn take_boxed_loader<S>(&mut self, source: S) -> Option<Box<dyn ConfigurationLoader>>
    where
        S: Into<Url>,
    {
        let source = source.into();
        if let Some(index) = self
            .source_list
            .iter_mut()
            .position(|(inner_source, _)| inner_source == &source)
        {
            let (_, maybe_loader) = self.source_list.remove(index);
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

    pub fn get_boxed_loader<S>(
        &self,
        source: S,
    ) -> Option<Arc<RwLock<Box<dyn ConfigurationLoader>>>>
    where
        S: Into<Url>,
    {
        let source = source.into();
        self.source_list
            .iter()
            .find(|(inner_source, _)| inner_source == &source)
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
        self.parser_list.push(Box::new(parser));
    }
}

impl Configuration {
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
                .for_each(|configuration| *configuration.maybe_parsed_mut() = None)
        })
    }
}
impl Configuration {
    pub fn try_load(&mut self, skip_retryable: bool) -> Result<(), ConfigurationLoadError> {
        let maybe_whitelist = self.maybe_whitelist.as_ref();
        self.source_list
            .iter_mut()
            .try_for_each(|(source, maybe_loader)| {
                let load_result = if let Some(loader) = maybe_loader {
                    let loader =
                        loader
                            .try_write()
                            .map_err(|_| ConfigurationLoadError::AcquireLock {
                                url: source.to_string(),
                            })?;
                    loader.try_load(
                        source.clone(),
                        maybe_whitelist.map(|vector| vector.as_slice()),
                    )
                } else if let Some(loader) = self
                    .loader_list
                    .iter_mut()
                    .find(|loader| loader.scheme_list().contains(&source.scheme().to_string()))
                {
                    loader.try_load(
                        source.clone(),
                        maybe_whitelist.map(|vector| vector.as_slice()),
                    )
                } else {
                    return Err(ConfigurationLoadError::UrlSchemeNotFound {
                        scheme: source.scheme().to_string(),
                    });
                };
                load_result
                    .or_else(|error| {
                        if skip_retryable && error.is_retryable() {
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
                    let parsed = configuration.parse(&self.parser_list).map_err(|error| {
                        ConfigurationError::Parse {
                            plugin_name: plugin_name.to_string(),
                            configuration_source: configuration.source().to_string(),
                            source: error,
                        }
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
                    .filter(|configuration| configuration.maybe_parsed().is_some())
                    .for_each(|configuration| {
                        plugx_input::merge::merge_with_positions(
                            &mut first,
                            plugx_input::position::new().new_with_key(plugin_name),
                            configuration.maybe_parsed().unwrap(),
                            plugx_input::position::new().new_with_key(configuration.source()),
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
