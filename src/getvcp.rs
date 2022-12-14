use {
	crate::{displays, util, DisplaySleep, GlobalArgs},
	anyhow::{anyhow, Error},
	clap::Args,
	ddc_hi::{traits::*, FeatureCode, Query},
	log::error,
	mccs_db::{Access, TableInterpretation, ValueInterpretation, ValueType},
};

/// Get VCP feature value
#[derive(Args, Debug)]
pub struct GetVCP {
	/// Feature code
	#[arg(value_parser(util::parse_feature))]
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
	pub fn run(self, sleep: &mut DisplaySleep, args: GlobalArgs, query: (Query, bool)) -> Result<i32, Error> {
		let mut exit_code = 0;
		let GetVCP {
			feature_code,
			scan,
			raw,
			table,
		} = self;
		for display in displays(query) {
			let mut display = display?;
			println!("Display on {}:", display.backend());
			println!("\tID: {}", display.id);
			if let Err(e) = (|| -> Result<(), Error> {
				let mut mccs_database = display.mccs_database().unwrap_or_default();
				let codes = if !feature_code.is_empty() {
					let _ = display.update_fast(args.capabilities);
					feature_code.clone()
				} else {
					if !scan {
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

				for code in codes {
					let feature = mccs_database.get(code);
					let handle = &mut display.handle;
					if let Err(e) = (|| -> Result<(), Error> {
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
									if raw {
										interpretation = ValueInterpretation::Continuous;
									}
									println!("\tFeature 0x{:02x} = {}", feature.code, interpretation.format(&value))
								},
								ValueType::NonContinuous {
									ref values,
									mut interpretation,
								} => {
									if raw {
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
									if raw {
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
							if table {
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
					})() {
						error!("Failed to get feature: {}", e);
						exit_code = 1;
					}
				}

				Ok(())
			})() {
				error!("Failed to get features: {}", e);
				exit_code = 1;
			}

			sleep.add(display);
		}

		Ok(exit_code)
	}
}
