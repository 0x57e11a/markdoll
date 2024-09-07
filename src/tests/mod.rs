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
	doll.ext_system.add_tag(ext::common::COMMENT_TAG);
	doll.ext_system.add_tag(ext::formatting::EMPHASIS_TAG);
	doll.ext_system.add_tag(ext::formatting::QUOTE_TAG);
	doll.ext_system.add_tag(ext::code::CODE_TAG);
	doll.ext_system.add_tag(ext::code::CODEBLOCK_TAG);
	doll.ext_system.add_tag(ext::links::LINK_TAG);
	doll.ext_system.add_tag(ext::links::DEF_TAG);
	doll.ext_system.add_tag(ext::links::REF_TAG);
	doll.ext_system.add_tag(ext::table::TBL_TAG);
	doll.ext_system.add_tag(ext::table::TBLROW_TAG);
	doll.ext_system.add_tag(ext::table::TBLCELL_TAG);

	println!("parse");

	doll.begin(SRC);

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
