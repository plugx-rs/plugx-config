use plugx_config::{
    ext::anyhow::{Context, Result},
    Configuration, Url,
};

fn main() -> Result<()> {
    let (trace, url_list) = get_options_from_cmd_args()?;

    tracing_subscriber::fmt()
        .pretty()
        .with_max_level(if trace {
            tracing::Level::TRACE
        } else {
            tracing::Level::INFO
        })
        .with_line_number(false)
        .with_file(false)
        .without_time()
        .init();

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

fn get_options_from_cmd_args() -> Result<(bool, Vec<Url>)> {
    std::env::args()
        .skip(1)
        .try_fold((false, Vec::new()), |(mut trace, mut list), arg| {
            if arg == "--trace" {
                trace = true;
            } else {
                list.push(
                    Url::parse(&arg).with_context(|| format!("Could not parse URL `{arg}`"))?,
                );
            }
            Ok((trace, list))
        })
}
