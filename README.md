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
```rust


```
#### Preparation of the demo
In this example we're going to load our plugins' configurations from a directory and environment-variables.  
Here we have four configuration files for four plugins `foo`, `bar`, `baz`, and `qux` inside our example `etc` directory:
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
<br/>

```shell
$ cat tests/etc/bar.json
```
```json
{
  "sqlite": {
    "recreate": true,
    "file": "/path/to/app.db"
  }
}
```
<br/>

```shell
$ cat tests/etc/baz.toml
```
```toml
[logging]
level = "debug"
output_serialize_format = "json"
```
<br/>

```shell
$ cat tests/etc/foo.env
```
```dotenv
SERVER__PORT="8080" # listen port
```
<br/>

```shell
$ cat tests/etc/qux.yml
```
```yaml
https:
  follow_redirects: false
  insecure: false
```
<br/>

Additionally, we set the following environment-variables:
```shell
$ export APP_NAME__FOO__SERVER__ADDRESS="127.0.0.1"
$ export APP_NAME__BAR__SQLITE__FILE="/path/to/app.db"
$ export APP_NAME__BAZ__LOGGING__LEVEL="debug"
$ export APP_NAME__QUX__HTTPS__INSECURE="false"
```
<br/>

Usage:
```rust
use plugx_config::{ext::url::Url, Configuration};
use plugx_input::schema::InputSchemaType;
use std::{collections::HashMap, env, fs};

// Add our URLs.
// Generally you need to get them from commandline arguments or somewhere else:
let env_url: Url = "env://?prefix=APP_NAME".parse().expect("Valid URL");
let current_directory = env::current_dir().expect("CWD");
let directory_url: Url = format!("file://{}/tests/etc/", current_directory.to_str().unwrap())
    .parse()
    .expect("Valid URL");

// Initialize plugins' configurations:
let mut configuration = Configuration::new().with_url(env_url)?.with_url(directory_url)?;


```