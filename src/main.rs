#![cfg(feature = "cli")]

use {
	clap::{Parser, Subcommand},
	markdoll::MarkDoll,
	std::io::Read,
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

	std::io::stdin()
		.read_to_string(&mut src)
		.expect("failed to read stdin");

	let mut out = String::new();

	let mut doll = MarkDoll::new();
	doll.ext_system.add_tags(markdoll::ext::common::TAGS);
	doll.ext_system.add_tags(markdoll::ext::formatting::TAGS);
	doll.ext_system.add_tags(markdoll::ext::code::TAGS);
	doll.ext_system.add_tags(markdoll::ext::links::TAGS);
	doll.ext_system.add_tags(markdoll::ext::table::TAGS);

	log::info!("parse");

	let mut ok = true;

	match doll.parse(&src) {
		Ok(mut ast) => match args.command {
			Command::Check => {
				log::info!("parse succeeded")
			}
			Command::Convert => {
				log::info!("emitting");

				if doll.emit(&mut ast, &mut out) {
					log::info!("output written to stdout");

					print!("{}", out);
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

	for report in markdoll::diagnostics::render(&doll.finish()) {
		report.eprint(&mut cache).unwrap();
	}

	if ok {
		log::info!("end");
	} else {
		log::error!("failed");
		std::process::exit(1);
	}
}
