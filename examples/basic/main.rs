extern crate plugx_config;

use plugx_config::{ext::anyhow::Result, Configuration, Url};

fn main() -> Result<()> {
    let mut configuration = Configuration::new();
    // Parse URL(s) from commandline arguments:
    std::env::args()
        .skip(1)
        .try_for_each(|arg| configuration.add_url(Url::parse(&arg).map_err(Into::into)?))?;
    // Load & Parse & Merge & print:
    configuration
        .load_parse_merge(true)?
        .iter()
        .for_each(|(plugin_name, configuration)| println!("{plugin_name}: {configuration}"));
    Ok(())
}
