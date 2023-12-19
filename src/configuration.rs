use crate::{
    entity::ConfigurationEntity,
    error::{ConfigurationError, ConfigurationLoadError},
    loader::ConfigurationLoader,
    parser::ConfigurationParser,
};
use anyhow::anyhow;
use plugx_input::{
    position::InputPosition,
    schema::{InputSchemaError, InputSchemaType},
    Input,
};
use std::{
    collections::HashMap,
    env::{self, VarError},
};
use url::Url;

#[derive(Debug)]
pub struct Configuration {
    parser_list: Vec<Box<dyn ConfigurationParser>>,
    #[allow(clippy::type_complexity)]
    loader_list: Vec<(Url, Box<dyn ConfigurationLoader>)>,
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
            #[cfg(feature = "env")]
            Box::<crate::parser::env::ConfigurationParserEnv>::default(),
            #[cfg(feature = "json")]
            Box::<crate::parser::json::ConfigurationParserJson>::default(),
            #[cfg(feature = "toml")]
            Box::<crate::parser::toml::ConfigurationParserToml>::default(),
            #[cfg(feature = "yaml")]
            Box::<crate::parser::yaml::ConfigurationParserYaml>::default(),
        ];
        new
    }
}

impl Configuration {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

impl Configuration {
    pub fn has_url(&mut self, url: &Url) -> bool {
        self.loader_list
            .iter()
            .any(|(inner_url, _)| inner_url == url)
    }

    pub fn with_url_and_loader<L>(self, url: Url, loader: L) -> Self
    where
        L: ConfigurationLoader + 'static,
    {
        self.with_url_and_boxed_loader(url, Box::new(loader))
    }

    pub fn add_url_and_loader<L>(&mut self, url: Url, loader: L)
    where
        L: ConfigurationLoader + 'static,
    {
        self.add_url_and_boxed_loader(url, Box::new(loader))
    }

    pub fn with_url_and_boxed_loader(
        mut self,
        url: Url,
        loader: Box<dyn ConfigurationLoader>,
    ) -> Self {
        self.add_url_and_boxed_loader(url, loader);
        self
    }

    pub fn add_url_and_boxed_loader(&mut self, url: Url, loader: Box<dyn ConfigurationLoader>) {
        self.loader_list.push((url, loader));
    }

    pub fn remove_url(&mut self, url: &Url) -> bool {
        let mut result = false;
        while let Some(index) = self
            .loader_list
            .iter()
            .position(|(inner_url, _)| inner_url == url)
        {
            self.loader_list.remove(index);
            result = true;
        }
        result
    }

    pub fn take_boxed_loader(&mut self, url: &Url) -> Option<Box<dyn ConfigurationLoader>> {
        if let Some(index) = self
            .loader_list
            .iter()
            .position(|(inner_url, _)| inner_url == url)
        {
            Some(self.loader_list.swap_remove(index).1)
        } else {
            None
        }
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

    pub fn add_to_whitelist<P: AsRef<str>>(&mut self, plugin_name: P) {
        let plugin_name = plugin_name.as_ref().to_lowercase();
        if let Some(whitelist) = self.maybe_whitelist.as_mut() {
            whitelist.push(plugin_name);
        } else {
            self.maybe_whitelist = Some(Vec::from([plugin_name]));
        }
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
        self.loader_list.iter_mut().try_for_each(|(url, loader)| {
            loader
                .try_load(url, maybe_whitelist.map(|vector| vector.as_slice()))
                .or_else(|error| {
                    if skip_retryable && error.is_skippable() {
                        Ok(Vec::new())
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
        schemas: &HashMap<String, InputSchemaType>,
    ) -> Result<(), InputSchemaError> {
        self.merged
            .iter_mut()
            .try_for_each(|(plugin_name, merged_configuration)| {
                if let Some(schema) = schemas.get(plugin_name) {
                    schema.validate(
                        merged_configuration,
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
        schemas: &HashMap<String, InputSchemaType>,
    ) -> Result<(), ConfigurationError> {
        self.try_load_parse_merge(skip_retryable)?;
        self.try_validate(schemas)
            .map_err(|source| ConfigurationError::Validate { source })
    }
}
