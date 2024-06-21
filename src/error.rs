//! All possible error types.

use url::Url;

/// Main error wrapper.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Errors from [crate::loader::Error].
    #[error(transparent)]
    Load {
        #[from]
        source: crate::loader::Error,
    },
    /// Errors from [crate::parser::Error].
    #[error("Error in parsing `{plugin_name}` configuration from `{url}` for `{item}`")]
    Parse {
        plugin_name: String,
        url: Url,
        item: Box<String>,
        source: crate::parser::Error,
    },
    /// Errors from [plugx_input::schema::InputSchemaError]
    #[error(transparent)]
    Validate {
        #[from]
        source: plugx_input::schema::InputSchemaError,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<url::ParseError> for Error {
    fn from(url_parser_error: url::ParseError) -> Self {
        Self::Other(anyhow::anyhow!(url_parser_error))
    }
}

impl From<(String, Url, String, crate::parser::Error)> for Error {
    fn from(
        (plugin_name, url, item, parser_error): (String, Url, String, crate::parser::Error),
    ) -> Self {
        Self::Parse {
            plugin_name,
            url,
            item: Box::new(item),
            source: parser_error,
        }
    }
}
