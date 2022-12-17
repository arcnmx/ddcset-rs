use {
	crate::{displays, DisplaySleep, GlobalArgs},
	anyhow::Error,
	clap::Args,
	ddc_hi::{traits::*, Query},
	log::error,
};

/// Request monitor to save settings
#[derive(Args, Debug)]
pub struct SaveCurrentSettings {}

impl SaveCurrentSettings {
	pub fn run(self, sleep: &mut DisplaySleep, _args: GlobalArgs, query: (Query, bool)) -> Result<i32, Error> {
		let mut exit_code = 0;
		for display in displays(query) {
			let mut display = display?;
			if let Err(e) = (|| -> Result<(), Error> {
				display.handle.save_current_settings()?;

				Ok(())
			})() {
				error!("Failed to save settings: {}", e);
				exit_code = 1;
			}

			sleep.add(display);
		}

		Ok(exit_code)
	}
}
