#![doc = include_str!("../README.md")]

pub mod entity;
pub mod error;
pub mod loader;
pub mod parser;

pub mod configuration;
pub use configuration::Configuration;

pub mod ext {
    //! Extern other crates.

    pub extern crate anyhow;
    pub extern crate plugx_input;
    pub extern crate serde;
    pub extern crate url;
}

mod logging;
