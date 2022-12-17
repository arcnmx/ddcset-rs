use {
	crate::{capabilities::Capabilities, detect::Detect, getvcp::GetVCP, save::SaveCurrentSettings, setvcp::SetVCP},
	anyhow::{anyhow, Error},
	clap::{Args, Parser, Subcommand},
	ddc_hi::{traits::*, Backend, Display, Query},
	log::{debug, warn},
	std::{
		io::{self, Write},
		iter,
		process::exit,
		sync::{
			atomic::{AtomicUsize, Ordering},
			Arc, Mutex,
		},
	},
};

mod capabilities;
mod detect;
mod getvcp;
mod save;
mod setvcp;
mod util;

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
	#[arg(short, long, number_of_values(1), value_parser(util::backend_parser()))]
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
pub struct GlobalArgs {
	/// Read display capabilities
	#[arg(short, long)]
	pub capabilities: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
	Detect(Detect),
	Capabilities(Capabilities),
	GetVCP(GetVCP),
	SetVCP(SetVCP),
	SaveCurrentSettings(SaveCurrentSettings),
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

fn main() {
	match main_result() {
		Ok(code) => exit(code),
		Err(e) => {
			let _ = writeln!(io::stderr(), "{}", e);
			exit(1);
		},
	}
}

pub(crate) fn displays((query, needs_caps): (Query, bool)) -> impl Iterator<Item = Result<Display, Error>> {
	let errors = Arc::new(Mutex::new(Vec::new()));
	let display_count = Arc::new(AtomicUsize::new(0));

	Display::enumerate_all()
		.into_iter()
		.filter_map({
			let errors = errors.clone();
			move |d| match d {
				Ok(d) => Some(d),
				Err(e) => {
					warn!("Failed to enumerate {}: {}", e.backend(), e);
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
				warn!("Failed to query {}/{}: {}", d.backend(), d.id, e);
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
		Command::Detect(cmd) => cmd.run(&mut sleep, args, query),
		Command::Capabilities(cmd) => cmd.run(&mut sleep, args, query),
		Command::GetVCP(cmd) => cmd.run(&mut sleep, args, query),
		Command::SetVCP(cmd) => cmd.run(&mut sleep, args, query),
		Command::SaveCurrentSettings(cmd) => cmd.run(&mut sleep, args, query),
	}
}
