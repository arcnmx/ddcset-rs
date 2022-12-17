use {
	anyhow::{anyhow, Error},
	clap::Args,
	ddc_hi::{traits::*, Display, FeatureCode},
	ddcset::{Config, DisplayCommand},
	log::{as_debug, error},
	mccs_db::{Access, Descriptor, TableInterpretation, ValueInterpretation, ValueType},
};

/// Get VCP feature value
#[derive(Args, Debug)]
pub struct GetVCP {
	/// Feature code
	#[arg(value_parser(crate::util::parse_feature))]
	pub feature_code: Vec<FeatureCode>,
	/// Show raw value
	#[arg(short, long)]
	pub raw: bool,
	/// Read as table value
	#[arg(short, long)]
	pub table: bool,
	/// Scan all VCP feature codes
	#[arg(short, long)]
	pub scan: bool,
}

impl GetVCP {
	fn get_code(&mut self, display: &mut Display, code: FeatureCode, feature: Option<&Descriptor>) -> Result<(), Error> {
		let handle = &mut display.handle;
		if let Some(feature) = feature {
			if feature.access == Access::WriteOnly {
				println!("\tFeature 0x{:02x} is write-only", code);
				return Ok(())
			}

			match feature.ty {
				ValueType::Unknown => {
					let value = handle.get_vcp_feature(code)?;
					println!(
						"\tFeature 0x{:02x} = {}",
						code,
						ValueInterpretation::Continuous.format(&value)
					);
				},
				ValueType::Continuous { mut interpretation } => {
					let value = handle.get_vcp_feature(code)?;
					if self.raw {
						interpretation = ValueInterpretation::Continuous;
					}
					println!("\tFeature 0x{:02x} = {}", feature.code, interpretation.format(&value))
				},
				ValueType::NonContinuous {
					ref values,
					mut interpretation,
				} => {
					if self.raw {
						interpretation = ValueInterpretation::Continuous;
					}

					let value = handle.get_vcp_feature(code)?;
					if let Some(&Some(ref name)) = values.get(&(value.value() as u8)) {
						println!(
							"\tFeature 0x{:02x} = {}: {}",
							feature.code,
							interpretation.format(&value),
							name
						)
					} else {
						println!("\tFeature 0x{:02x} = {}", feature.code, interpretation.format(&value))
					}
				},
				ValueType::Table { mut interpretation } => {
					if self.raw {
						interpretation = TableInterpretation::Generic;
					}

					let value = handle.table_read(code)?;
					println!(
						"\tFeature 0x{:02x} = {}",
						code,
						interpretation
							.format(&value)
							.map_err(|_| anyhow!("table interpretation failed"))?
					);
				},
			}
		} else {
			if self.table {
				let value = handle.table_read(code)?;
				println!(
					"\tFeature 0x{:02x} = {}",
					code,
					TableInterpretation::Generic.format(&value).unwrap()
				);
			} else {
				let value = handle.get_vcp_feature(code)?;
				println!(
					"\tFeature 0x{:02x} = {}",
					code,
					ValueInterpretation::Continuous.format(&value)
				);
			};
		}

		Ok(())
	}
}

impl DisplayCommand for GetVCP {
	const NAME: &'static str = "get-vcp";

	fn process(&mut self, args: &Config, display: &mut Display) -> Result<(), Error> {
		println!("Display on {}:", display.backend());
		println!("\tID: {}", display.id);
		let mut mccs_database = display.mccs_database().unwrap_or_default();
		let codes = if !self.feature_code.is_empty() {
			let _ = display.update_fast(args.request_caps);
			self.feature_code.clone()
		} else {
			if !self.scan {
				display.update_capabilities()?;
				(0..0x100)
					.map(|v| v as FeatureCode)
					.filter(|&c| mccs_database.get(c).is_some())
					.collect()
			} else {
				(0..0x100).map(|v| v as FeatureCode).collect()
			}
		};
		if let Some(db) = display.mccs_database() {
			mccs_database = db;
		}

		let mut errors = Vec::new();
		for code in codes {
			let feature = mccs_database.get(code);
			if let Err(e) = self.get_code(display, code, feature) {
				error!(
					target: "ddcset::get-vcp",
					command = "get-vcp",
					operation = "get_vcp_feature",
					feature_code = code,
					feature = as_debug!(feature),
					error = as_debug!(e),
					display = display;
					"Failed to get feature 0x{code:02x} for {display}: {e}"
				);
				errors.push(e);
			}
		}

		errors.into_iter().next().map(Err).unwrap_or(Ok(()))
	}
}
