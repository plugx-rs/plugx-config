use crate::{
    entity::ConfigurationEntity,
    error::{ConfigurationError, ConfigurationLoadError},
    loader::ConfigurationLoader,
    parser::ConfigurationParser,
};
use anyhow::anyhow;
use cfg_if::cfg_if;
use plugx_input::{position::InputPosition, schema::InputSchemaType, Input};
use std::env;
use url::Url;

#[derive(Debug)]
pub struct Configuration {
    loader_list: Vec<(Url, Box<dyn ConfigurationLoader>)>,
    parser_list: Vec<Box<dyn ConfigurationParser>>,
    maybe_whitelist: Option<Vec<String>>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            parser_list: vec![
                #[cfg(feature = "env")]
                Box::<crate::parser::env::ConfigurationParserEnv>::default(),
                #[cfg(feature = "json")]
                Box::<crate::parser::json::ConfigurationParserJson>::default(),
                #[cfg(feature = "toml")]
                Box::<crate::parser::toml::ConfigurationParserToml>::default(),
                #[cfg(feature = "yaml")]
                Box::<crate::parser::yaml::ConfigurationParserYaml>::default(),
            ],
            loader_list: Default::default(),
            maybe_whitelist: Default::default(),
        }
    }
}

impl Configuration {
    pub fn new() -> Self {
        Self {
            parser_list: Default::default(),
            loader_list: Default::default(),
            maybe_whitelist: Default::default(),
        }
    }
}

impl Configuration {
    pub fn has_url(&mut self, url: &Url) -> bool {
        self.loader_list
            .iter()
            .any(|(inner_url, _)| inner_url == url)
    }

    pub fn with_url(mut self, url: Url) -> Result<Self, ConfigurationLoadError> {
        self.add_url(url)?;
        Ok(self)
    }

    pub fn add_url(&mut self, url: Url) -> Result<(), ConfigurationLoadError> {
        let scheme = url.scheme().to_string();
        #[cfg(feature = "env")]
        {
            let loader = crate::loader::env::ConfigurationLoaderEnv::new();
            if loader.scheme_list().contains(&scheme) {
                return Ok(self.add_url_and_boxed_loader(url, Box::new(loader)));
            };
        }
        #[cfg(feature = "fs")]
        {
            let loader = crate::loader::fs::ConfigurationLoaderFs::new();
            if loader.scheme_list().contains(&scheme) {
                return Ok(self.add_url_and_boxed_loader(url, Box::new(loader)));
            };
        }
        Err(ConfigurationLoadError::LoaderNotFound { scheme, url })
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
            self.loader_list.swap_remove(index);
            result = true;
        }
        result
    }

    pub fn take_boxed_loader(&mut self, url: &Url) -> Option<Box<dyn ConfigurationLoader>> {
        self.loader_list
            .iter()
            .position(|(inner_url, _)| inner_url == url)
            .map(|index| {
                let (_, loader) = self.loader_list.swap_remove(index);
                loader
            })
    }

    pub fn load(
        &mut self,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, ConfigurationLoadError> {
        let maybe_whitelist = self.maybe_whitelist.as_ref().map(|list| list.as_slice());
        load(
            self.loader_list.as_slice(),
            maybe_whitelist,
            skip_soft_errors,
        )
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

    pub fn parse(
        &mut self,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, ConfigurationError> {
        let mut load_result = self.load(skip_soft_errors)?;
        parse(load_result.as_mut(), self.parser_list.as_slice())?;
        Ok(load_result)
    }
}

impl Configuration {
    pub fn load_whitelist_from_env<K: AsRef<str>>(
        &mut self,
        key: K,
    ) -> Result<(), ConfigurationError> {
        let whitelist = env::var(key.as_ref())
            .map(|value| value.trim().to_lowercase())
            .map(|value| {
                if value.is_empty() {
                    Vec::new()
                } else {
                    value.split([' ', ',', ';']).map(String::from).collect()
                }
            })
            .map_err(|error| {
                ConfigurationError::Other(anyhow!("Invalid key or the value is not set: {}", error))
            })?;
        if whitelist.is_empty() {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::warn!(key=key.as_ref(), "Whitelist environment-variable is set to empty")
                } else if #[cfg(feature = "logging")] {
                    log::warn!("key={:?} message=\"Whitelist environment-variable is set to empty\"", key.as_ref())
                }
            }
        }
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
    pub fn merge(
        &mut self,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Input)>, ConfigurationError> {
        let mut parsed = self.parse(skip_soft_errors)?;
        merge(parsed.as_mut())
    }
}

impl Configuration {
    pub fn validate(
        &mut self,
        schema_list: &[(String, InputSchemaType)],
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Input)>, ConfigurationError> {
        let mut merged = self.merge(skip_soft_errors)?;
        validate(merged.as_mut(), schema_list)
    }
}

pub fn load(
    loader_list: &[(Url, Box<dyn ConfigurationLoader>)],
    maybe_whitelist: Option<&[String]>,
    skip_soft_errors: bool,
) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, ConfigurationLoadError> {
    let mut result: Vec<(String, Vec<_>)> = Vec::new();
    loader_list
        .iter()
        .try_for_each(|(url, loader)| {
            loader
                .try_load(url, maybe_whitelist)
                .or_else(|error| {
                    if skip_soft_errors && error.is_skippable() {
                        Ok(Vec::new())
                    } else {
                        Err(error)
                    }
                })
                .map(|loaded_list| {
                    loaded_list
                        .into_iter()
                        .for_each(|(plugin_name, configuration)| {
                            if let Some((_, configuration_list)) = result
                                .iter_mut()
                                .find(|(loaded_plugin_name, _)| loaded_plugin_name == &plugin_name)
                            {
                                configuration_list.push(configuration);
                            } else {
                                result.push((plugin_name.clone(), [configuration].to_vec()))
                            }
                        });
                })
        })
        .map(|_| result)
}

pub fn parse(
    plugin_configuration_list: &mut [(String, Vec<ConfigurationEntity>)],
    parser_list: &[Box<dyn ConfigurationParser>],
) -> Result<(), ConfigurationError> {
    plugin_configuration_list
        .iter_mut()
        .try_for_each(|(plugin_name, configuration_list)| {
            configuration_list
                .into_iter()
                .try_for_each(|configuration| {
                    if configuration.maybe_parsed_contents().is_none() {
                        let parsed =
                            configuration.parse_contents(parser_list).map_err(|error| {
                                ConfigurationError::Parse {
                                    plugin_name: plugin_name.to_string(),
                                    url: configuration.url().clone(),
                                    source: error,
                                }
                            })?;
                        configuration.set_parsed_contents(parsed);
                    }
                    Ok::<_, ConfigurationError>(())
                })?;
            Ok::<_, ConfigurationError>(())
        })
}

pub fn merge(
    plugin_configuration_list: &[(String, Vec<ConfigurationEntity>)],
) -> Result<Vec<(String, Input)>, ConfigurationError> {
    let mut result = Vec::with_capacity(plugin_configuration_list.len());
    plugin_configuration_list
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
            result.push((plugin_name.to_string(), first));
        });
    Ok(result)
}

pub fn validate(
    plugin_configuration_list: &[(String, Input)],
    schema_list: &[(String, InputSchemaType)],
) -> Result<Vec<(String, Input)>, ConfigurationError> {
    let mut result = Vec::with_capacity(plugin_configuration_list.len());
    plugin_configuration_list
        .iter()
        .try_for_each(|(plugin_name, configuration)| {
            let mut configuration = configuration.clone();
            if let Some((_, schema_type)) = schema_list
                .iter()
                .find(|(schema_plugin_name, _)| schema_plugin_name == plugin_name)
            {
                schema_type.validate(
                    &mut configuration,
                    Some(InputPosition::new().new_with_key(plugin_name)),
                )
            } else {
                Ok(())
            }?;
            result.push((plugin_name.to_string(), configuration));
            Ok::<_, ConfigurationError>(())
        })
        .map(|_| result)
}
