use {
	anyhow::Error,
	clap::Args,
	ddc_hi::{traits::*, Display},
	ddcset::{Config, DisplayCommand},
};

/// Request monitor to save settings
#[derive(Args, Debug)]
pub struct SaveCurrentSettings {}

impl DisplayCommand for SaveCurrentSettings {
	const NAME: &'static str = "save-current-settings";

	fn process(&mut self, _args: &Config, display: &mut Display) -> Result<(), Error> {
		display.handle.save_current_settings()?;

		Ok(())
	}
}
