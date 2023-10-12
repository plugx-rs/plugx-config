pub use crate::loader::ConfigurationLoadError;
pub use crate::parser::ConfigurationParserError;

#[derive(Debug, thiserror::Error)]
pub enum ConfigurationError {
    #[error(transparent)]
    Load {
        #[from]
        source: ConfigurationLoadError,
    },
    #[error("Error in parsing `{plugin_name}` configuration from `{configuration_source}`")]
    Parse {
        plugin_name: String,
        configuration_source: String,
        source: ConfigurationParserError,
    },
    #[error(transparent)]
    Validate {
        #[from]
        source: plugx_input::validation::InputValidateError,
    },
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ConfigurationError {
    pub fn is_skippable(&self) -> bool {
        if let Self::Load { source, .. } = self {
            source.is_skippable()
        } else {
            false
        }
    }
}
