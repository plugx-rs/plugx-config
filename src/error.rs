//! All possible error types.

#[doc(inline)]
pub use crate::loader::ConfigurationLoadError;
#[doc(inline)]
pub use crate::parser::ConfigurationParserError;

use url::Url;

/// Main error wrapper.
#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    /// Errors from [ConfigurationLoadError].
    #[error(transparent)]
    Load {
        #[from]
        source: ConfigurationLoadError,
    },
    /// Errors from [ConfigurationParserError].
    #[error("Error in parsing `{plugin_name}` configuration from `{url}`")]
    Parse {
        plugin_name: String,
        url: Url,
        source: ConfigurationParserError,
    },
    /// Errors from [plugx_input::validation::InputValidateError]
    #[error(transparent)]
    Validate {
        #[from]
        source: plugx_input::schema::InputSchemaError,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
