use crate::{
    entity::ConfigurationEntity, error::Error, loader::Error as LoaderError, loader::Loader,
    parser::Parser,
};
use anyhow::anyhow;
use cfg_if::cfg_if;
use plugx_input::{position::InputPosition, schema::InputSchemaType, Input};
use std::env;
use url::Url;

#[derive(Debug, Default)]
pub struct Configuration {
    url_list: Vec<Url>,
    loader_list: Vec<Box<dyn Loader>>,
    parser_list: Vec<Box<dyn Parser>>,
    maybe_whitelist: Option<Vec<String>>,
}

impl Configuration {
    pub fn new() -> Self {
        let new = Self {
            parser_list: vec![
                #[cfg(feature = "env")]
                Box::new(crate::parser::env::Env::new()),
                #[cfg(feature = "json")]
                Box::new(crate::parser::json::Json::new()),
                #[cfg(feature = "toml")]
                Box::new(crate::parser::toml::Toml::new()),
                #[cfg(feature = "yaml")]
                Box::new(crate::parser::yaml::Yaml::new()),
            ],
            ..Default::default()
        };
        let parser_name_list: Vec<_> = new
            .parser_list
            .iter()
            .map(|parser| format!("{parser}"))
            .collect();
        if parser_name_list.is_empty() {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!("Initialized with no parser")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("msg=\"Initialized with no parser\"")
                }
            }
        } else {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(parsers=?parser_name_list, "Initialized with parser(s)")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("msg=\"Initialized with parser(s)\" parsers={parser_name_list:?}")
                }
            }
        }
        new
    }
}

impl Configuration {
    pub fn url_list(&self) -> &[Url] {
        self.url_list.as_slice()
    }

    pub fn has_url(&mut self, url: &Url) -> bool {
        self.url_list.contains(url)
    }

    pub fn has_url_scheme(&mut self, url: &Url) -> bool {
        self.url_list
            .iter()
            .any(|inner_url| inner_url.scheme() == url.scheme())
    }

    pub fn with_url(mut self, url: Url) -> Result<Self, Error> {
        self.add_url(url)?;
        Ok(self)
    }

    pub fn add_url(&mut self, url: Url) -> Result<(), Error> {
        let scheme = url.scheme().to_string();
        let maybe_loader_name = if let Some(loader) = self
            .loader_list
            .iter()
            .find(|loader| loader.scheme_list().contains(&scheme))
        {
            self.url_list.push(url.clone());
            Some(format!("{loader}"))
        } else {
            #[allow(unused_mut)]
            let mut included_loader_list: Vec<Box<dyn Loader>> = Vec::new();

            #[cfg(feature = "env")]
            included_loader_list.push(Box::new(crate::loader::env::Env::new()));

            #[cfg(feature = "fs")]
            included_loader_list.push(Box::new(crate::loader::fs::Fs::new()));

            included_loader_list
                .into_iter()
                .find(|loader| loader.scheme_list().contains(&scheme))
                .map(|loader| {
                    let name = format!("{loader}");
                    self.add_boxed_loader(loader);
                    self.url_list.push(url.clone());
                    name
                })
        };
        maybe_loader_name.map(|_loader_name| {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(url=%url, loader=_loader_name, "Added configuration URL");
                } else if #[cfg(feature = "logging")] {
                    log::debug!("msg=\"Added configuration URL\", url=\"{url}\" loader={_loader_name:?}");
                }
            }
            Ok(())
        }).unwrap_or(Err(LoaderError::LoaderNotFound { scheme, url }.into()))
    }

    pub fn remove_url(&mut self, url: &Url) -> bool {
        let mut result = false;
        while let Some(index) = self.url_list.iter().position(|inner_url| inner_url == url) {
            self.url_list.remove(index);
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(url=%url, "Removed URL")
                } else if #[cfg(feature = "logging")] {
                    log::debug!("msg=\"Removed URL\" url=\"{url}\"")
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
        L: Loader + 'static,
    {
        self.add_boxed_loader(Box::new(loader));
        self
    }

    pub fn add_loader<L>(&mut self, loader: L)
    where
        L: Loader + 'static,
    {
        self.add_boxed_loader(Box::new(loader));
    }

    pub fn with_boxed_loader(mut self, loader: Box<dyn Loader>) -> Self {
        self.add_boxed_loader(loader);
        self
    }

    pub fn add_boxed_loader(&mut self, loader: Box<dyn Loader>) {
        cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::debug!(
                    loader=%loader,
                    schema_list=?loader.scheme_list(),
                    "Added configuration loader"
                );
            } else if #[cfg(feature = "logging")] {
                log::debug!(
                    "msg=\"Added configuration loader\" loader=\"{loader}\" schema_list={:?}",
                    loader.scheme_list()
                );
            }
        }
        self.loader_list.push(loader);
    }

    pub fn remove_loader_and_urls<S: AsRef<str>>(
        &mut self,
        scheme: S,
    ) -> Option<(Box<dyn Loader>, Vec<Url>)> {
        let scheme_string = scheme.as_ref().to_string();
        if let Some(index) = self
            .loader_list
            .iter()
            .position(|loader| loader.scheme_list().contains(&scheme_string))
        {
            let loader = self.loader_list.swap_remove(index);
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::debug!(
                        loader=%loader,
                        schema_list=?loader.scheme_list(),
                        "Removed configuration loader"
                    );
                } else if #[cfg(feature = "logging")] {
                    log::debug!(
                        "message=\"Removed configuration loader\" loader=\"{loader}\" schema_list={:?}",
                        loader.scheme_list()
                    );
                }
            }
            Some((loader, self.remove_scheme(scheme)))
        } else {
            None
        }
    }

    pub fn load(
        &self,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, Error> {
        load(
            self.url_list.as_slice(),
            self.loader_list.as_slice(),
            self.maybe_whitelist.as_deref(),
            skip_soft_errors,
        )
        .map_err(Error::from)
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
        P: Parser + 'static,
    {
        self.add_parser(parser);
        self
    }

    pub fn add_parser<P>(&mut self, parser: P)
    where
        P: Parser + 'static,
    {
        self.add_boxed_parser(Box::new(parser));
    }

    pub fn with_boxed_parser(mut self, parser: Box<dyn Parser>) -> Self {
        self.add_boxed_parser(parser);
        self
    }

    pub fn add_boxed_parser(&mut self, parser: Box<dyn Parser>) {
        cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::debug!(
                    parser=%parser,
                    format_list=?parser.supported_format_list(),
                    "Added configuration parser"
                );
            } else if #[cfg(feature = "logging")] {
                log::debug!(
                    "msg=\"Added configuration parser\" parser=\"{parser}\" format_list={:?}",
                    parser.supported_format_list()
                );
            }
        }
        self.parser_list.push(parser);
    }

    pub fn remove_parser<F: AsRef<str>>(&mut self, format: F) -> Vec<Box<dyn Parser>> {
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
                    tracing::debug!(
                        parser=%parser,
                        format_list=?parser.supported_format_list(),
                        "Removed configuration parser"
                    );
                } else if #[cfg(feature = "logging")] {
                    log::debug!(
                        "msg=\"Removed configuration parser\" parser=\"{parser}\" format_list={:?}",
                        parser.supported_format_list()
                    );
                }
            }
            parser_list.push(parser)
        }
        parser_list
    }

    pub fn load_and_parse(
        &self,
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, Error> {
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

    pub fn load_whitelist_from_env<K: AsRef<str>>(&mut self, key: K) -> Result<(), Error> {
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
                Error::Other(anyhow!("Invalid key or the value is not set: {}", error))
            })?;
        if whitelist.is_empty() {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::warn!(key=key.as_ref(), "Whitelist environment-variable is set to empty")
                } else if #[cfg(feature = "logging")] {
                    log::warn!("msg=\"Whitelist environment-variable is set to empty\" key={:?}", key.as_ref())
                }
            }
        } else {
            cfg_if! {
                if #[cfg(feature = "tracing")] {
                    tracing::info!(key=key.as_ref(), "Set whitelist from environment-variable")
                } else if #[cfg(feature = "logging")] {
                    log::info!("msg=\"Set whitelist from environment-variable\" key={:?}", key.as_ref())
                }
            }
        }
        self.set_whitelist(whitelist.as_ref());
        Ok(())
    }

    pub fn set_whitelist_from_env<K: AsRef<str>>(mut self, key: K) -> Result<Self, Error> {
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
                log::debug!("msg=\"Added to whitelist\" name={name:?}")
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
    pub fn load_parse_merge(&self, skip_soft_errors: bool) -> Result<Vec<(String, Input)>, Error> {
        let mut parsed = self.load_and_parse(skip_soft_errors)?;
        merge(parsed.as_mut())
    }

    pub fn load_parse_merge_validate(
        &self,
        schema_list: &[(String, InputSchemaType)],
        skip_soft_errors: bool,
    ) -> Result<Vec<(String, Input)>, Error> {
        let mut merged = self.load_parse_merge(skip_soft_errors)?;
        validate(merged.as_mut(), schema_list)
    }
}

pub fn load(
    url_list: &[Url],
    loader_list: &[Box<dyn Loader>],
    maybe_whitelist: Option<&[String]>,
    skip_soft_errors: bool,
) -> Result<Vec<(String, Vec<ConfigurationEntity>)>, LoaderError> {
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
                    .load(url, maybe_whitelist, skip_soft_errors)
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
                Err(LoaderError::LoaderNotFound {
                    scheme: scheme_string,
                    url: url.clone(),
                })
            }
        })
        .map(|_| result)
}

pub fn parse(
    plugin_configuration_list: &mut [(String, Vec<ConfigurationEntity>)],
    parser_list: &[Box<dyn Parser>],
) -> Result<(), Error> {
    plugin_configuration_list
        .iter_mut()
        .try_for_each(|(plugin_name, configuration_list)| {
            configuration_list
                .iter_mut()
                .try_for_each(|configuration| {
                    if configuration.maybe_parsed_contents().is_none() {
                        let parsed =
                            configuration.parse_contents(parser_list).map_err(|error| {
                                Error::Parse {
                                    plugin_name: plugin_name.to_string(),
                                    url: configuration.url().clone(),
                                    item: configuration.item().clone().into(),
                                    source: error,
                                }
                            })?;
                        configuration.set_parsed_contents(parsed);
                    }
                    Ok::<_, Error>(())
                })?;
            Ok::<_, Error>(())
        })
}

pub fn merge(
    plugin_configuration_list: &[(String, Vec<ConfigurationEntity>)],
) -> Result<Vec<(String, Input)>, Error> {
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
) -> Result<Vec<(String, Input)>, Error> {
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
            Ok::<_, Error>(())
        })
        .map(|_| result)
}
