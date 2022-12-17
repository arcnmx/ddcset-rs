use {
	crate::{capabilities::Capabilities, detect::Detect, getvcp::GetVCP, save::SaveCurrentSettings, setvcp::SetVCP},
	anyhow::Error,
	clap::{Args, Parser, Subcommand},
	ddc_hi::{Backend, Query},
	ddcset::{CliCommand, Config, DisplaySleep},
	std::{
		io::{self, Write},
		process::exit,
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
	#[command(alias = "caps")]
	Capabilities(Capabilities),
	#[command(alias = "getvcp", alias = "get")]
	GetVCP(GetVCP),
	#[command(alias = "setvcp", alias = "set")]
	SetVCP(SetVCP),
	#[command(alias = "save")]
	SaveCurrentSettings(SaveCurrentSettings),
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

	let config = Config {
		query: filter.query(),
		needs_caps: filter.needs_caps(),
		needs_edid: filter.needs_edid(),
		request_caps: args.capabilities,
	};

	let mut sleep = DisplaySleep::default();

	match command {
		Command::Detect(mut cmd) => cmd.run(&mut sleep, &config),
		Command::Capabilities(mut cmd) => cmd.run(&mut sleep, &config),
		Command::GetVCP(mut cmd) => cmd.run(&mut sleep, &config),
		Command::SetVCP(mut cmd) => cmd.run(&mut sleep, &config),
		Command::SaveCurrentSettings(mut cmd) => cmd.run(&mut sleep, &config),
	}
}
