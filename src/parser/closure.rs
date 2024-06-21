//! Custom configuration parser with [Fn].
//!
//! ### Example
//! In the following example we implement a complete [HJSON](https://hjson.github.io/) parser.
//!
//! ```rust
//! use plugx_config::{
//!     ext::{plugx_input::Input, anyhow::anyhow},
//!     error::Error,
//!     parser::{Parser, closure::Closure}
//! };
//!
//! let parser_fn = |bytes: &[u8]| -> anyhow::Result<Input> {
//!     deser_hjson::from_slice(bytes).map_err(|error| anyhow!(error))
//! };
//!
//! let parser_name = "HJSON";
//! let parser_format = "hjson";
//! let parser = Closure::new("HJSNO", "hjson", Box::new(parser_fn));
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
//! let parsed = parser.parse(bytes).unwrap();
//! assert!(
//!     parsed.as_map().len() == 2 &&
//!     parsed.as_map().contains_key("foo") &&
//!     parsed.as_map().contains_key("hello")
//! );
//! let foo = parsed.as_map().get("foo").unwrap();
//! assert!(
//!     foo.as_map().len() == 2 &&
//!     foo.as_map().contains_key("bar") &&
//!     foo.as_map().contains_key("xyz")
//! );
//! let bar = foo.as_map().get("bar").unwrap();
//! assert_eq!(bar.as_map().get("baz").unwrap(), &"Qux".into());
//! assert_eq!(bar.as_map().get("abc").unwrap(), &3.14.into());
//! let xyz = foo.as_map().get("xyz").unwrap();
//! assert_eq!(xyz, &false.into());
//! let list = ["w", "o", "l", "d"].into();
//! assert_eq!(parsed.as_map().get("hello").unwrap(), &list);
//! ```
//!

use crate::parser::Parser;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

/// A `|&[u8]| -> Result<Input, ConfigurationParserError>` [Fn] to parse contents.
pub type BoxedParserFn = Box<dyn Fn(&[u8]) -> anyhow::Result<Input> + Send + Sync>;
/// A `|&[u8]| -> Option<bool>` [Fn] to validate contents.
pub type BoxedValidatorFn = Box<dyn Fn(&[u8]) -> Option<bool> + Send + Sync>;

/// Builder struct.
pub struct Closure {
    name: String,
    parser: BoxedParserFn,
    validator: BoxedValidatorFn,
    supported_format_list: Vec<String>,
}

impl Display for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name.as_str())
    }
}

impl Debug for Closure {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationParserFn")
            .field("name", &self.name)
            .field("supported_format_list", &self.supported_format_list)
            .finish()
    }
}

impl Closure {
    pub fn new<N: AsRef<str>, F: AsRef<str>>(
        name: N,
        supported_format: F,
        parser: BoxedParserFn,
    ) -> Self {
        Self {
            name: name.as_ref().to_string(),
            parser,
            validator: Box::new(|_| None),
            supported_format_list: [supported_format.as_ref().to_string()].to_vec(),
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
}

impl Parser for Closure {
    fn supported_format_list(&self) -> Vec<String> {
        self.supported_format_list.clone()
    }

    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        (self.parser)(bytes)
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        (self.validator)(bytes)
    }
}
