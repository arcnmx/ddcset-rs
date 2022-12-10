use {
	anyhow::{format_err, Error},
	clap::{builder::TypedValueParser, Args, Parser, Subcommand},
	ddc_hi::{traits::*, Backend, Display, FeatureCode, Query},
	log::{debug, error, warn},
	mccs_db::{Access, TableInterpretation, ValueInterpretation, ValueType},
	once_cell::sync::Lazy,
	std::{
		io::{self, Write},
		process::exit,
		str::FromStr,
	},
};

type HexString = Vec<u8>;
fn parse_hex_string(s: &str) -> Result<HexString, hex::FromHexError> {
	hex::decode(s)
}

fn parse_feature(s: &str) -> Result<FeatureCode, Error> {
	if s.starts_with("0x") {
		FeatureCode::from_str_radix(&s[2..], 16).map_err(Into::into)
	} else {
		FeatureCode::from_str(s).map_err(Into::into)
	}
}

fn backend_parser() -> impl TypedValueParser {
	clap::builder::PossibleValuesParser::from(Backend::values().iter().map(|b| b.name()))
		.try_map(|s| Backend::from_str(&s))
}

#[derive(Copy, Clone, Debug)]
struct BackendValue(Backend);

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

/// DDC/CI monitor control
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Cli {
	#[command(flatten)]
	args: GlobalArgs,
	#[command(flatten)]
	filter: Filter,
	#[command(subcommand)]
	command: Command,
}

#[derive(Args, Debug)]
struct Filter {
	/// Backend driver whitelist
	#[arg(short, long, number_of_values(1), value_parser(backend_parser()))]
	pub backend: Vec<Backend>,
	/// Filter by matching backend ID
	#[arg(short, long)]
	pub id: Option<String>,
	/// Filter by matching manufacturer ID
	#[arg(short = 'g', long = "mfg")]
	pub manufacturer: Option<String>,
	/// Filter by matching model
	#[arg(short = 'l', long = "model")]
	pub model_name: Option<String>,
	/// Filter by matching serial number
	#[arg(short = 'n', long = "sn")]
	pub serial: Option<String>,
	// TODO: filter by index? winapi makes things difficult, nothing is identifying...
}

impl Filter {
	fn query(&self) -> Query {
		let mut query = Query::Any;
		if !self.backend.is_empty() {
			let backends = self.backend.iter().copied().map(Query::Backend).collect();
			query = Query::And(vec![query, Query::Or(backends)])
		}
		if let Some(id) = &self.id {
			query = Query::And(vec![query, Query::Id(id.into())])
		}
		if let Some(manufacturer) = &self.manufacturer {
			query = Query::And(vec![query, Query::ManufacturerId(manufacturer.into())])
		}
		if let Some(model) = &self.model_name {
			query = Query::And(vec![query, Query::ModelName(model.into())]);
		}
		if let Some(serial) = &self.serial {
			query = Query::And(vec![query, Query::SerialNumber(serial.into())])
		}

		query
	}

	fn needs_caps(&self) -> bool {
		self.model_name.is_some()
	}

	fn needs_edid(&self) -> bool {
		self.manufacturer.is_some() || self.serial.is_some()
	}
}

#[derive(Args, Debug)]
struct GlobalArgs {
	/// Read display capabilities
	#[arg(short, long)]
	pub capabilities: bool,
}

/// List detected displays
#[derive(Args, Debug)]
struct Detect {}

/// Query display capabilities
#[derive(Args, Debug)]
struct Capabilities {}

/// Get VCP feature value
#[derive(Args, Debug)]
struct GetVCP {
	/// Feature code
	#[arg(value_parser(parse_feature))]
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

/// Set VCP feature value
#[derive(Args, Debug)]
struct SetVCP {
	/// Feature code
	#[arg(value_parser(parse_feature))]
	pub feature_code: FeatureCode,
	/// Value to set
	#[arg(required_unless_present = "table")]
	pub value: Option<u16>,
	/// Read value after writing
	#[arg(short, long)]
	pub verify: bool,
	/// Write a table value
	#[arg(short, long = "table", conflicts_with = "value", value_parser(parse_hex_string))]
	pub table: Option<HexString>,
	/// Table write offset
	#[arg(short, long)]
	pub offset: Option<u16>,
}

#[derive(Subcommand, Debug)]
enum Command {
	Detect(Detect),
	Capabilities(Capabilities),
	GetVCP(GetVCP),
	SetVCP(SetVCP),
}

#[derive(Default)]
struct DisplaySleep(Vec<Display>);

impl DisplaySleep {
	fn add(&mut self, display: Display) {
		self.0.push(display)
	}
}

impl Drop for DisplaySleep {
	fn drop(&mut self) {
		debug!("Waiting for display communication delays before exit");
		for display in self.0.iter_mut() {
			display.handle.sleep()
		}
	}
}

fn main() {
	match main_result() {
		Ok(code) => exit(code),
		Err(e) => {
			let _ = writeln!(io::stderr(), "{}", e);
			exit(1);
		},
	}
}

fn displays((query, needs_caps): (Query, bool)) -> Result<Vec<Display>, Error> {
	let mut errors = Vec::new();
	let displays: Vec<_> = Display::enumerate_all()
		.into_iter()
		.filter_map(|d| match d {
			Ok(d) => Some(d),
			Err(e) => {
				warn!("Failed to enumerate {}: {}", e.backend(), e);
				errors.push(e);
				None
			},
		})
		.map(|mut d| {
			if let Err(e) = d.update_fast(needs_caps) {
				warn!("Failed to query {}/{}: {}", d.backend(), d.id, e);
			}
			d
		})
		.filter(|d| match &query {
			Query::Any => true,
			query => query.matches(&d.info()),
		})
		.collect();

	match errors.into_iter().next() {
		Some(e) if displays.is_empty() => Err(e.into()),
		_ => Ok(displays),
	}
}

fn log_init() {
	use {
		env_logger::{Builder, Env},
		log::LevelFilter,
	};

	Builder::new()
		.filter_level(LevelFilter::Warn)
		.parse_env(Env::default())
		.init()
}

fn main_result() -> Result<i32, Error> {
	log_init();

	let Cli { args, command, filter } = Cli::parse();

	let query = (filter.query(), filter.needs_caps());
	// TODO: filter.needs_edid()

	let mut sleep = DisplaySleep::default();

	match command {
		Command::Detect(cmd) => {
			for mut display in displays(query)? {
				{
					println!("Display on {}:", display.backend());
					println!("\tID: {}", display.id);

					let info = display.info();
					let res = if args.capabilities {
						display.update_all()
					} else {
						Ok(drop(display.update_version()))
					};
					if let Err(e) = res {
						warn!("Failed to query display: {}", e);
					}

					if let Some(value) = info.manufacturer_id.as_ref() {
						println!("\tManufacturer ID: {}", value);
					}
					if let Some(value) = info.model_name.as_ref() {
						println!("\tModel: {}", value);
					}
					if let Some(value) = info.serial_number.as_ref() {
						println!("\tSerial: {}", value);
					}
					if let Some(value) = info.mccs_version.as_ref() {
						println!("\tMCCS: {}", value);
					} else {
						println!("\tMCCS: Unavailable");
					}
				}

				sleep.add(display);
			}

			Ok(0)
		},
		Command::Capabilities(cmd) => {
			let mut exit_code = 0;
			for mut display in displays(query)? {
				if let Err(e) = (|| -> Result<(), Error> {
					println!("Display on {}:", display.backend());
					println!("\tID: {}", display.id);
					display.update_fast(true)?;
					let mccs_database = display.mccs_database().unwrap_or_default();
					for feature in (0..0x100).filter_map(|v| mccs_database.get(v as _)) {
						println!(
							"\tFeature 0x{:02x}: {}",
							feature.code,
							feature.name.as_ref().map(|v| &v[..]).unwrap_or("Unknown")
						);
						println!("\t\tAccess: {:?}", feature.access);
						if feature.mandatory {
							println!("\t\tRequired");
						}
						if let Some(group) = feature.group.as_ref() {
							println!("\t\tGroup: {}", group);
						}
						if !feature.interacts_with.is_empty() {
							println!("\t\tInteracts:");
							for code in &feature.interacts_with {
								println!("\t\t\t{:02x}", code);
							}
						}
						match feature.ty {
							ValueType::Unknown => (),
							ValueType::Continuous { .. } => println!("\t\tType: Continuous"),
							ValueType::NonContinuous { .. } => println!("\t\tType: Non-Continuous"),
							ValueType::Table { .. } => println!("\t\tType: Table"),
						}
						if let Some(desc) = feature.description.as_ref() {
							println!("\t\t{}", desc);
						}
						match feature.ty {
							ValueType::NonContinuous { ref values, .. } =>
								for (value, name) in values {
									println!(
										"\t\t\t0x{:02x}: {}",
										value,
										name.as_ref().map(|v| &v[..]).unwrap_or("Unknown")
									);
								},
							_ => (),
						}
					}

					Ok(())
				})() {
					error!("Failed to get capabilities: {}", e);
					exit_code = 1;
				}

				sleep.add(display);
			}

			Ok(exit_code)
		},
		Command::GetVCP(GetVCP {
			feature_code,
			scan,
			raw,
			table,
		}) => {
			let mut exit_code = 0;
			for mut display in displays(query)? {
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
												.map_err(|_| format_err!("table interpretation failed"))?
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
		},
		Command::SetVCP(SetVCP {
			value,
			table,
			feature_code,
			offset,
			verify,
		}) => {
			let value = match (value, table) {
				(Some(value), None) => Ok(value),
				(None, Some(table)) => Err(table),
				_ => unreachable!(),
			};

			let mut exit_code = 0;
			for mut display in displays(query)? {
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
							return Err(format_err!("Verification failed"))
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
		},
	}
}
