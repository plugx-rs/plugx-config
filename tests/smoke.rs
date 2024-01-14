#[test]
fn smoke() -> Result<(), anyhow::Error> {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tracing")] {
            let _ = tracing_subscriber::fmt()
                .json()
                .with_max_level(tracing::Level::TRACE)
                .without_time()
                .try_init();
        } else if #[cfg(feature = "logging")] {
            let _ = env_logger::builder()
                .filter_level(log::LevelFilter::max())
                .format_timestamp(None)
                .is_test(true)
                .try_init();
        }
    }

    use plugx_config::{ext::url::Url, Configuration};
    use plugx_input::schema::InputSchemaType;
    use std::{collections::HashMap, env, fs};

    env::set_var("APP_NAME__FOO__SERVER__ADDRESS", "127.0.0.1");
    env::set_var("APP_NAME__BAR__SQLITE__FILE", "/path/to/app.db");
    env::set_var("APP_NAME__BAZ__LOGGING__LEVEL", "debug");
    env::set_var("APP_NAME__QUX__HTTPS__INSECURE", "false");

    let env_url: Url = "env://?prefix=APP_NAME".parse().expect("Valid URL");
    let current_dir = env::current_dir()
        .unwrap()
        .join("tests")
        .join("etc")
        .to_str()
        .unwrap()
        .to_string();
    cfg_if::cfg_if! {
        if #[cfg(feature = "tracing")] {
            tracing::trace!(cwd=current_dir);
        } else if #[cfg(feature = "logging")] {
            log::trace!("cwd={current_dir:?}");
        }
    }
    let file_url: Url = format!("file:{current_dir}?strip-slash=true")
        .parse()
        .expect("Valid URL");

    let configuration = Configuration::new().with_url(env_url)?.with_url(file_url)?;
    let skip_soft_errors = true;
    let merged = configuration.load_parse_merge(skip_soft_errors).unwrap();
    merged
        .iter()
        .for_each(|(plugin, config)| println!("{plugin}: {config}"));
    // Prints:
    //  foo: {"server": {"port": 8080, "address": "127.0.0.1"}}
    //  qux: {"https": {"follow_redirects": false, "insecure": false}}
    //  baz: {"logging": {"level": "debug", "output_serialize_format": "json"}}
    //  bar: {"sqlite": {"file": "/path/to/app.db", "recreate": true}}

    let rules_yml =
        fs::read_to_string(env::current_dir().unwrap().join("tests").join("rules.yml")).unwrap();
    let rules: HashMap<String, InputSchemaType> = serde_yaml::from_str(rules_yml.as_str()).unwrap();
    let rules: Vec<(String, InputSchemaType)> = rules.into_iter().collect();
    configuration
        .load_parse_merge_validate(&rules, skip_soft_errors)
        .unwrap();
    env::set_var("APP_NAME__FOO__SERVER__ADDRESS", "127.0.0.1.bad.ip");
    let error = configuration
        .load_parse_merge_validate(&rules, skip_soft_errors)
        .err()
        .unwrap();
    println!("{:#}", plugx_config::ext::anyhow::anyhow!(error));
    // Prints:
    //   [foo][server][address] invalid IP address syntax ("127.0.0.1.bad.ip")
    Ok(())
}
