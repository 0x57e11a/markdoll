use {
	::hashbrown::HashMap,
	::markdoll::emit::{BuiltInEmitters, HtmlEmit},
	ariadne::Source,
	markdoll::{diagnostics::render, ext, MarkDoll},
};

#[test]
pub fn parses_own_spec() {
	env_logger::builder()
		.target(env_logger::Target::Pipe(Box::new(
			std::fs::File::create("target/trace.txt").unwrap(),
		)))
		.filter_level(log::LevelFilter::Trace)
		.default_format()
		.init();

	const SRC: &'static str = include_str!("../spec.doll");

	let mut out = HtmlEmit {
		write: String::new(),
		section_level: 0,
		code_block_format: HashMap::new(),
	};

	out.code_block_format
		.insert("doll", |_: &mut MarkDoll, to: &mut HtmlEmit, text: &str| {
			to.write
				.push_str(&format!("<pre>{}</pre>", &html_escape::encode_text(&text)));
		});

	let mut doll = MarkDoll::new();
	doll.ext_system.add_tags(ext::common::tags());
	doll.ext_system.add_tags(ext::formatting::tags());
	doll.ext_system.add_tags(ext::code::tags());
	doll.ext_system.add_tags(ext::links::tags());
	doll.ext_system.add_tags(ext::table::tags());
	doll.set_emitters(BuiltInEmitters::<HtmlEmit>::default());

	println!("parse");

	let mut ok = true;

	match doll.parse_document(SRC) {
		Ok((frontmatter, mut ast)) => {
			println!("frontmatter: {frontmatter:?}");
			println!("emitting");

			if doll.emit(&mut ast, &mut out) {
				println!("output written to spec.html");

				std::fs::write("./spec.html", out.write).unwrap();
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
