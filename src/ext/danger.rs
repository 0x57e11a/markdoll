//! any tags that may be considered dangerous
//!
//! locked behind the `danger` feature

use {
	crate::{
		args,
		emit::html::HtmlEmit,
		ext::{Emitters, TagDefinition, TagEmitter},
		tree::TagContent,
		MarkDoll,
	},
	::miette::SourceSpan,
	::spanner::Span,
	::std::io::Write,
};

/// `invoke` tag
///
/// call an external program, passing the arguments on the command line, piping the content as stdin, and dumping the output into the output stream raw
///
/// # arguments
///
/// - `program`\
///   path to the program to run
/// - rest of arguments are passed as command line args
///
/// # content
///
/// anything
pub mod invoke {
	use {super::*, crate::diagnostics::DiagnosticKind, ::miette::LabeledSpan};

	/// an error with an external command
	#[derive(Debug, ::thiserror::Error, ::miette::Diagnostic)]
	pub enum InvokeDiagnosticKind {
		/// external command io error
		#[error("external command io error")]
		#[diagnostic(code(markdoll::ext::danger::invoke::io))]
		Io {
			/// the program
			#[label("{error}")]
			at: SourceSpan,
			/// where the program was invoked
			#[label(collection)]
			context: Vec<LabeledSpan>,
			/// the error
			error: ::std::io::Error,
		},
		/// non zero exit code
		#[error("non-zero exit code")]
		#[diagnostic(code(markdoll::ext::danger::invoke::non_zero_exit_code))]
		NonZeroExitCode {
			/// the program
			#[label("exited with code {}", exit_code.map_or_else(|| "<unknown>".to_string(), |ec| ec.to_string()))]
			at: SourceSpan,
			/// where the program was invoked
			#[label(collection)]
			context: Vec<LabeledSpan>,
			/// the exit code, if applicable
			exit_code: Option<i32>,
		},
		/// non utf-8 output
		#[error("non-utf8 output")]
		#[diagnostic(code(markdoll::ext::danger::invoke::non_utf8_output))]
		NonUTF8Output {
			/// the program
			#[label("this command did not return valid utf-8 data")]
			at: SourceSpan,
			/// where the program was invoked
			#[label(collection)]
			context: Vec<LabeledSpan>,
		},
	}

	/// the tag
	#[must_use]
	#[allow(clippy::zombie_processes, reason = "kill")]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "invoke",
			parse: |mut doll, mut args, text, tag_span| {
				fn exec<'a>(
					prgm: &'a str,
					args: impl Iterator<Item = &'a str>,
					stdin: &'a str,
				) -> Result<::std::process::Output, ::std::io::Error> {
					let mut child = ::std::process::Command::new(prgm)
						.args(args)
						.stdin(::std::process::Stdio::piped())
						.stdout(::std::process::Stdio::piped())
						.stderr(::std::process::Stdio::piped())
						.spawn()?;
					if let Err(error) = child.stdin.take().unwrap().write_all(stdin.as_bytes()) {
						child.kill().expect("could not kill child");
						return Err(error);
					}
					child.wait_with_output()
				}

				args! {
					&mut args;
					doll, tag_span;

					args(program);
				}

				match exec(&program, args.iter().map(|arg| &**arg), &text) {
					Ok(output) => {
						if !output.status.success() {
							let (at, context) = doll.resolve_span(program.span());
							doll.diag(DiagnosticKind::Tag(Box::new(
								InvokeDiagnosticKind::NonZeroExitCode {
									at,
									context,
									exit_code: output.status.code(),
								},
							)));
						}

						if let Ok(stdout) = String::from_utf8(output.stdout) {
							Some(Box::new(stdout))
						} else {
							let (at, context) = doll.resolve_span(program.span());
							doll.diag(DiagnosticKind::Tag(Box::new(
								InvokeDiagnosticKind::NonUTF8Output { at, context },
							)));
							None
						}
					}
					Err(error) => {
						let (at, context) = doll.resolve_span(program.span());
						doll.diag(DiagnosticKind::Tag(Box::new(InvokeDiagnosticKind::Io {
							at,
							context,
							error,
						})));
						None
					}
				}
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		_: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		_: &mut Ctx,
		content: &mut dyn TagContent,
		_: Span,
	) {
		let stdout = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<String>()
			.unwrap();

		to.write.push_str(stdout);
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	[invoke::tag()]
}
