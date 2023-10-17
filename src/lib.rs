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

pub mod ext {
    //! Extern other crates.

    pub extern crate anyhow;
    pub extern crate plugx_input;
    pub extern crate serde;
    pub extern crate url;
}

mod configuration;
mod logging;

#[cfg(test)]
mod tests {
    use crate::logging::enable_logging;
    use crate::Configuration;
    use std::{collections::HashMap, env, fs};
    use tempdir::TempDir;
    use url::Url;

    #[test]
    fn smoke() {
        enable_logging();

        // In this example we're going to load our plugins' configurations from
        // a directory and environment-variables.
        // Here we have 4 plugins `foo`, `bar`, `baz`, and `qux`.

        // Set some configurations in environment-variables:
        env::set_var("APP_NAME__FOO__SERVER__ADDRESS", "127.0.0.1");
        env::set_var("APP_NAME__BAR__SQLITE__FILE", "/path/to/app.db");
        env::set_var("APP_NAME__BAZ__LOGGING__LEVEL", "debug");
        env::set_var("APP_NAME__QUX__HTTPS__INSECURE", "false");

        // Create a temporary directory `/tmp/.../etc/app-name` (which will be removed after running our example)
        let root_tmp = TempDir::new("example").expect("Create temporary directory");
        let cfg_dir = root_tmp.path().join("etc").join("app.d");
        fs::create_dir_all(cfg_dir.clone()).unwrap();
        // Write some configurations inside and example directory `/tmp/.../etc/app.d/`:
        fs::write(
            cfg_dir.join("foo.env"),
            "SERVER__PORT=8080 # This is a comment",
        )
        .unwrap();
        fs::write(
            cfg_dir.join("bar.json"),
            "{\"sqlite\": {\"recreate\": true}}",
        )
        .unwrap();
        fs::write(
            cfg_dir.join("baz.toml"),
            "[logging]\noutput_serialize_format = \"json\"",
        )
        .unwrap();
        fs::write(
            cfg_dir.join("qux.yaml"),
            "https:\n  follow_redirects: false",
        )
        .unwrap();

        // Create a URL for our environment-variables configuration:
        let env_url: Url = "env://?prefix=APP_NAME__&key_separator=__"
            .parse()
            .expect("Valid URL");
        // Create a URL for our `/tmp/.../etc/app.d/` directory:
        // `skippable` query-string key is list of skippable error names.
        // Here we want to skip `not found` error if the directory does not exists:
        let file_url: Url = format!("file://{}?skippable[0]=notfound", cfg_dir.to_str().unwrap())
            .parse()
            .expect("Valid URL");

        // We want to check our plugins' configurations for them but we do not know what they want!
        // We can load them and ask them what keys and values they expect to have before loading
        // and checking configurations.
        // Here for example we asked them about their configuration and collected their rules in
        // JSON format:
        let rules_json = r#"
        {
            "foo": {
                "type": "static_map",
                "definitions": {
                    "server": {
                        "definition": {
                            "type": "static_map",
                            "definitions": {
                                "address": {"definition": {"type": "ip"}},
                                "port": {"definition": {"type": "integer", "range": {"min": 1, "max": 65535}}}
                            }
                        }
                    }
                }
            },
            "bar": {
                "type": "static_map",
                "definitions": {
                    "sqlite": {
                        "definition": {
                            "type": "static_map",
                            "definitions": {
                                "recreate": {"definition": {"type": "boolean"}, "default": true},
                                "file": {"definition": {"type": "path", "file_type": "file", "access": ["write"]}}
                            }
                        }
                    }
                }
            },
            "baz": {
                "type": "static_map",
                "definitions": {
                    "logging": {
                        "definition": {
                            "type": "static_map",
                            "definitions": {
                                "level": {"definition": {"type": "log_level"}, "default": "info"},
                                "output_serialize_format": {"definition": {"type": "enum", "items": ["json", "logfmt"]}}
                            }
                        }
                    }
                }
            },
            "qux": {
                "type": "static_map",
                "definitions": {
                    "https": {
                        "definition": {
                            "type": "static_map",
                            "definitions": {
                                "insecure": {"definition": {"type": "boolean"}},
                                "follow_redirects": {"definition": {"type": "boolean"}}
                            }
                        }
                    }
                }
            }
        }
        "#;
        let rules: HashMap<_, _> = serde_json::from_str(rules_json).unwrap();

        // Here's the actual work:
        let mut configuration = Configuration::default()
            .with_url(env_url)
            .with_url(file_url);
        let apply_akippable_errors = true;
        configuration
            .try_load_parse_merge_validate(apply_akippable_errors, &rules)
            .unwrap();
        configuration
            .configuration()
            .iter()
            .for_each(|(plugin, config)| println!("{plugin}: {config:#}"));
        // Prints:
        //  foo: {"server": {"address": "127.0.0.1", "port": 8080}}
        //  baz: {"logging": {"output_serialize_format": "json", "level": "debug"}}
        //  bar: {"sqlite": {"file": "/path/to/app.db", "recreate": true}}
        //  qux: {"https": {"insecure": false, "follow_redirects": false}}
    }
}
