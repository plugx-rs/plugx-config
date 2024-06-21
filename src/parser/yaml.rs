//! YAML configuration parser.
//!
//! This is only usable if you enabled `yaml` Cargo feature.
//!
//! ### Example
//! ```rust
//! use plugx_config::parser::{Parser, yaml::Yaml};
//! use plugx_input::Input;
//!
//! let bytes = br#"
//! hello:
//!   - "w"
//!   - "o"
//!   - "l"
//!   - "d"
//! foo:
//!   bar:
//!     baz: Qux
//!     abc: 3.14
//!   xyz: false
//! "#;
//!
//! let parser = Yaml::new();
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

use crate::parser::Parser;
use anyhow::anyhow;
use cfg_if::cfg_if;
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

#[derive(Default, Debug, Copy, Clone)]
pub struct Yaml;

impl Display for Yaml {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("YAML")
    }
}

impl Yaml {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Parser for Yaml {
    fn supported_format_list(&self) -> Vec<String> {
        ["yml".into(), "yaml".into()].into()
    }

    fn try_parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        serde_yaml::from_slice(bytes)
            .map(|parsed: Input| {
                cfg_if! {
                    if #[cfg(feature = "tracing")] {
                        tracing::trace!(
                            input=String::from_utf8_lossy(bytes).to_string(),
                            output=%parsed,
                            "Parsed YAML contents"
                        );
                    } else if #[cfg(feature = "logging")] {
                        log::trace!(
                            "msg=\"Parsed YAML contents\" input={:?} output={:?}",
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
        Some(serde_yaml::from_slice::<serde_yaml::Value>(bytes).is_ok())
    }
}
