use {
	anyhow::Error,
	clap::Args,
	ddc_hi::Display,
	ddcset::{Config, DisplayCommand},
	mccs_db::ValueType,
};

/// Query display capabilities
#[derive(Args, Debug)]
pub struct Capabilities {}

impl DisplayCommand for Capabilities {
	const NAME: &'static str = "capabilities";

	fn process(&mut self, _args: &Config, display: &mut Display) -> Result<(), Error> {
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
	}
}
