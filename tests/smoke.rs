#[test]
fn smoke() {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tracing")] {
            let _ = tracing_subscriber::fmt()
                .json()
                .with_max_level(tracing::Level::TRACE)
                .try_init();
        } else if #[cfg(feature = "logging")] {
            let _ = env_logger::builder()
                .filter_level(log::LevelFilter::max())
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

    let env_url: Url = "env://?prefix=APP_NAME__&key_separator=__"
        .parse()
        .expect("Valid URL");
    let current_dir = env::current_dir()
        .unwrap()
        .join("tests")
        .join("etc")
        .to_str()
        .unwrap()
        .to_string();
    let file_url: Url = format!("file://{current_dir}?skippable[0]=notfound")
        .parse()
        .expect("Valid URL");

    let mut configuration = Configuration::default()
        .with_url(env_url)
        .with_url(file_url);
    let apply_skippable_errors = true;
    configuration
        .try_load_parse_merge(apply_skippable_errors)
        .unwrap();
    configuration
        .configuration()
        .iter()
        .for_each(|(plugin, config)| println!("{plugin}: {config}"));
    // Prints:
    //  foo: {"server": {"address": "127.0.0.1", "port": 8080}}
    //  baz: {"logging": {"output_serialize_format": "json", "level": "debug"}}
    //  bar: {"sqlite": {"file": "/path/to/app.db", "recreate": true}}
    //  qux: {"https": {"insecure": false, "follow_redirects": false}}

    let rules_yml =
        fs::read_to_string(env::current_dir().unwrap().join("tests").join("rules.yml")).unwrap();
    let rules: HashMap<String, InputSchemaType> = serde_yaml::from_str(rules_yml.as_str()).unwrap();
    configuration
        .try_load_parse_merge_validate(apply_skippable_errors, &rules)
        .unwrap();
    env::set_var("APP_NAME__FOO__SERVER__ADDRESS", "127.0.0.1.bad.ip");
    let error = configuration
        .try_load_parse_merge_validate(apply_skippable_errors, &rules)
        .err()
        .unwrap();
    println!("{error}");
    // Prints:
    // [foo][server][address] Could not parse IP address: invalid IP address syntax (expected IP address and got "127.0.0.1.bad.ip")
}
