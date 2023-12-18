# Plugin configuration manager (work-in-progress)
[**Package**](https://crates.io/crates/plugx-config)   |   [**Documentation**](https://docs.rs/plugx-config)   |   [**Repository**](https://github.com/plugx-rs/plugx-config)

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

#### Demo code
```rust
use plugx_config::{
    Configuration,
    ext::{
        url::Url,
        plugx_input::schema::InputSchemaType,
    }
};
use std::{env, fs, collections::HashMap};

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
let file_url: Url = format!("file://{current_dir}?skippable[0]=notfound") // Skips error if `current_dir` does not exists
    .parse()
    .expect("Valid URL");

let mut configuration = Configuration::default().with_url(env_url).with_url(file_url);
let apply_skippable_errors = true;
configuration.try_load_parse_merge(apply_skippable_errors).unwrap();
// Print all configurations:
configuration
    .configuration()
    .iter()
    .for_each(|(plugin, config)| println!("{plugin}: {config}"));
// Prints:
//  foo: {"server": {"port": 8080}}
//  baz: {"logging": {"output_serialize_format": "json", "level": "debug"}}
//  bar: {"sqlite": {"file": "/path/to/app.db", "recreate": true}}
//  qux: {"https": {"insecure": false, "follow_redirects": false}}

// Also we can validate our plugins' configurations.
// Here we just check foo's validation:
let rules_yml = r#"
foo:
  type: static_map
  items:
    server:
      schema:
        type: static_map
        items:         
          address:
            schema:
              type: ip
            default: 127.0.0.1
          port:
            schema:
              type: integer
              range:
                min: 1
                max: 65535
"#;
let rules: HashMap<String, InputSchemaType> = serde_yaml::from_str(rules_yml).unwrap();
configuration
    .try_load_parse_merge_validate(apply_skippable_errors, &rules) // Validates configurations too
    .unwrap();
// Set invalid IP address to test validation:
env::set_var("APP_NAME__FOO__SERVER__ADDRESS", "127.0.0.1.bad.ip");
let error = configuration
    .try_load_parse_merge_validate(apply_skippable_errors, &rules)
    .err()
    .unwrap();
println!("{error}");
// Prints:
//  [foo][server][address] Could not parse IP address: invalid IP address syntax (expected IP address and got "127.0.0.1.bad.ip")
```
