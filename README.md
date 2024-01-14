# Plugin configuration manager (work-in-progress)
[**Package**](https://crates.io/crates/plugx-config)   |   [**Documentation**](https://docs.rs/plugx-config)   |   [**Repository**](https://github.com/plugx-rs/plugx-config)

## Features
* Loads and parses and merges and validates configurations.
* Loads configuration from URLs.
* Built-in File-system, Environment-variables, and HTTP configuration loaders (Cargo features).
* Built-in Environment-variables, JSON, YAML, and TOML configuration parsers (Cargo features).
* Easy to implement your own configuration loader or parser.
* Ability to skip soft errors for different configuration loaders (e.g. if configuration file does not exist).
* Human-readable errors.
* Easy to reload configuration.
* [log](https://crates.io/crates/log) and [tracing](https://crates.io/crates/tracing) integration.

## Architecture
```text
 Example URLs:
  file:///etc/directory/
  file://file.[json|yml|toml|env]
  http://config-server.tld/path/to/config
  env://?prefix=APP_NAME
  custom://custom?custom=custom
┌──────────────────────────────────────────────┐ ┌──────────────────────┐ ┌───┐ ┌─────────┐
│ FILE SYSTEM                                  │ │                      │ │   │ │         │
│ ┌───────────────┐ ┌────────────────────────┐ │ │                      │ │   │ │         │
│ │/etc/directory/│ │file.[json|yml|toml|env]│ │ │REST API (e.g. Consul)│ │Env│ │ Custom  │
│ └────────────┬──┘ └──┬─────────────────────┘ │ │                      │ │   │ │         │
│              │       │                       │ │                      │ │   │ │         │
└──────────────┼───────┼───────────────────────┘ └─┬────────────────────┘ └─┬─┘ └──┬──────┘
               │       │                           │                        │      │
               │       │                           │                        │      │
               │       │                           │                        │      │
┌──────────────┼───────┼───────────────────────────┼────────────────────────┼──────┼──────┐
│ plugx-config │       │                           │                        │      │      │
│ ┌────────────┼───────┼───────────────────────────┼────────────────────────┼──────┼────┐ │
│ │ Loader     │       │                           │                        │      │    │ │
│ │ ┌──────────▼───────▼───┐ ┌─────────────────────▼─┐ ┌────────────────────▼┐ ┌───▼──┐ │ │
│ │ │       LoaderFs       │ │       LoaderHttp      │ │      LoaderEnv      │ │Custom│ │ │
│ │ └──────────┬───────┬──┬┘ └─────────────────────┬─┘ └────────────┬────────┘ └───┬──┘ │ │
│ │            │       │  │                        │                │              │    │ │
│ └────────────┼───────┼──┼────────────────────────┼────────────────┼──────────────┼────┘ │
│              │       │  │                        │                │              │      │
│              │       │  └───────────────┐        │                │              │      │
│              │       │                  │        │                │              │      │
│ ┌────────────┼───────┼──────────────────┼────────┼────────────────┼──────────────┼────┐ │
│ │ Parser     │       │                  │        │                │              │    │ │
│ │ ┌──────────▼───┐ ┌─▼──────────────┐ ┌─▼────────▼─────┐ ┌────────▼────────┐ ┌───▼──┐ │ │
│ │ │  ParserYaml  │ │   ParserToml   │ │   ParserJson   │ │    ParserEnv    │ │Custom│ │ │
│ │ └──────────┬───┘ └─┬──────────────┘ └─┬────────┬─────┘ └────────┬────────┘ └───┬──┘ │ │
│ │            │       │                  │        │                │              │    │ │
│ └────────────┼───────┼──────────────────┼────────┼────────────────┼──────────────┼────┘ │
│              │       │                  │        │                │              │      │
│              │       │                  │        │                │              │      │
│ ┌────────────▼───────▼──────────────────▼────────▼────────────────▼──────────────▼────┐ │
│ │                                       Merge                                         │ │
│ └─────────────────────────────────────────┬───────────────────────────────────────────┘ │
│                                           │                                             │
│                                           │                                             │
│ ┌─────────────────────────────────────────▼───────────────────────────────────────────┐ │
│ │                                      Validate                                       │ │
│ └─────────────────────────────────────────┬───────────────────────────────────────────┘ │
│                                           │                                             │
└───────────────────────────────────────────┼─────────────────────────────────────────────┘
                                            │
                                            │
                                            ▼
                                   Vec<(Name, Config)>
```

## Basic usage
In this example we're going to load our plugins' configurations from a directory and environment-variables.  
Here we have four configuration files for four plugins `foo`, `bar`, `baz`, and `qux` inside our example `tests/etc` directory:
```shell
$ tree tests/etc
```
```text
tests/etc
├── bar.json
├── baz.toml
├── foo.env
└── qux.yml
```

#### tests/etc/bar.json
```json
{
  "sqlite": {
    "recreate": true
  }
}
```

#### tests/etc/baz.toml
```toml
[logging]
format = "json"
```

#### tests/etc/foo.env
```dotenv
SERVER__PORT="8080" # listen port
```

#### tests/etc/qux.yml
```yaml
https:
  follow_redirects: false
```

Additionally, we set the following environment-variables:
```shell
export APP_NAME__FOO__SERVER__ADDRESS="127.0.0.1"
export APP_NAME__BAR__SQLITE__FILE="/path/to/app.db"
export APP_NAME__BAZ__LOGGING__LEVEL="debug"
export APP_NAME__QUX__HTTPS__INSECURE="false"
```

### Example main.rs
```rust
use plugx_config::{
    ext::anyhow::{Context, Result},
    Configuration, Url,
};

fn main() -> Result<()> {
    let url_list: Vec<Url> = get_url_list_from_cmd_args()?;

    let mut configuration = Configuration::new();
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

fn get_url_list_from_cmd_args() -> Result<Vec<Url>> {
    std::env::args()
        .skip(1)
        .try_fold(Vec::new(), |mut list, arg| {
            list.push(
                Url::parse(&arg).with_context(|| format!("Could not parse URL `{arg}`"))?,
            );
            Ok(list)
        })
}
```

### Output
```shell
$ /path/to/main 'env://?prefix=APP_NAME' 'fs:///tests/etc/?strip-slash=true'
```
```text
bar: {"sqlite": {"recreate": true, "file": "/path/to/app.db"}}
foo: {"server": {"address": "127.0.0.1", "port": 8080}}
baz: {"logging": {"level": "debug", "format": "json"}}
qux: {"https": {"follow_redirects": false, "insecure": false}}
```
