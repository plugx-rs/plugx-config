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

#[derive(Debug, Default)]
pub struct Configuration {
    url_list: Vec<Url>,
    loader_list: Vec<Box<dyn ConfigurationLoader>>,
    parser_list: Vec<Box<dyn ConfigurationParser>>,
    maybe_whitelist: Option<Vec<String>>,
}

impl Configuration {
    pub fn new() -> Self {
        let new = Self {
            parser_list: vec![
                #[cfg(feature = "env")]
                Box::new(crate::parser::env::ConfigurationParserEnv::new()),
                #[cfg(feature = "json")]
                Box::new(crate::parser::json::ConfigurationParserJson::new()),
                #[cfg(feature = "toml")]
                Box::new(crate::parser::toml::ConfigurationParserToml::new()),
                #[cfg(feature = "yaml")]
                Box::new(crate::parser::yaml::ConfigurationParserYaml::new()),
            ],
            ..Default::default()
        };
        let parser_name_list: Vec<_> = new.parser_list.iter().map(|parser| parser.name()).collect();
        if parser_name_list.is_empty() {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!("Initialized with no parser")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("message=\"Initialized with no parser\"")
                }
            }
        } else {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(parser_list=?parser_name_list, "Initialized with parser(s)")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("parser_list={parser_name_list:?} message=\"Initialized with parser(s)\"")
                }
            }
        }
        new
    }
}

impl Configuration {
    pub fn has_url(&mut self, url: &Url) -> bool {
        self.url_list.contains(url)
    }

    pub fn has_url_scheme(&mut self, url: &Url) -> bool {
        self.url_list
            .iter()
            .any(|inner_url| inner_url.scheme() == url.scheme())
    }

    pub fn with_url(mut self, url: Url) -> Result<Self, ConfigurationLoadError> {
        self.add_url(url)?;
        Ok(self)
    }

    pub fn add_url(&mut self, url: Url) -> Result<(), ConfigurationLoadError> {
        let scheme = url.scheme().to_string();
        if self
            .loader_list
            .iter()
            .any(|loader| loader.scheme_list().contains(&scheme))
        {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(url=%url, "Added URL")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("url=\"{url}\" message=\"Added URL\"")
                }
            }
            self.url_list.push(url);
            return Ok(());
        }
        #[cfg(feature = "env")]
        {
            use crate::loader::env::{ConfigurationLoaderEnv, SCHEME_LIST};

            if SCHEME_LIST.contains(&scheme.as_str()) {
                self.add_boxed_loader(Box::new(ConfigurationLoaderEnv::new()));
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::debug!(url=%url, "Added URL")
                    } else if #[cfg(feature = "logging")] {
                        log::debug!("url=\"{url}\" message=\"Added URL\"")
                    }
                }
                self.url_list.push(url);
                return Ok(());
            };
        }
        #[cfg(feature = "fs")]
        {
            use crate::loader::fs::{ConfigurationLoaderFs, SCHEME_LIST};

            if SCHEME_LIST.contains(&scheme.as_str()) {
                self.add_boxed_loader(Box::new(ConfigurationLoaderFs::new()));
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::debug!(url=%url, "Added URL")
                    } else if #[cfg(feature = "logging")] {
                        log::debug!("url=\"{url}\" message=\"Added URL\"")
                    }
                }
                self.url_list.push(url);
                return Ok(());
            };
        }
        Err(ConfigurationLoadError::LoaderNotFound { scheme, url })
    }

    pub fn remove_url(&mut self, url: &Url) -> bool {
        let mut result = false;
        while let Some(index) = self.url_list.iter().position(|inner_url| inner_url == url) {
            self.url_list.remove(index);
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(url=%url, "Removed URL")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("url=\"{url}\" message=\"Removed URL\"")
                }
            }
            result = true;
        }
        result
    }

    pub fn remove_scheme<S: AsRef<str>>(&mut self, scheme: S) -> Vec<Url> {
        let mut url_list = Vec::new();
        while let Some(url) = self
            .url_list
            .iter()
            .find(|url| url.scheme() == scheme.as_ref())
        {
            url_list.push(url.clone())
        }
        url_list.iter().for_each(|url| {
            self.remove_url(url);
        });
        url_list
    }
}

impl Configuration {
    pub fn has_loader(&mut self, url: &Url) -> bool {
        let scheme = url.scheme().to_string();
        self.loader_list
            .iter()
            .any(|loader| loader.scheme_list().contains(&scheme))
    }

    pub fn with_loader<L>(mut self, loader: L) -> Self
    where
        L: ConfigurationLoader + 'static,
    {
        self.add_boxed_loader(Box::new(loader));
        self
    }

    pub fn add_loader<L>(&mut self, loader: L)
    where
        L: ConfigurationLoader + 'static,
    {
        self.add_boxed_loader(Box::new(loader));
    }

    pub fn with_boxed_loader(mut self, loader: Box<dyn ConfigurationLoader>) -> Self {
        self.add_boxed_loader(loader);
        self
    }

    pub fn add_boxed_loader(&mut self, loader: Box<dyn ConfigurationLoader>) {
        cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::debug!(loader=loader.name(), schema_list=?loader.scheme_list(), "Added loader")
            } else if #[cfg(feature = "logging")] {
                log::debug!("loader={:?} schema_list={:?} message=\"Added loader\"", loader.name(), loader.scheme_list())
            }
        }
        self.loader_list.push(loader);
    }

    pub fn remove_loader_and_urls<S: AsRef<str>>(
        &mut self,
        scheme: S,
    ) -> Option<(Box<dyn ConfigurationLoader>, Vec<Url>)> {
        let scheme_string = scheme.as_ref().to_string();
        if let Some(index) = self
            .loader_list
            .iter()
            .position(|loader| loader.scheme_list().contains(&scheme_string))
        {
            let loader = self.loader_list.swap_remove(index);
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(loader=loader.name(), schema_list=?loader.scheme_list(), "Removed loader")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("loader={:?} schema_list={:?} message=\"Removed loader\"", loader.name(), loader.scheme_list())
                }
            }
            Some((loader, self.remove_scheme(scheme)))
        } else {
            None
        }
    }

    pub fn load(
        &mut self,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, ConfigurationLoadError> {
        load(
            self.url_list.as_slice(),
            self.loader_list.as_slice(),
            self.maybe_whitelist.as_deref(),
            skip_soft_errors,
        )
    }
}

impl Configuration {
    pub fn has_parser<F: AsRef<str>>(&self, format: F) -> bool {
        let format = format.as_ref().to_lowercase();
        self.parser_list
            .iter()
            .any(|parser| parser.supported_format_list().contains(&format))
    }

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
        cfg_if! {if #[cfg(feature = "tracing")] {
                tracing::debug!(parser=parser.name(), format_list=?parser.supported_format_list(), "Added parser")
            } else if #[cfg(feature = "logging")] {
                log::debug!("parser={:?} format_list={:?} message=\"Added parser\"", parser.name(), parser.supported_format_list())
            }
        }
        self.parser_list.push(parser);
    }

    pub fn remove_parser<F: AsRef<str>>(&mut self, format: F) -> Vec<Box<dyn ConfigurationParser>> {
        let format = format.as_ref().to_lowercase();
        let mut parser_list = Vec::new();
        while let Some(index) = self
            .parser_list
            .iter()
            .position(|parser| parser.supported_format_list().contains(&format))
        {
            let parser = self.parser_list.swap_remove(index);
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(parser=parser.name(), format_list=?parser.supported_format_list(), "Removed parser")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("parser={:?} format_list={:?} message=\"Removed parser\"", parser.name(), parser.supported_format_list())
                }
            }
            parser_list.push(parser)
        }
        parser_list
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
    pub fn is_in_whitelist<P: AsRef<str>>(&self, name: P) -> bool {
        let name = name.as_ref().to_lowercase();
        self.maybe_whitelist
            .as_ref()
            .map(|whitelist| whitelist.contains(&name))
            .unwrap_or(false)
    }

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
        } else {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::info!(key=key.as_ref(), "Set whitelist from environment-variable")
                } else if #[cfg(feature = "logging")] {
                    log::info!("key={:?} message=\"Set whitelist from environment-variable\"", key.as_ref())
                }
            }
        }
        self.set_whitelist(whitelist.as_ref());
        Ok(())
    }

    pub fn set_whitelist_from_env<K: AsRef<str>>(
        mut self,
        key: K,
    ) -> Result<Self, ConfigurationError> {
        self.load_whitelist_from_env(key)?;
        Ok(self)
    }

    pub fn set_whitelist<N: AsRef<str>>(&mut self, whitelist: &[N]) {
        whitelist
            .iter()
            .for_each(|name| self.add_to_whitelist(name));
    }

    pub fn with_whitelist<N: AsRef<str>>(mut self, whitelist: &[N]) -> Self {
        self.set_whitelist(whitelist);
        self
    }

    pub fn add_to_whitelist<N: AsRef<str>>(&mut self, name: N) {
        let name = name.as_ref().to_lowercase();
        cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::debug!(name=name, "Added to whitelist")
            } else if #[cfg(feature = "logging")] {
                log::debug!("name={name:?} message=\"Added to whitelist\"")
            }
        }
        if let Some(whitelist) = self.maybe_whitelist.as_mut() {
            if !whitelist.contains(&name) {
                whitelist.push(name);
            }
        } else {
            self.maybe_whitelist = Some(Vec::from([name]));
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
    url_list: &[Url],
    loader_list: &[Box<dyn ConfigurationLoader>],
    maybe_whitelist: Option<&[String]>,
    skip_soft_errors: bool,
) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, ConfigurationLoadError> {
    let mut result: Vec<(String, Vec<_>)> = Vec::with_capacity(url_list.len());
    url_list
        .iter()
        .try_for_each(|url| {
            let scheme_string = url.scheme().to_string();
            if let Some(loader) = loader_list
                .iter()
                .find(|loader| loader.scheme_list().contains(&scheme_string))
            {
                loader
                    .try_load(url, maybe_whitelist, skip_soft_errors)
                    .map(|loaded_list| {
                        loaded_list
                            .into_iter()
                            .for_each(|(plugin_name, configuration)| {
                                if let Some((_, configuration_list)) =
                                    result.iter_mut().find(|(loaded_plugin_name, _)| {
                                        loaded_plugin_name == &plugin_name
                                    })
                                {
                                    configuration_list.push(configuration);
                                } else {
                                    result.push((plugin_name.clone(), [configuration].to_vec()))
                                }
                            });
                    })
            } else {
                Err(ConfigurationLoadError::LoaderNotFound {
                    scheme: scheme_string,
                    url: url.clone(),
                })
            }
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
                .iter_mut()
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
