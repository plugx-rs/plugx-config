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

