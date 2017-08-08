use trimmer::{Template, Options, ParseError};
use rustc_serialize::{Decoder, Decodable};
use quire::validate::{Structure, Scalar};

use template;

lazy_static! {
    static ref OPTIONS: Options = Options::new()
        .syntax_oneline()
        .clone();
}


#[derive(Debug)]
pub struct Format {
    pub template_source: String,
    pub template: Template,
}

impl Decodable for Format {
    fn decode<D: Decoder>(d: &mut D) -> Result<Self, D::Error> {
        #[derive(RustcDecodable)]
        struct FormatRaw {
            template: String,
        }
        let raw = FormatRaw::decode(d)?;
        Format::from_string(raw.template)
            .map_err(|e| d.error(&format!("{}", e)))
    }
}

impl PartialEq for Format {
    fn eq(&self, other: &Format) -> bool {
        self.template_source == other.template_source
    }
}

impl Eq for Format { }

pub fn format_validator<'x>() -> Structure<'x> {
    Structure::new()
    .member("template", Scalar::new())
}

impl Format {
    pub fn from_string(template: String) -> Result<Format, ParseError> {
        Ok(Format {
            template: template::PARSER
                .parse_with_options(&*OPTIONS, &template)?,
            template_source: template,
        })
    }
}
