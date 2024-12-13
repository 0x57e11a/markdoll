#![cfg(feature = "cli")]

use {
	::clap::{Parser, Subcommand},
	::hashbrown::HashMap,
	::markdoll::{
		diagnostics,
		emit::{BuiltInEmitters, HtmlEmit},
		ext, MarkDoll,
	},
	::std::{io::Read, rc::Rc},
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
	/// check the provided stdin and print any parsing errors
	Check,
	/// convert the provided stdin to html and output to stdout
	Convert,
}

fn main() {
	env_logger::init();

	let args = Cli::parse();

	let mut src = String::new();

	::std::io::stdin()
		.read_to_string(&mut src)
		.expect("failed to read stdin");

	let mut doll = MarkDoll::new();
	doll.ext_system.add_tags(ext::common::tags());
	doll.ext_system.add_tags(ext::formatting::tags());
	doll.ext_system.add_tags(ext::code::tags());
	doll.ext_system.add_tags(ext::links::tags());
	doll.ext_system.add_tags(ext::table::tags());
	doll.set_emitters(BuiltInEmitters::<HtmlEmit>::default());

	log::info!("parse");

	let mut ok = true;

	match doll.parse_document(&src) {
		Ok((_, mut ast)) => match args.command {
			Command::Check => {
				log::info!("parse succeeded")
			}
			Command::Convert => {
				log::info!("emitting");

				let mut out = HtmlEmit {
					write: String::new(),
					section_level: 0,
					code_block_format: Rc::new(|_, _, _, _| {}),
				};

				if doll.emit(&mut ast, &mut out) {
					log::info!("output written to stdout");

					print!("{}", out.write);
				} else {
					log::error!("emit failed");
					ok = false;
				}
			}
		},
		Err(_) => {
			log::error!("parse failed");
			ok = false;
		}
	}

	log::info!("diagnostics");

	let mut cache = ariadne::Source::from(&src);

	for report in diagnostics::render(&doll.finish()) {
		report.eprint(&mut cache).unwrap();
	}

	if ok {
		log::info!("end");
	} else {
		log::error!("failed");
		::std::process::exit(1);
	}
}
