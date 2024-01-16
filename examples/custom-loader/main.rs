use anyhow::anyhow;
use cfg_if::cfg_if;
use plugx_config::{
    entity::ConfigurationEntity,
    error::ConfigurationLoadError,
    ext::{
        anyhow::{Context, Result},
        serde::Deserialize,
    },
    loader::{deserialize_query_string, ConfigurationLoader, SoftErrors},
    Configuration, Url,
};
use std::{fs, io::ErrorKind, path::PathBuf};

#[derive(Debug, Default)]
struct MyLoader;

#[derive(Debug, Deserialize)]
struct MyOptions {
    name: PathBuf,
    // `SoftErrors<_>` is a wrapper on top of my error enum
    // A custom deserializer is implemented for `SoftErrors<_>` so its value can be string "all" or one of enum fields.
    #[serde(default)]
    soft_errors: SoftErrors<MySoftErrors>,
}

// It must have `Debug` & `Deserialize`
// `PartialEq` helps by using `SoftErrors<MySoftErrors>.contain(&MySoftErrors::<FIELD>)` to check if error exists.
#[derive(Debug, PartialEq, Deserialize)]
enum MySoftErrors {
    NotFound,
}

//
impl ConfigurationLoader for MyLoader {
    // For logging:
    fn name(&self) -> String {
        "Home".into()
    }

    // So any URL stating with "cfg" and "config" is assigned to this loader
    // For example:
    //    cfg://?name=MY_APP
    //    config://?name=APP_NAME&soft-errors=not-found
    fn scheme_list(&self) -> Vec<String> {
        ["cfg".into(), "config".into()].into()
    }

    fn load(
        &self,
        url: &Url,
        maybe_whitelist: Option<&[String]>,
        skip_soft_errors: bool,
    ) -> std::result::Result<Vec<(String, ConfigurationEntity)>, ConfigurationLoadError> {
        // Deserialize given URL query-string with helper function `deserialize_query_string`:
        let url_options: MyOptions = deserialize_query_string(self.name(), url)?;
        let config_dir = match dirs::config_dir() {
            Some(config_dir) => config_dir,
            None => {
                return if skip_soft_errors && url_options.soft_errors.skip_all() {
                    Ok(Vec::new())
                } else {
                    Err(ConfigurationLoadError::Other(anyhow!(
                        "Could not get system config directory"
                    )))
                }
            }
        };
        // Logging (if enabled via Cargo features `tracing` or `logging`):
        cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::info!(directory=?config_dir, "Detected config directory");
            } else if #[cfg(feature = "logging")] {
                log::info!("msg=\"Detected config directory\" directory={config_dir:?}");
            }
        }
        let sub_path = config_dir.join(url_options.name);
        // Check if directory exists:
        if !sub_path.is_dir() {
            return if skip_soft_errors
                && (url_options.soft_errors.skip_all()
                    || url_options.soft_errors.contains(&MySoftErrors::NotFound))
            {
                Ok(Vec::new())
            } else {
                Err(ConfigurationLoadError::InvalidUrl {
                    loader: self.name(),
                    url: url.clone().into(),
                    source: anyhow!("Path ({sub_path:?}) is not a directory or does not exist"),
                })
            };
        };
        // Get list of files:
        match sub_path.read_dir() {
            Ok(file_iter) => {
                let mut config_list = Vec::new();
                file_iter
                    // Check if result is readable:
                    .filter_map(|read_result| read_result.ok())
                    // Convert them to `PathBuf`
                    .map(|entry| entry.path())
                    // Keep regular files:
                    .filter(|path| path.is_file())
                    // Keep regular files with names (for example do not keep files starting with dot):
                    .filter(|path| path.file_stem().is_some())
                    // Keep files with extension:
                    .filter(|path| path.extension().is_some())
                    .try_for_each(|path| {
                        let contents = fs::read_to_string(&path).map_err(|error| {
                            ConfigurationLoadError::Load {
                                loader: self.name().into(),
                                url: url.clone().into(),
                                description: format!("Could not read file contents from {path:?}")
                                    .into(),
                                source: anyhow!(error),
                            }
                        })?;
                        let plugin_name =
                            path.file_stem().unwrap().to_str().unwrap().to_lowercase();
                        // Check whitelist:
                        if let Some(whitelist) = maybe_whitelist {
                            if !whitelist.contains(&plugin_name) {
                                return Ok::<_, ConfigurationLoadError>(());
                            }
                        }
                        let format = path.extension().unwrap().to_str().unwrap().to_lowercase();
                        let config_entity = ConfigurationEntity::new(
                            path.to_str().unwrap(),
                            url.clone(),
                            &plugin_name,
                            self.name(),
                        )
                        .with_format(format)
                        .with_contents(contents);
                        config_list.push((plugin_name, config_entity));
                        Ok::<_, ConfigurationLoadError>(())
                    })?;
                Ok(config_list)
            }
            Err(error) => {
                if skip_soft_errors
                    && (url_options.soft_errors.skip_all()
                        || (error.kind() == ErrorKind::NotFound
                            && url_options.soft_errors.contains(&MySoftErrors::NotFound)))
                {
                    Ok(Vec::new())
                } else {
                    Err(ConfigurationLoadError::Load {
                        loader: self.name().into(),
                        url: url.clone().into(),
                        description: format!("Could not read directory contents from {sub_path:?}")
                            .into(),
                        source: anyhow!(error),
                    })
                }
            }
        }
    }
}

fn main() -> Result<()> {
    let (_trace, url_list) = get_options_from_cmd_args()?;

    cfg_if::cfg_if! {
        if #[cfg(feature = "tracing")] {
            tracing_subscriber::fmt()
                .pretty()
                .with_max_level(if _trace {
                    tracing::Level::TRACE
                } else {
                    tracing::Level::INFO
                })
                .with_line_number(false)
                .with_file(false)
                .without_time()
                .init();
        } else if #[cfg(feature = "logging")] {
            env_logger::builder()
                .filter_level(if _trace {
                    log::LevelFilter::Trace
                } else {
                    log::LevelFilter::Info
                })
                .format_timestamp(None)
                .init();
        }
    }

    let mut configuration = Configuration::new();
    // Set our custom loader:
    configuration.add_loader(MyLoader::default());
    url_list
        .into_iter()
        .try_for_each(|url| configuration.add_url(url))?;
    // Load & Parse & Merge & print:
    configuration
        .load_parse_merge(true)?
        .iter()
        .for_each(|(plugin_name, configuration)| println!("{plugin_name}: {configuration}"));

    Ok(())
}

fn get_options_from_cmd_args() -> Result<(bool, Vec<Url>)> {
    std::env::args()
        .skip(1)
        .try_fold((false, Vec::new()), |(mut trace, mut list), arg| {
            if arg == "--trace" {
                trace = true;
            } else {
                list.push(
                    Url::parse(&arg).with_context(|| format!("Could not parse URL `{arg}`"))?,
                );
            }
            Ok((trace, list))
        })
}
