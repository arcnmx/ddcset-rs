use {
	crate::{displays, DisplaySleep, GlobalArgs},
	anyhow::Error,
	clap::Args,
	ddc_hi::Query,
	log::warn,
};

/// List detected displays
#[derive(Args, Debug)]
pub struct Detect {}

impl Detect {
	pub fn run(self, sleep: &mut DisplaySleep, args: GlobalArgs, query: (Query, bool)) -> Result<i32, Error> {
		for display in displays(query) {
			let mut display = display?;
			{
				println!("Display on {}:", display.backend());
				println!("\tID: {}", display.id);

				if let Err(e) = display.update_fast(false) {
					warn!("failed to retrieve display info: {:?}", e);
				}

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
	}
}
