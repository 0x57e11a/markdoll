use {
	crate::{diagnostics::render, ext, MarkDoll},
	alloc::{boxed::Box, string::String},
	ariadne::Source,
};

extern crate std;
use std::println;

#[test]
pub fn test_syntax() {
	env_logger::builder()
		.target(env_logger::Target::Pipe(Box::new(
			std::fs::File::create("target/trace.txt").unwrap(),
		)))
		.filter_level(log::LevelFilter::Trace)
		.default_format()
		.init();

	const SRC: &'static str = include_str!("../../spec.doll");

	let mut out = String::new();

	let mut doll = MarkDoll::new();
	doll.ext_system.add_tags(ext::common::TAGS);
	doll.ext_system.add_tags(ext::formatting::TAGS);
	doll.ext_system.add_tags(ext::code::TAGS);
	doll.ext_system.add_tags(ext::links::TAGS);
	doll.ext_system.add_tags(ext::table::TAGS);

	println!("parse");

	let mut ok = true;

	match doll.parse(SRC) {
		Ok(mut ast) => {
			println!("emitting");

			if doll.emit(&mut ast, &mut out) {
				println!("output written to spec.html");

				std::fs::write("./spec.html", out).unwrap();
			} else {
				println!("emit failed");
				ok = false;
			}
		}
		Err(_) => {
			println!("parse failed");
			ok = false;
		}
	}

	println!("diagnostics");

	let mut cache = Source::from(SRC);

	for report in render(&doll.finish()) {
		report.eprint(&mut cache).unwrap();
	}

	if ok {
		println!("end");
	} else {
		panic!("failed");
	}
}
