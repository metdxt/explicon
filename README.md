# Explicon

Explicon is a tiny utility crate for removing surprises from configuration.

Have you ever been annoyed that when you set some value in your `config.toml`
if silently got overriden by some environment variable you weren't even aware of?

This crate addresses exactly that problem, by making configuration EXPLICIT.

## Example

```rust
// Hypothetical other config crate (pseudo-code)

#[derive(Debug, FancyConfig, serde::Deserialize, serde::Serialize)]
struct MyAwesomeConfig {
    host: String,
    port: u16
}

let config = MyAwesomeConfig::from_file("config.toml")
    .from_env() // Silently overrides values in config.toml, BAD!
```

In contrast:

```rust
// Explicon
use explicon::Sourced;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct MyAwesomeConfig {
    host: Sourced<String>
    port: Sourced<u16>
}


let config: MyAwesomeConfig = toml::from_str(r#"
    host = { env = "MY_ENV_VAR_FOR_HOST" }
    port = 8080
"#).unwrap();

let host: String = config.host.resolve().unwrap() // Get actual value from config
let port: u16 = config.port.resolve().unwrap_or(3000)
```

As you can see, using `explicon::Sourced` in your configuration
allows you to see exactly where values come from only from your 
`config.toml` file, no need to parse actual source of your program
to figure out that your host is actually overriden by some `MY_COOL_APP_HOST`.

***No hidden flow.***
