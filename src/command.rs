// use crate::{Configuration, ConfigurationError};
// use clap::{Parser, Subcommand};
// use plugx_input::Input;
// use serde::{Deserialize, Serialize};
// use std::collections::HashMap;
// use url::Url;
//
// #[derive(Debug, Clone, Serialize, Deserialize)]
// #[cfg_attr(feature = "clap", derive(Subcommand))]
// pub enum ConfigurationCommand {
//     AddUrl {
//         url: Url,
//     },
//     RemoveUrl {
//         url: Url,
//     },
//     HasUrl {
//         url: Url,
//     },
//     AddPluginToWhitelist {
//         plugin_name: String,
//     },
//     GetConfiguration,
//     SetConfiguration {
//         configuration: HashMap<String, Input>,
//     },
// }
//
// impl ConfigurationCommand {
//     pub fn apply_to(
//         &self,
//         configuration: &mut Configuration,
//     ) -> Result<Option<Input>, ConfigurationError> {
//         match self {
//             Self::AddUrl { url } => {
//                 configuration.add_url(url.clone());
//                 Ok(None)
//             }
//             Self::RemoveUrl { url } => Ok(Some(
//                 configuration.remove_url_and_loader(url.clone()).into(),
//             )),
//             Self::HasUrl { url } => Ok(Some(configuration.has_url(url.clone()).into())),
//             Self::AddPluginToWhitelist { plugin_name } => {
//                 configuration.add_to_whitelist(plugin_name);
//                 Ok(None)
//             }
//             Self::GetConfiguration => Ok(Some(configuration.configuration().clone().into())),
//             Self::SetConfiguration {
//                 configuration: new_configuration,
//             } => {
//                 *configuration.configuration_mut() = new_configuration.clone();
//                 Ok(None)
//             }
//         }
//     }
// }
