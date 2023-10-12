//! Custom configuration parser with [Fn].
//!
//! ### Example
//! In the following example we implement a complete [HJSON](https://hjson.github.io/) parser.
//!
//! ```rust
//! use plugx_config::{
//!     error::ConfigurationParserError,
//!     parser::{ConfigurationParser, closure::ConfigurationParserFn}
//! };
//!
//! let parser_fn = |bytes: &[u8]| {
//!     deser_hjson::from_slice(bytes).map_err(|error| ConfigurationParserError::Parse {
//!         data: String::from_utf8_lossy(bytes).to_string(),
//!         parser: "HJSON".to_string(),
//!         supported_format_list: ["hjson".into()].into(),
//!         source: error.into(),
//!     })
//! };
//!
//! let parser = ConfigurationParserFn::new("hjson", Box::new(parser_fn));
//! let bytes = br#"
//! {
//!     hello: ["w", "o", "l", "d"]
//!     foo: {
//!         bar: {
//!             baz: Qux
//!             abc: 3.14
//!         }
//!         xyz: false
//!     }
//! }
//! "#;
//! let parsed = parser.try_parse(bytes).unwrap();
//! assert!(
//!     parsed.map_ref().unwrap().len() == 2 &&
//!     parsed.map_ref().unwrap().contains_key("foo") &&
//!     parsed.map_ref().unwrap().contains_key("hello")
//! );
//! let foo = parsed.map_ref().unwrap().get("foo").unwrap();
//! assert!(
//!     foo.map_ref().unwrap().len() == 2 &&
//!     foo.map_ref().unwrap().contains_key("bar") &&
//!     foo.map_ref().unwrap().contains_key("xyz")
//! );
//! let bar = foo.map_ref().unwrap().get("bar").unwrap();
//! assert_eq!(bar.map_ref().unwrap().get("baz").unwrap(), &"Qux".into());
//! assert_eq!(bar.map_ref().unwrap().get("abc").unwrap(), &3.14.into());
//! let xyz = foo.map_ref().unwrap().get("xyz").unwrap();
//! assert_eq!(xyz, &false.into());
//! let list = ["w", "o", "l", "d"].into();
//! assert_eq!(parsed.map_ref().unwrap().get("hello").unwrap(), &list);
//! ```
//!

use crate::error::ConfigurationParserError;
use crate::parser::{BoxedModifierFn, ConfigurationParser};
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

/// A `|&[u8]| -> Result<Input, ConfigurationParserError>` [Fn] to parse contents.
pub type BoxedParserFn =
    Box<dyn Fn(&[u8]) -> Result<Input, ConfigurationParserError> + Send + Sync>;
/// A `|&[u8]| -> Option<bool>` [Fn] to validate contents.
pub type BoxedValidatorFn = Box<dyn Fn(&[u8]) -> Option<bool> + Send + Sync>;

/// Builder struct.
pub struct ConfigurationParserFn {
    parser: BoxedParserFn,
    validator: BoxedValidatorFn,
    supported_format_list: Vec<String>,
    maybe_modifier: Option<BoxedModifierFn>,
}

impl Display for ConfigurationParserFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            self.supported_format_list
                .iter()
                .last()
                .map_or("unknown", |format| format.as_str()),
        )
    }
}

impl Debug for ConfigurationParserFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationParserFn")
            .field("supported_format_list", &self.supported_format_list)
            .finish()
    }
}

impl ConfigurationParserFn {
    pub fn new<F: AsRef<str>>(supported_format: F, parser: BoxedParserFn) -> Self {
        Self {
            parser,
            validator: Box::new(|_| None),
            supported_format_list: [supported_format.as_ref().to_string()].to_vec(),
            maybe_modifier: None,
        }
    }

    pub fn set_parser(&mut self, parser: BoxedParserFn) {
        self.parser = parser;
    }

    pub fn with_parser(mut self, parser: BoxedParserFn) -> Self {
        self.set_parser(parser);
        self
    }

    pub fn set_validator(&mut self, validator: BoxedValidatorFn) {
        self.validator = validator;
    }

    pub fn with_validator(mut self, validator: BoxedValidatorFn) -> Self {
        self.set_validator(validator);
        self
    }

    pub fn set_format_list<N: AsRef<str>>(&mut self, format_list: &[N]) {
        self.supported_format_list = format_list
            .iter()
            .map(|format| format.as_ref().to_string())
            .collect();
    }

    pub fn with_format_list<N: AsRef<str>>(mut self, format_list: &[N]) -> Self {
        self.set_format_list(format_list);
        self
    }

    pub fn set_modifier(&mut self, modifier: BoxedModifierFn) {
        self.maybe_modifier = Some(modifier);
    }

    pub fn with_modifier(mut self, modifier: BoxedModifierFn) -> Self {
        self.set_modifier(modifier);
        self
    }
}

impl ConfigurationParser for ConfigurationParserFn {
    fn supported_format_list(&self) -> Vec<String> {
        self.supported_format_list.clone()
    }

    fn try_parse(&self, bytes: &[u8]) -> Result<Input, ConfigurationParserError> {
        let mut result = (self.parser)(bytes)?;
        if let Some(ref modifier) = self.maybe_modifier {
            modifier(bytes, &mut result)?;
        }
        Ok(result)
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        (self.validator)(bytes)
    }
}
