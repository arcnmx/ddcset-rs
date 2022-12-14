use {
	crate::{displays, util, DisplaySleep, GlobalArgs},
	anyhow::{anyhow, Error},
	clap::Args,
	ddc_hi::{traits::*, FeatureCode, Query},
	log::error,
};

/// Set VCP feature value
#[derive(Args, Debug)]
pub struct SetVCP {
	/// Feature code
	#[arg(value_parser(util::parse_feature))]
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
		value_parser(util::parse_hex_string)
	)]
	pub table: Option<util::HexString>,
	/// Table write offset
	#[arg(short, long)]
	pub offset: Option<u16>,
}

impl SetVCP {
	pub fn run(self, sleep: &mut DisplaySleep, _args: GlobalArgs, query: (Query, bool)) -> Result<i32, Error> {
		let SetVCP {
			value,
			table,
			feature_code,
			offset,
			verify,
		} = self;
		let value = match (value, table) {
			(Some(value), None) => Ok(value),
			(None, Some(table)) => Err(table),
			_ => unreachable!(),
		};

		let mut exit_code = 0;
		for display in displays(query) {
			let mut display = display?;
			println!("Display on {}:", display.backend());
			println!("\tID: {}", display.id);

			if let Err(e) = (|| -> Result<(), Error> {
				match value {
					Ok(value) => display.handle.set_vcp_feature(feature_code, value),
					Err(ref table) => display
						.handle
						.table_write(feature_code, offset.unwrap_or_default(), &table),
				}?;

				if verify {
					let matches = match value {
						Ok(value) => display.handle.get_vcp_feature(feature_code)?.value() == value,
						Err(ref table) => &display.handle.table_read(feature_code)? == table,
					};

					if !matches {
						return Err(anyhow!("Verification failed"))
					}
				}

				Ok(())
			})() {
				error!("Failed to set feature: {}", e);
				exit_code = 1;
			}

			sleep.add(display);
		}

		Ok(exit_code)
	}
}
