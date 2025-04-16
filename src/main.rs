use {
	::clap::{Parser, Subcommand},
	::markdoll::{emit::html::HtmlEmit, ext, MarkDoll},
	::miette::Report,
	::serde_json::json,
	::std::io::Read,
	::tracing::error_span,
};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Command,
	/// emit machine-readable JSON information to stderr
	///
	/// refer to the json output section of the readme for details
	#[arg(long, global = true)]
	json: bool,
	/// do not emit status updates
	#[arg(long, global = true)]
	no_status: bool,
	/// always complete all stages
	///
	/// not recommended, as the state after an error occurs is not defined
	#[arg(long, global = true)]
	idc: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
	/// check the provided stdin and print any parsing errors
	Check,
	/// convert the provided stdin to html and output to stdout
	Convert,
}

fn status_update(stage: &'static str, status: &'static str, json: bool) {
	if json {
		eprintln!(
			"{}",
			json!({
				"kind": "status-update",
				"stage": stage,
				"status": status,
			})
		);
	} else {
		eprintln!("[{stage}] {status}");
	}
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
	doll.add_tags(ext::all_tags());
	#[cfg(feature = "danger")]
	{
		doll.add_tags(ext::danger::tags());
	}
	doll.builtin_emitters.put(HtmlEmit::default_emitters());

	if !args.no_status {
		status_update("parse", "working", args.json);
	}

	let (mut ok, mut diagnostics, _, mut ast) = doll.parse_document("stdin".to_string(), src, None);

	if !args.no_status {
		status_update("parse", if ok { "success" } else { "failure" }, args.json);
	}

	if ok || args.idc {
		if let Command::Convert = args.command {
			let mut out = HtmlEmit {
				write: String::new(),
				section_level: 1,
			};

			if !args.no_status {
				status_update("emit", "working", args.json);
			}

			let (emit_ok, mut emit_diagnostics) = doll.emit(&mut ast, &mut out, &mut ());
			diagnostics.append(&mut emit_diagnostics);
			ok &= emit_ok;

			if !args.no_status {
				status_update(
					"emit",
					if emit_ok { "success" } else { "failure" },
					args.json,
				);
			}

			if ok || args.idc {
				print!("{}", out.write);

				if !args.no_status {
					status_update("emit", "written", args.json);
				}
			}
		}
	}

	let source = doll.finish();
	// let report = Report::from(diagnostic).with_source_code(source.clone());
	// report.labels().unwrap().map(|a| a.)

	if args.json {
		eprintln!(
			"{}",
			json!({
				"kind": "diagnostics",
				"diagnostics": diagnostics.into_iter().map(|diagnostic| {
					let _traced = error_span!("diagnostic", ?diagnostic).entered();

					let report = Report::from(diagnostic).with_source_code(source.clone());

					json!({
						"message": report.to_string(),
						"code": report.code().map(|code| code.to_string()),
						"severity": match report.severity() {
							Some(::miette::Severity::Advice) => "advice",
							Some(::miette::Severity::Warning) => "warning",
							Some(::miette::Severity::Error) | None => "error",
						},
						"help": report.help().map(|code| code.to_string()),
						"url": report.url().map(|code| code.to_string()),
						"labels": report.labels().map(|labels| labels.map(|label| {
							if let Some(src) = report.source_code() {
								if let Ok(span) = src.read_span(label.inner(), 0, 0) {
									return json!({
										"primary": label.primary(),
										"label": label.label(),
										"location": format!("{}:{}:{}", span.name().unwrap_or("<unknown>"), span.line() + 1, span.column() + 1),
									});
								}
							}

							json!({
								"primary": label.primary(),
								"label": label.label(),
								"location": "unknown",
							})
						}).collect::<Vec<_>>()),
						"cause_chain": report.chain().map(|cause| cause.to_string()).collect::<Vec<_>>(),
						"rendered": format!("{report:?}"),
					})
				}).collect::<Vec<_>>()
			})
		);
	} else {
		for diagnostic in diagnostics {
			let _traced = error_span!("diagnostic", ?diagnostic).entered();

			eprintln!(
				"{:?}",
				Report::from(diagnostic).with_source_code(source.clone())
			);
		}
	}

	if !ok {
		::std::process::exit(1);
	}
}
