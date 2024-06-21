//! TOML configuration parser.
//!
//! This is only usable if you enabled `toml` Cargo feature.
//!
//! ### Example
//! ```rust
//! use plugx_config::parser::{Parser, toml::Toml};
//! use plugx_input::Input;
//!
//! let bytes = br#"
//! hello=["w", "o", "l", "d"]
//!
//! [foo]
//! bar = {baz = "Qux", abc = 3.14}
//! xyz = false
//! "#;
//!
//! let parser = Toml::new();
//! let parsed: Input = parser.parse(bytes.as_slice()).unwrap();
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

#[derive(Default, Debug, Clone, Copy)]
pub struct Toml;

impl Toml {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Display for Toml {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TOML")
    }
}

impl Parser for Toml {
    fn supported_format_list(&self) -> Vec<String> {
        ["toml".into()].into()
    }

    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        String::from_utf8(bytes.to_vec())
            .map_err(|error| anyhow!("Could not decode contents to UTF-8 ({error})"))
            .and_then(|text| {
                toml::from_str(text.as_str())
                    .map(|parsed: Input| {
                        cfg_if! {
                            if #[cfg(feature = "tracing")] {
                                tracing::trace!(
                                    input=text,
                                    output=%parsed,
                                    "Parsed TOML contents"
                                );
                            } else if #[cfg(feature = "logging")] {
                                log::trace!(
                                    "msg=\"Parsed TOML contents\" input={text:?} output={:?}",
                                    parsed.to_string()
                                );
                            }
                        }
                        parsed
                    })
                    .map_err(|error| anyhow!(error))
            })
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        Some(if let Ok(text) = String::from_utf8(bytes.to_vec()) {
            toml::from_str::<toml::Value>(text.as_str()).is_ok()
        } else {
            false
        })
    }
}
