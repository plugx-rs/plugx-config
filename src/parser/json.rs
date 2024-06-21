//! JSON configuration parser.
//!
//! This is only usable if you enabled `json` Cargo feature.
//!
//! ### Example
//! ```rust
//! use plugx_config::parser::{Parser, json::Json};
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
//! let parser = Json::new();
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

use crate::parser::Parser;
use anyhow::anyhow;
use cfg_if::cfg_if;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

#[derive(Clone, Copy, Default)]
pub struct Json;

impl Display for Json {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("JSON")
    }
}

impl Debug for Json {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationParserJson").finish()
    }
}

impl Json {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Parser for Json {
    fn supported_format_list(&self) -> Vec<String> {
        ["json".into()].into()
    }

    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        serde_json::from_slice(bytes)
            .map(|parsed: Input| {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::trace!(
                            input=String::from_utf8_lossy(bytes).to_string(),
                            output=%parsed,
                            "Parsed JSON contents"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "msg=\"Parsed JSON contents\" input={:?} output={:?}",
                            String::from_utf8_lossy(bytes).to_string(),
                            parsed.to_string()
                        );
                    }
                }
                parsed
            })
            .map_err(|error| anyhow!(error))
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(serde_json::from_slice::<serde_json::Value>(bytes).is_ok())
    }
}
