//! Configuration entity for each plugin.
//!
//! ### Example usage
//! ```rust
//! use std::env::set_var;
//! use plugx_config::{
//!     ext::{url::Url, plugx_input::Input},
//!     entity::ConfigurationEntity,
//!     loader::{ConfigurationLoader, env::ConfigurationLoaderEnv},
//!     parser::{ConfigurationParser, env::ConfigurationParserEnv},
//! };
//!
//! let url = "env://".parse::<Url>().expect("Valid URL");
//! let plugin_name = "foo";
//! let loader = ConfigurationLoaderEnv::new().with_prefix("MY_APP_NAME").with_separator("__");
//! set_var("MY_APP_NAME__FOO__BAR__BAZ", "3.14");
//! set_var("MY_APP_NAME__FOO__QUX", "false");
//! let loaded = loader.try_load(&url, None).unwrap();
//! println!("{loaded:?}");
//! let foo_entity = loaded.get(plugin_name).expect("`foo` value");
//! // Above `loader` actually does this:
//! let loader_name = loader.name();
//! let mut foo_entity2 = ConfigurationEntity::new(url.clone(), plugin_name, loader_name)
//!     .with_format("env")
//!     .with_contents("BAR__BAZ=\"3.14\"\nQUX=\"false\"");
//!
//! assert_eq!(&foo_entity2, foo_entity);
//!
//! // We can pass a list of `ConfigurationParser` to an entity to parse its contents.
//! let parser = ConfigurationParserEnv::new().with_key_separator("__");
//! let parser_list: Vec<Box<dyn ConfigurationParser>> = vec![Box::new(parser)];
//! let input = foo_entity2.parse_contents_mut(&parser_list).unwrap();
//! assert_eq!(input.as_map().get("qux").expect("`qux` value"), &false.into());
//! ```
use crate::parser::{ConfigurationParser, ConfigurationParserError};
use plugx_input::Input;
use std::fmt::{Display, Formatter};
use url::Url;

/// A configuration entity for each plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationEntity {
    loader_name: String,
    url: Url,
    plugin_name: String,
    maybe_format: Option<String>,
    maybe_contents: Option<String>,
    maybe_parsed: Option<Input>,
}

impl ConfigurationEntity {
    /// Constructs a new [ConfigurationEntity].
    ///
    /// It's better to set the format (via [Self::set_format] or [Self::with_format]) and if we
    /// don't and try to parse its  contents, All parsers that support
    /// [ConfigurationParser::is_format_supported] method try to validate the
    /// contents to pick it up for future parsing!
    pub fn new<P, L>(url: Url, plugin_name: P, loader_name: L) -> Self
    where
        P: AsRef<str>,
        L: AsRef<str>,
    {
        Self {
            url,
            plugin_name: plugin_name.as_ref().to_string(),
            loader_name: loader_name.as_ref().to_string(),
            maybe_format: Default::default(),
            maybe_contents: Default::default(),
            maybe_parsed: Default::default(),
        }
    }

    pub fn set_format<F: AsRef<str>>(&mut self, format: F) {
        self.maybe_format = Some(format.as_ref().to_string());
    }

    pub fn with_format<F: AsRef<str>>(mut self, format: F) -> Self {
        self.set_format(format);
        self
    }

    pub fn set_contents<C: AsRef<str>>(&mut self, contents: C) {
        self.maybe_contents = Some(contents.as_ref().to_string());
    }

    pub fn with_contents<C: AsRef<str>>(mut self, contents: C) -> Self {
        self.set_contents(contents);
        self
    }

    pub fn set_parsed_contents<I: Into<Input>>(&mut self, contents: I) {
        self.maybe_parsed = Some(contents.into());
    }

    pub fn with_parsed_contents<I: Into<Input>>(mut self, contents: I) -> Self {
        self.set_parsed_contents(contents);
        self
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn url_mut(&mut self) -> &mut Url {
        &mut self.url
    }

    pub fn plugin_name(&self) -> &String {
        &self.plugin_name
    }

    pub fn plugin_name_mut(&mut self) -> &mut String {
        &mut self.plugin_name
    }

    pub fn maybe_format(&self) -> Option<&String> {
        self.maybe_format.as_ref()
    }

    pub fn maybe_format_mut(&mut self) -> &mut Option<String> {
        &mut self.maybe_format
    }

    pub fn maybe_contents(&self) -> Option<&String> {
        self.maybe_contents.as_ref()
    }

    pub fn maybe_contents_mut(&mut self) -> &mut Option<String> {
        &mut self.maybe_contents
    }

    pub fn maybe_parsed_contents(&self) -> Option<&Input> {
        self.maybe_parsed.as_ref()
    }

    pub fn maybe_parsed_contents_mut(&mut self) -> &mut Option<Input> {
        &mut self.maybe_parsed
    }

    /// We have to call it after calling [Self::set_contents] or [Self::with_contents] and If no
    /// contents is set, It yields [None] too.
    pub fn guess_format(&self, parser_list: &[Box<dyn ConfigurationParser>]) -> Option<String> {
        let contents = self.maybe_contents()?;
        let bytes = contents.as_bytes();
        if let Some(parser) = parser_list
            .iter()
            .find(|parser| parser.is_format_supported(bytes).unwrap_or_default())
        {
            parser.supported_format_list().iter().last().cloned()
        } else {
            None
        }
    }

    pub fn parse_contents(
        &self,
        parser_list: &[Box<dyn ConfigurationParser>],
    ) -> Result<Input, ConfigurationParserError> {
        let contents = if let Some(contents) = self.maybe_contents() {
            contents
        } else {
            return Ok(Input::new_map());
        };
        let format = if let Some(format) = self.maybe_format() {
            format.clone()
        } else if let Some(format) = self.guess_format(parser_list) {
            format
        } else {
            return Err(ConfigurationParserError::ParserNotFound);
        };
        if let Some(parser) = parser_list
            .iter()
            .find(|parser| parser.supported_format_list().contains(&format))
        {
            parser.try_parse(contents.as_bytes())
        } else {
            Err(ConfigurationParserError::ParserNotFound)
        }
    }

    pub fn parse_contents_mut(
        &mut self,
        parser_list: &[Box<dyn ConfigurationParser>],
    ) -> Result<&mut Input, ConfigurationParserError> {
        let input = self.parse_contents(parser_list)?;
        self.set_parsed_contents(input);
        Ok(self
            .maybe_parsed_contents_mut()
            .as_mut()
            .expect("input has been set!"))
    }
}

impl Display for ConfigurationEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(format!("Configuration entity for {}", self.plugin_name).as_str())
    }
}
