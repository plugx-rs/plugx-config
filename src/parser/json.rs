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
//! let parsed: Input = parser.parse(bytes.as_slice()).unwrap();
//!
//! assert!(parsed.is_map());
//! let map = parsed.as_map();
//! assert!(
//!     map.len() == 2 &&
//!     map.contains_key("foo") &&
//!     map.contains_key("hello")
//! );
//! ```

use crate::parser::ConfigurationParser;
use anyhow::anyhow;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Copy, Default)]
pub struct ConfigurationParserJson;

impl Display for ConfigurationParserJson {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON parser")
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
}

impl ConfigurationParser for ConfigurationParserJson {
    fn name(&self) -> String {
        "JSON".into()
    }

    fn supported_format_list(&self) -> Vec<String> {
        ["json".into()].into()
    }

    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        serde_json::from_slice(bytes).map_err(|error| anyhow!(error))
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(serde_json::from_slice::<serde_json::Value>(bytes).is_ok())
    }
}
