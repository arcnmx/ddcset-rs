use {
	anyhow::Error,
	clap::Args,
	ddc_hi::Display,
	ddcset::{Config, DisplayCommand},
	log::{as_error, warn},
};

/// List detected displays
#[derive(Args, Debug)]
pub struct Detect {}

impl DisplayCommand for Detect {
	const NAME: &'static str = "detect";

	fn process(&mut self, args: &Config, display: &mut Display) -> Result<(), Error> {
		println!("Display on {}:", display.backend());
		println!("\tID: {}", display.id);

		if let Err(e) = display.update_fast(false) {
			warn!(
				command = "detect",
				operation = "update_fast",
				error = as_error!(e),
				display = display;
				"failed to retrieve {display} info: {e}"
			);
		}

		let info = display.info();
		let res = if args.request_caps {
			display.update_all()
		} else {
			Ok(drop(display.update_version()))
		};
		if let Err(e) = res {
			warn!(
				command = "detect",
				operation = "update_all",
				error = as_error!(e),
				display = display;
				"Failed to query {display}: {e}"
			);
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

		Ok(())
	}
}
