use {
	::clap::{Parser, Subcommand},
	::hashbrown::HashMap,
	::markdoll::{
		diagnostics,
		emit::{html::HtmlEmit, BuiltInEmitters},
		ext, MarkDoll,
	},
	::miette::{miette, Diagnostic, LabeledSpan, Report, SourceCode},
	::std::{io::Read, rc::Rc},
	::tracing::{error_span, trace, trace_span},
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
	#[cfg(feature = "cli-trace")]
	{
		use ::tracing_subscriber::layer::SubscriberExt;

		::tracing::subscriber::set_global_default(::tracing_subscriber::registry().with(
			::tracing_fancytree::FancyTree::new(::std::io::stdout(), true),
		))
		.unwrap();
	}

	let args = Cli::parse();

	let mut src = String::new();

	::std::io::stdin()
		.read_to_string(&mut src)
		.expect("failed to read stdin");

	let mut doll = MarkDoll::new();
	doll.add_tags(ext::common::tags());
	doll.add_tags(ext::formatting::tags());
	doll.add_tags(ext::code::tags());
	doll.add_tags(ext::links::tags());
	doll.add_tags(ext::table::tags());
	doll.builtin_emitters.put(HtmlEmit::default_emitters());

	eprintln!("[parse] parsing...");

	let (mut ok, mut diagnostics, frontmatter, mut ast) =
		doll.parse_document("stdin".to_string(), src, None);

	if ok {
		eprintln!("[parse] complete!");

		if let Command::Convert = args.command {
			let mut out = HtmlEmit {
				write: String::new(),
				section_level: 1,
			};

			eprintln!("[emit] emitting...");

			let (emit_ok, mut emit_diagnostics) = doll.emit(&mut ast, &mut out, &mut ());
			diagnostics.append(&mut emit_diagnostics);

			if ok {
				eprintln!("[emit] complete!");
				eprintln!("[emit] writing output to stdout...");

				print!("{}", out.write);

				eprintln!("[emit] output written!");
			} else {
				eprintln!("[emit] failed");

				ok = false;
			}
		}
	} else {
		eprintln!("[parse] failed");
	}

	eprintln!("diagnostics");

	let source = doll.finish();
	let mut reports = Vec::new();

	for diagnostic in diagnostics {
		let traced = error_span!("diagnostic", ?diagnostic).entered();

		let report = Report::from(diagnostic).with_source_code(source.clone());
		reports.push(format!("{report:?}"));
	}

	for report in reports {
		eprintln!("{report}");
	}

	if ok {
		eprintln!("end");
	} else {
		eprintln!("failed");
		::std::process::exit(1);
	}
}
