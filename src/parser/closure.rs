use crate::parser::{BoxedModifierFn, ConfigurationParser, MODIFIER_FN_DEBUG};
use plugx_input::Input;
use std::fmt::{Debug, Display, Formatter};

pub type BoxedParserFn = Box<dyn Fn(&[u8]) -> anyhow::Result<Input> + Send + Sync>;
pub type BoxedValidatorFn = Box<dyn Fn(&[u8]) -> Option<bool> + Send + Sync>;

pub struct ConfigurationParserFn {
    parser: BoxedParserFn,
    validator: BoxedValidatorFn,
    supported_format_list: Vec<String>,
    maybe_modifier: Option<BoxedModifierFn>,
}

impl Display for ConfigurationParserFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(
            self.supported_format_list
                .iter()
                .last()
                .map_or("unknown", |format| format.as_str()),
        )
    }
}

impl Debug for ConfigurationParserFn {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ConfigurationParserFn")
            .field(
                "parser",
                &stringify!(Box<dyn Fn(&[u8]) -> anyhow::Result<Input> + Send + Sync>),
            )
            .field(
                "validator",
                &stringify!(Box<dyn Fn(&[u8]) -> Option<bool> + Send + Sync>),
            )
            .field("supported_format_list", &self.supported_format_list)
            .field("maybe_modifier", &MODIFIER_FN_DEBUG)
            .finish()
    }
}

impl ConfigurationParserFn {
    pub fn new<F: AsRef<str>>(supported_format: F, parser: BoxedParserFn) -> Self {
        Self {
            parser,
            validator: Box::new(|_| None),
            supported_format_list: [supported_format.as_ref().to_string()].to_vec(),
            maybe_modifier: None,
        }
    }

    pub fn set_parser(&mut self, parser: BoxedParserFn) {
        self.parser = parser;
    }

    pub fn with_parser(mut self, parser: BoxedParserFn) -> Self {
        self.set_parser(parser);
        self
    }

    pub fn set_validator(&mut self, validator: BoxedValidatorFn) {
        self.validator = validator;
    }

    pub fn with_validator(mut self, validator: BoxedValidatorFn) -> Self {
        self.set_validator(validator);
        self
    }

    pub fn set_format_list<N: AsRef<str>>(&mut self, format_list: &[N]) {
        self.supported_format_list = format_list
            .iter()
            .map(|format| format.as_ref().to_string())
            .collect();
    }

    pub fn with_format_list<N: AsRef<str>>(mut self, format_list: &[N]) -> Self {
        self.set_format_list(format_list);
        self
    }

    pub fn set_modifier(&mut self, modifier: BoxedModifierFn) {
        self.maybe_modifier = Some(modifier);
    }

    pub fn with_modifier(mut self, modifier: BoxedModifierFn) -> Self {
        self.set_modifier(modifier);
        self
    }
}

impl ConfigurationParser for ConfigurationParserFn {
    fn maybe_get_modifier(&self) -> Option<&BoxedModifierFn> {
        self.maybe_modifier.as_ref()
    }

    fn supported_format_list(&self) -> Vec<String> {
        self.supported_format_list.clone()
    }

    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Input> {
        (self.parser)(bytes)
    }

    fn is_format_supported(&self, bytes: &[u8]) -> Option<bool> {
        (self.validator)(bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::enable_logging;
    use std::collections::HashMap;

    #[test]
    fn xml() {
        enable_logging();

        // let mut xml_format = InputFormatFn::new()
        //     .unwrap()
        //     .with_serializer(Box::new(|input: &Input| {
        //         serde_xml_rs::to_string(input)
        //             .map(|string| string.into_bytes())
        //             .map_err(|error| anyhow!(error))
        //     }))
        //     .with_deserializer(Box::new(|bytes: &[u8]| {
        //         let string = String::from_utf8(bytes.to_vec()).map_err(|error| anyhow!(error))?;
        //         serde_xml_rs::from_str(string.as_str()).map_err(|error| anyhow!(error))
        //     }))
        //     .with_name_list(["xml"].to_vec());
        // let mut input: Input = HashMap::<String, Input>::new().into();
        // let mut copy = input.clone();
        // // <foo/>
        // input
        //     .map_mut()
        //     .unwrap()
        //     .insert("foo".to_string(), Input::from("p p p "));
        // // copy = input.clone();
        // // // <foo><bar><foo/></bar></foo>
        // // input
        // //     .map_mut()
        // //     .unwrap()
        // //     .get_mut("foo")
        // //     .unwrap()
        // //     .map_mut()
        // //     .unwrap()
        // //     .insert("bar".to_string(), copy.clone());
        // // copy = input.clone();
        // // // <foo><bar><baz>value</baz><foo/></bar></foo>
        // // input
        // //     .map_mut()
        // //     .unwrap()
        // //     .get_mut("foo")
        // //     .unwrap()
        // //     .map_mut()
        // //     .unwrap()
        // //     .get_mut("bar")
        // //     .unwrap()
        // //     .map_mut()
        // //     .unwrap()
        // //     .insert("baz".to_string(), Input::from("p p p "));
        // let xml_text = String::from_utf8(xml_format.serialize(&input).unwrap()).unwrap();
        // println!("{xml_text:#?}");
        // let new_input = xml_format
        //     .deserialize("<?xml version=\"1.0\" encoding=\"utf-8\"?><foo>salam</foo>".as_bytes())
        //     .unwrap();
        // println!("{new_input:#?}");
        // assert_eq!(input, new_input);
    }
}
