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

## Demo
#### Preparation of the demo
In this example we're going to load our plugins' configurations from a directory and environment-variables.  
Here we have four configuration files for four plugins `foo`, `bar`, `baz`, and `qux`. This is our example `etc` directory:
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

