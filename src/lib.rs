use {
	anyhow::{anyhow, Error},
	ddc_hi::{traits::*, Display, Query},
	log::{debug, error, warn},
	std::{
		iter,
		sync::{
			atomic::{AtomicUsize, Ordering},
			Arc, Mutex,
		},
	},
};

#[derive(Debug, Default, Clone)]
pub struct Config {
	pub query: Query,
	pub needs_caps: bool,
	pub needs_edid: bool,
	pub request_caps: bool,
}

pub trait CliCommand {
	fn run(&mut self, sleep: &mut DisplaySleep, args: &Config) -> Result<i32, Error>;
}

pub trait DisplayCommand {
	const NAME: &'static str;

	fn process(&mut self, args: &Config, display: &mut Display) -> Result<(), Error>;
}

impl<T: DisplayCommand> CliCommand for T {
	fn run(&mut self, sleep: &mut DisplaySleep, args: &Config) -> Result<i32, Error> {
		let mut errors = Vec::new();
		for display in query_displays(args) {
			let mut display = display?;

			match self.process(args, &mut display) {
				Ok(()) => (),
				Err(e) => {
					error!(
						target: &format!("ddcset::{}", Self::NAME),
						"failed to process display: {e:?}"
					);
					errors.push(e);
				},
			}

			sleep.add(display);
		}

		match errors.into_iter().next() {
			Some(e) => Err(e),
			None => Ok(0),
		}
	}
}

#[derive(Default)]
pub struct DisplaySleep(Vec<Display>);

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

pub fn query_displays<'a>(args: &'a Config) -> impl Iterator<Item = Result<Display, Error>> + 'a {
	let Config {
		ref query, needs_caps, ..
	} = *args;
	let errors = Arc::new(Mutex::new(Vec::new()));
	let display_count = Arc::new(AtomicUsize::new(0));

	Display::enumerate_all()
		.into_iter()
		.filter_map({
			let errors = errors.clone();
			move |d| match d {
				Ok(d) => Some(d),
				Err(e) => {
					warn!(
						target: &format!("ddc_hi::enumerate::{}", e.backend().name()),
						"Failed to enumerate: {}", e
					);
					errors.lock().unwrap().push(e);
					None
				},
			}
		})
		.filter_map(move |mut d| {
			if !needs_caps && query.matches(&d.info()) {
				return Some(d)
			}

			if let Err(e) = d.update_fast(needs_caps) {
				warn!(
					target: &format!("ddc_hi::enumerate::{}", d.backend().name()),
					"Failed to query {}: {}", d.id, e
				);
			}

			match query.matches(&d.info()) {
				true => Some(d),
				false => None,
			}
		})
		.map({
			let display_count = display_count.clone();
			move |v| {
				display_count.fetch_add(1, Ordering::AcqRel);

				Ok(v)
			}
		})
		.chain(iter::from_fn(move || match display_count.load(Ordering::Acquire) {
			0 => Some(Err(match errors.lock().unwrap().drain(..).next() {
				Some(e) => e.into(),
				None => anyhow!("no matching displays found"),
			})),
			_ => None,
		}))
}
