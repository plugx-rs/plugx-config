use crate::parser::{ConfigurationParser, ConfigurationParserError};

use plugx_input::Input;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, PartialEq)]
pub struct ConfigurationEntity {
    loader_name: String,
    source: String,
    plugin_name: String,
    maybe_format: Option<String>,
    maybe_contents: Option<String>,
    maybe_parsed: Option<Input>,
}

impl ConfigurationEntity {
    pub fn new<S, P, L>(source: S, plugin_name: P, loader_name: L) -> Self
    where
        S: AsRef<str>,
        P: AsRef<str>,
        L: AsRef<str>,
    {
        Self {
            source: source.as_ref().to_string(),
            plugin_name: plugin_name.as_ref().to_string(),
            loader_name: loader_name.as_ref().to_string(),
            maybe_format: None,
            maybe_contents: None,
            maybe_parsed: None,
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

    pub fn source(&self) -> &String {
        &self.source
    }

    pub fn source_mut(&mut self) -> &mut String {
        &mut self.source
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

    pub fn maybe_parsed(&self) -> Option<&Input> {
        self.maybe_parsed.as_ref()
    }

    pub fn maybe_parsed_mut(&mut self) -> &mut Option<Input> {
        &mut self.maybe_parsed
    }

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

    pub fn parse(
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
            parser.parse_and_modify_with_logging(contents.as_bytes())
        } else {
            Err(ConfigurationParserError::ParserNotFound)
        }
    }
}

impl Display for ConfigurationEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.source, f)
    }
}
