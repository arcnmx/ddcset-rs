use std::io::{self, Write};
use std::str::FromStr;
use std::process::exit;
use clap::{Arg, App, SubCommand, AppSettings};
use anyhow::{Error, format_err};
use log::{info, error};
use ddc_hi::{Backend, Display, Query, FeatureCode, Ddc, DdcTable, DdcHost};
use mccs_db::{Access, ValueInterpretation, TableInterpretation, ValueType};

#[derive(Default)]
struct DisplaySleep(Vec<Display>);

impl DisplaySleep {
    fn add(&mut self, display: Display) {
        self.0.push(display)
    }
}

impl Drop for DisplaySleep {
    fn drop(&mut self) {
        info!("Waiting for display communication delays before exit");
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

fn displays(query: (Query, bool)) -> Result<Vec<Display>, Error> {
    let needs_caps = query.1;
    let query = query.0;
    Display::enumerate().into_iter()
        .map(|mut d| if needs_caps && d.info.backend == Backend::WinApi {
            d.update_capabilities().map(|_| d)
        } else {
            Ok(d)
        }).filter(|d| if let &Ok(ref d) = d {
            query.matches(&d.info)
        } else {
            true
        }).collect()
}

fn main_result() -> Result<i32, Error> {
    env_logger::init();

    let backend_values: Vec<_> = Backend::values().iter()
        .map(|v| v.to_string()).collect();
    let backend_values: Vec<_> = backend_values.iter().map(|v| &v[..]).collect();

    let app = App::new("ddcset")
        .version(env!("CARGO_PKG_VERSION"))
        .author("arcnmx")
        .about("DDC/CI monitor control")
        .arg(Arg::with_name("backend")
            .short("b")
            .long("backend")
            .value_name("BACKEND")
            .takes_value(true)
            .multiple(true)
            .number_of_values(1)
            .possible_values(&backend_values)
            .help("Backend driver whitelist")
        ).arg(Arg::with_name("id")
            .short("i")
            .long("id")
            .value_name("ID")
            .takes_value(true)
            .help("Filter by matching backend ID")
        ).arg(Arg::with_name("manufacturer")
            .short("g")
            .long("mfg")
            .value_name("MANUFACTURER")
            .takes_value(true)
            .help("Filter by matching manufacturer ID")
        ).arg(Arg::with_name("model")
            .short("l")
            .long("model")
            .value_name("MODEL NAME")
            .takes_value(true)
            .help("Filter by matching model")
        ).arg(Arg::with_name("serial")
            .short("n")
            .long("sn")
            .value_name("SERIAL")
            .takes_value(true)
            .help("Filter by matching serial number")
            // TODO: filter by index? winapi makes things difficult, nothing is identifying...
        ).subcommand(SubCommand::with_name("detect")
            .about("List detected displays")
            .arg(Arg::with_name("caps")
                .short("c")
                .long("capabilities")
                .help("Read display capabilities")
            )
        ).subcommand(SubCommand::with_name("capabilities")
            .about("Query display capabilities")
        ).subcommand(SubCommand::with_name("getvcp")
            .about("Get VCP feature value")
            .arg(Arg::with_name("feature")
                .value_name("FEATURE CODE")
                .takes_value(true)
                .multiple(true)
                .help("Feature code (hexadecimal)")
            ).arg(Arg::with_name("raw")
                .short("r")
                .long("raw")
                .help("Show raw value")
            ).arg(Arg::with_name("table")
                .short("t")
                .long("table")
                .help("Read as table value")
            ).arg(Arg::with_name("caps")
                .short("c")
                .long("capabilities")
                .help("Read display capabilities")
            ).arg(Arg::with_name("scan")
                .short("s")
                .long("scan")
                .help("Scan all VCP feature codes")
            )
        ).subcommand(SubCommand::with_name("setvcp")
            .about("Set VCP feature value")
            .arg(Arg::with_name("feature")
                .value_name("FEATURE CODE")
                .takes_value(true)
                .required(true)
                .help("Feature code hexadecimal")
            ).arg(Arg::with_name("value")
                .value_name("VALUE")
                .takes_value(true)
                .required(true)
                .help("Value to set")
            ).arg(Arg::with_name("verify")
                .short("v")
                .long("verify")
                .help("Read value after writing")
            ).arg(Arg::with_name("table")
                .short("t")
                .long("table")
                .help("VALUE becomes a hex string")
            ).arg(Arg::with_name("offset")
                .short("o")
                .long("offset")
                .help("Table write offset")
            )
        ).setting(AppSettings::SubcommandRequiredElseHelp);

    let matches = app.get_matches();

    let mut query = Query::Any;
    let mut needs_caps = false;
    if let Some(backends) = matches.values_of("backend").map(|v| v.map(Backend::from_str)) {
        let backends = backends
            .map(|b| b.map(Query::Backend))
            .collect::<Result<_, _>>().unwrap();
        query = Query::And(vec![query, Query::Or(backends)])
    }
    if let Some(id) = matches.value_of("id") {
        query = Query::And(vec![query, Query::Id(id.into())])
    }
    if let Some(manufacturer) = matches.value_of("manufacturer") {
        query = Query::And(vec![query, Query::ManufacturerId(manufacturer.into())])
    }
    if let Some(model) = matches.value_of("model") {
        query = Query::And(vec![query, Query::ModelName(model.into())]);
        needs_caps = true;
    }
    if let Some(serial) = matches.value_of("serial") {
        query = Query::And(vec![query, Query::SerialNumber(serial.into())])
    }

    let query = (query, needs_caps);

    let mut sleep = DisplaySleep::default();

    match matches.subcommand() {
        ("detect", Some(matches)) => {
            let opt_caps = matches.is_present("caps");

            for mut display in displays(query)? {
                {
                    let _ = display.update_from_ddc();
                    println!("Display on {}:", display.info.backend);
                    println!("\tID: {}", display.info.id);
                    if let Some(value) = display.info.manufacturer_id.as_ref() {
                        println!("\tManufacturer ID: {}", value);
                    }
                    if opt_caps {
                        if let Err(e) = display.update_capabilities() {
                            error!("Failed to update capabilities: {}", e);
                        }
                    }
                    if let Some(value) = display.info.model_name.as_ref() {
                        println!("\tModel: {}", value);
                    }
                    if let Some(value) = display.info.serial_number.as_ref() {
                        println!("\tSerial: {}", value);
                    }
                    if let Some(value) = display.info.mccs_version.as_ref() {
                        println!("\tMCCS: {}", value);
                    } else {
                        println!("\tMCCS: Unavailable");
                    }
                }

                sleep.add(display);
            }

            Ok(0)
        },
        ("capabilities", Some(..)) => {
            let mut exit_code = 0;
            for mut display in displays(query)? {
                if let Err(e) = (|| -> Result<(), Error> {
                    println!("Display on {}:", display.info.backend);
                    println!("\tID: {}", display.info.id);
                    display.update_capabilities()?;
                    for feature in (0..0x100).filter_map(|v| display.info.mccs_database.get(v as _)) {
                        println!("\tFeature 0x{:02x}: {}", feature.code, feature.name.as_ref().map(|v| &v[..]).unwrap_or("Unknown"));
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
                            ValueType::NonContinuous { ref values, .. } => {
                                for (value, name) in values {
                                    println!("\t\t\t0x{:02x}: {}", value, name.as_ref().map(|v| &v[..]).unwrap_or("Unknown"));
                                }
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
        ("getvcp", Some(matches)) => {
            let codes = matches.values_of("feature")
                .map(|s|
                    s.map(|s| FeatureCode::from_str_radix(s, 16).or_else(|_| FeatureCode::from_str(s)))
                    .collect::<Result<Vec<_>, _>>()
                ).transpose()?;

            let opt_raw = matches.is_present("raw");
            let opt_table = matches.is_present("table");
            let opt_caps = matches.is_present("caps");
            let opt_scan = matches.is_present("scan");

            let mut exit_code = 0;
            for mut display in displays(query)? {
                println!("Display on {}:", display.info.backend);
                println!("\tID: {}", display.info.id);
                if let Err(e) = (|| -> Result<(), Error> {
                    let codes = if let Some(codes) = codes.clone() {
                        let _ = display.update_from_ddc();
                        if opt_caps || display.info.mccs_version.is_none() {
                            display.update_capabilities()?;
                        }
                        codes
                    } else {
                        if !opt_scan {
                            display.update_capabilities()?;
                            (0..0x100).map(|v| v as FeatureCode).filter(|&c| display.info.mccs_database.get(c).is_some()).collect()
                        } else {
                            (0..0x100).map(|v| v as FeatureCode).collect()
                        }
                    };

                    for code in codes {
                        let feature = display.info.mccs_database.get(code);
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
                                        println!("\tFeature 0x{:02x} = {}", code, ValueInterpretation::Continuous.format(&value));
                                    },
                                    ValueType::Continuous { mut interpretation } => {
                                        let value = handle.get_vcp_feature(code)?;
                                        if opt_raw {
                                            interpretation = ValueInterpretation::Continuous;
                                        }
                                        println!("\tFeature 0x{:02x} = {}", feature.code, interpretation.format(&value))
                                    },
                                    ValueType::NonContinuous { ref values, mut interpretation } => {
                                        if opt_raw {
                                            interpretation = ValueInterpretation::Continuous;
                                        }

                                        let value = handle.get_vcp_feature(code)?;
                                        if let Some(&Some(ref name)) = values.get(&(value.value() as u8)) {
                                            println!("\tFeature 0x{:02x} = {}: {}", feature.code, interpretation.format(&value), name)
                                        } else {
                                            println!("\tFeature 0x{:02x} = {}", feature.code, interpretation.format(&value))
                                        }
                                    },
                                    ValueType::Table { mut interpretation } => {
                                        if opt_raw {
                                            interpretation = TableInterpretation::Generic;
                                        }

                                        let value = handle.table_read(code)?;
                                        println!("\tFeature 0x{:02x} = {}", code, interpretation.format(&value).map_err(|_| format_err!("table interpretation failed"))?);
                                    },
                                }
                            } else {
                                if opt_table {
                                    let value = handle.table_read(code)?;
                                    println!("\tFeature 0x{:02x} = {}", code, TableInterpretation::Generic.format(&value).unwrap());
                                } else {
                                    let value = handle.get_vcp_feature(code)?;
                                    println!("\tFeature 0x{:02x} = {}", code, ValueInterpretation::Continuous.format(&value));
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
        ("setvcp", Some(matches)) => {
            let feature = matches.value_of("feature").map(|s| FeatureCode::from_str_radix(s, 16).or_else(|_| FeatureCode::from_str(s))).unwrap()?;
            let opt_offset = matches.value_of("offset").map(u16::from_str).unwrap_or(Ok(0))?;
            let opt_table = matches.is_present("table");
            let opt_verify = matches.is_present("verify");

            let value = if opt_table {
                Err(hex::decode(matches.value_of("value").unwrap())?)
            } else {
                Ok(matches.value_of("value").map(u16::from_str).unwrap()?)
            };

            let mut exit_code = 0;
            for mut display in displays(query)? {
                println!("Display on {}:", display.info.backend);
                println!("\tID: {}", display.info.id);

                if let Err(e) = (|| -> Result<(), Error> {
                    match value {
                        Ok(value) => display.handle.set_vcp_feature(feature, value),
                        Err(ref table) => display.handle.table_write(feature, opt_offset, &table),
                    }?;

                    if opt_verify {
                        let matches = match value {
                            Ok(value) => display.handle.get_vcp_feature(feature)?.value() == value,
                            Err(ref table) => &display.handle.table_read(feature)? == table,
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
        _ => unreachable!("unknown command"),
    }
}
