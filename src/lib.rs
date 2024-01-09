#![doc = include_str!("../README.md")]
#![doc(test(no_crate_inject))]

pub mod entity;
pub mod error;
pub mod loader;
pub mod parser;

#[doc(inline)]
pub use configuration::Configuration;
#[doc(inline)]
pub use error::ConfigurationError;
#[doc(inline)]
/// Final configuration data type
pub use plugx_input::Input;

pub mod ext {
    //! Extern other crates.

    pub extern crate anyhow;
    pub extern crate plugx_input;
    pub extern crate serde;
    pub extern crate url;
}

mod configuration;
mod logging;
