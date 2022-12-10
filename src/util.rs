use {
	anyhow::Error,
	clap::builder::TypedValueParser,
	ddc_hi::{Backend, FeatureCode},
	once_cell::sync::Lazy,
	std::str::FromStr,
};

pub type HexString = Vec<u8>;
pub fn parse_hex_string(s: &str) -> Result<HexString, hex::FromHexError> {
	hex::decode(s)
}

pub fn parse_feature(s: &str) -> Result<FeatureCode, Error> {
	if s.starts_with("0x") {
		FeatureCode::from_str_radix(&s[2..], 16).map_err(Into::into)
	} else {
		FeatureCode::from_str(s).map_err(Into::into)
	}
}

pub fn backend_parser() -> impl TypedValueParser {
	clap::builder::PossibleValuesParser::from(Backend::values().iter().map(|b| b.name()))
		.try_map(|s| Backend::from_str(&s))
}

#[derive(Copy, Clone, Debug)]
pub struct BackendValue(Backend);

impl clap::ValueEnum for BackendValue {
	fn value_variants<'a>() -> &'a [Self] {
		static VALID_BACKENDS: Lazy<Vec<BackendValue>> =
			Lazy::new(|| Backend::values().iter().cloned().map(BackendValue).collect());
		&VALID_BACKENDS
	}

	fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
		Some(self.0.name().into())
	}
}
