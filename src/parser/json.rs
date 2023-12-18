//! JSON configuration parser.
//!
//! This is only usable if you enabled `json` Cargo feature.
//!
//! ### Example
//! ```rust
//! use plugx_config::parser::{ConfigurationParser, json::ConfigurationParserJson};
//! use plugx_input::Input;
//!
//! let bytes = br#"
//! {
//!     "hello": ["w", "o", "l", "d"],
//!     "foo": {
//!         "bar": {
//!             "baz": "Qux",
//!             "abc": 3.14
//!         },
//!         "xyz": false
//!     }
//! }
//! "#;
//!
//! let parser = ConfigurationParserJson::new();
//! // You can set nested key separator like this:
//! // parser.set_key_separator("__");
//! let parsed: Input = parser.try_parse(bytes.as_slice()).unwrap();
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

use crate::{
    error::ConfigurationParserError,
    parser::{BoxedModifierFn, ConfigurationParser},
};
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

const NAME: &str = "JSON";
const SUPPORTED_FORMAT_LIST: &[&str] = &["json"];

#[derive(Default)]
pub struct ConfigurationParserJson {
    maybe_modifier: Option<BoxedModifierFn>,
}

impl Display for ConfigurationParserJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(NAME)
    }
}

impl Debug for ConfigurationParserJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationParserJson").finish()
    }
}

impl ConfigurationParserJson {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn set_modifier(&mut self, modifier: BoxedModifierFn) {
        self.maybe_modifier = Some(modifier);
    }

    pub fn with_modifier(mut self, modifier: BoxedModifierFn) -> Self {
        self.set_modifier(modifier);
        self
    }
}

impl ConfigurationParser for ConfigurationParserJson {
    fn supported_format_list(&self) -> Vec<String> {
        SUPPORTED_FORMAT_LIST
            .iter()
            .cloned()
            .map(Into::into)
            .collect()
    }

    fn try_parse(&self, bytes: &[u8]) -> Result<Input, ConfigurationParserError> {
        let mut result =
            serde_json::from_slice(bytes).map_err(|error| ConfigurationParserError::Parse {
                data: String::from_utf8_lossy(bytes).to_string(),
                parser: NAME.to_string(),
                supported_format_list: self.supported_format_list(),
                source: error.into(),
            })?;
        if let Some(ref modifier) = self.maybe_modifier {
            modifier(bytes, &mut result)?;
        }
        Ok(result)
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(serde_json::from_slice::<serde_json::Value>(bytes).is_ok())
    }
}
