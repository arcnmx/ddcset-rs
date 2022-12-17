use {
	anyhow::{anyhow, Error},
	clap::Args,
	ddc_hi::{traits::*, Display, FeatureCode},
	ddcset::{Config, DisplayCommand},
};

/// Set VCP feature value
#[derive(Args, Debug)]
pub struct SetVCP {
	/// Feature code
	#[arg(value_parser(crate::util::parse_feature))]
	pub feature_code: FeatureCode,
	/// Value to set
	#[arg(required_unless_present = "table")]
	pub value: Option<u16>,
	/// Read value after writing
	#[arg(short, long)]
	pub verify: bool,
	/// Write a table value
	#[arg(
		short,
		long = "table",
		conflicts_with = "value",
		value_parser(crate::util::parse_hex_string)
	)]
	pub table: Option<crate::util::HexString>,
	/// Table write offset
	#[arg(short, long)]
	pub offset: Option<u16>,
}

impl DisplayCommand for SetVCP {
	const NAME: &'static str = "set-vcp";

	fn process(&mut self, _args: &Config, display: &mut Display) -> Result<(), Error> {
		let value = match (self.value, &self.table) {
			(Some(value), None) => Ok(value),
			(None, Some(table)) => Err(table),
			_ => unreachable!(),
		};

		println!("Display on {}:", display.backend());
		println!("\tID: {}", display.id);

		match value {
			Ok(value) => display.handle.set_vcp_feature(self.feature_code, value),
			Err(ref table) => display
				.handle
				.table_write(self.feature_code, self.offset.unwrap_or_default(), &table),
		}?;

		if self.verify {
			let matches = match value {
				Ok(value) => display.handle.get_vcp_feature(self.feature_code)?.value() == value,
				Err(table) => &display.handle.table_read(self.feature_code)? == table,
			};

			if !matches {
				return Err(anyhow!("Verification failed"))
			}
		}

		Ok(())
	}
}
