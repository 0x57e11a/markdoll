/// `code`/`codeblock` tags
pub mod code;
/// `//` tag
pub mod common;
/// `em`/`quote` tags
pub mod formatting;
/// `link`/`def`/`ref` tags
pub mod links;
/// `table`/`tr`/`tc` tags
pub mod table;

pub use emitters::Emitters;
use {
	crate::{tree::TagContent, MarkDoll, MarkDollSrc},
	::miette::{LabeledSpan, SourceSpan},
	::spanner::{Span, SrcSpan},
};

mod emitters;

/// the parsing signature tags use
pub type TagParser<Ctx> = fn(
	doll: &mut MarkDoll<Ctx>,
	args: Vec<SrcSpan<MarkDollSrc>>,
	text: SrcSpan<MarkDollSrc>,
	tag_span: Span,
) -> Option<Box<dyn TagContent>>;
/// the emitting signature tags use for a given `To`
pub type TagEmitter<Ctx, To = ()> = fn(
	doll: &mut MarkDoll<Ctx>,
	to: &mut To,
	ctx: &mut Ctx,
	content: &mut dyn TagContent,
	tag_span: Span,
);

/// defines a tag name, how to parse its contents, and how to emit it
#[derive(Debug, Clone)]
#[allow(
	clippy::type_complexity,
	reason = "type is never mentioned outside of this struct, simple functions"
)]
pub struct TagDefinition<Ctx> {
	/// the tag key
	pub key: &'static str,

	/// parse the tag contents
	pub parse: TagParser<Ctx>,

	/// emit the tag content
	pub emitters: Emitters<TagEmitter<Ctx>>,
}

/// helper macro to parse arguments into variables
///
/// ```rs
/// args! {
///     args; // pass the args
///     doll, tag_span; // pass in the markdoll and args
///
///     args(arg1, arg2: usize); // parse required arguments, which may be parsed into another type, if applicable. ex: `(2)`
///     opt_args(oarg1, oarg2: usize); // parse optional arguments, which will be `Some` when present (and parsed into another type, if applicable), or `None` if not. ex: `(2)`
///     flags(flag1, flag2); // parse flags, which will be `true` when present and `false` when not. ex: `(flag2)`
///     props(oarg1, oarg2: usize); // parse named props, which will be `Some` when present (and parsed into another type, if applicable), or `None` if not. ex: `(oarg2=2)`
/// }
/// ```
#[macro_export]
macro_rules! args {
	{
		$args:ident;
		$doll:ident, $tag_span:ident;

		$(args($($arg:ident$(: $arg_ty:ty)?),*);)?
		$(opt_args($($opt_arg:ident$(: $opt_arg_ty:ty)?),*);)?
		$(flags($($flag:ident),*);)?
		$(props($($prop:ident$(: $prop_ty:ty)?),*);)?
	} => {
		#[allow(unused, reason = "macro")]
		let (mut doll, mut args) = (&mut *$doll, $args);

		args! {
			&mut args;
			doll, $tag_span;

			$(args($($arg$(: $arg_ty)?),*);)?
			$(opt_args($($opt_arg$(: $opt_arg_ty)?),*);)?
			$(flags($($flag),*);)?
			$(props($($prop$(: $prop_ty)?),*);)?
		}

		for arg in args {
			let (at, context) = doll.resolve_span(arg.into());
			doll.diag($crate::ext::TagInputDiagnostic::ExtraneousInput {
				at,
				context,
			}.into());
		}
	};

	{
		&mut $args:ident;
		$doll:ident, $tag_span:ident;

		$(args($($arg:ident$(: $arg_ty:ty)?),*);)?
		$(opt_args($($opt_arg:ident$(: $opt_arg_ty:ty)?),*);)?
		$(flags($($flag:ident),*);)?
		$(props($($prop:ident$(: $prop_ty:ty)?),*);)?
	} => {
		$($(let $arg;)*)?
		$($(let $opt_arg;)*)?
		$($(let mut $flag = false;)*)?
		$($(let mut $prop = args! {
			@if [$($prop_ty)?] {
				Option::<$($prop_ty)?>::None
			} else {
				Option::<$crate::spanner::SrcSpan<$crate::MarkDollSrc>>::None
			}
		};)*)?

		#[allow(unused, reason = "macro")]
		{
			let (doll, args, tag_span, mut arg_i) = (&mut $doll, &mut $args, &$tag_span, 0);

			$($(
				arg_i += 1;
				$arg = if !args.is_empty() {
					args! {
						@if [$($arg_ty)?] {
							let span = args.remove(0);
							#[allow(irrefutable_let_patterns, reason = "macro")]
							match span.parse::<$($arg_ty)?>() {
								Ok(value) => value,
								Err(reason) => {
									let (at, context) = doll.resolve_span(span.into());
									doll.diag($crate::ext::TagInputDiagnostic::InvalidArgument {
										num: arg_i,
										name: stringify!($arg),
										reason: reason.to_string(),
										at,
										context,
									}.into());

									return None;
								}
							}
						} else {
							args.remove(0)
						}
					}
				} else {
					let (at, context) = doll.resolve_span(*tag_span);
					doll.diag($crate::ext::TagInputDiagnostic::MissingArgument {
						num: arg_i,
						name: stringify!($arg),
						at,
						context,
					}.into());

					return None;
				};
			)*)?

			$($(
				arg_i += 1;
				$opt_arg = if !args.is_empty() {
					Some(args! {
						@if [$($opt_arg_ty)?] {
							let span = args.remove(0);
							#[allow(irrefutable_let_patterns, reason = "macro")]
							if let Ok(value) = span.parse::<$($opt_arg_ty)?>() {
								value
							} else {
								let (at, context) = doll.resolve_span(span.into());
								doll.diag($crate::ext::TagInputDiagnostic::InvalidArgument {
									num: arg_i,
									name: stringify!($opt_arg),
									at,
									context,
								}.into());

								return None;
							}
						} else {
							args.remove(0)
						}
					})
				} else {
					None
				};
			)*)?

			args! {
				@if [$($($flag)*)? $($($prop)*)?] {
					let mut retain_ok = true;

					args.retain(|arg| match &**arg {
						$($(
							stringify!($flag) => {
								$flag = true;
								false
							}
						)*)?
						input => args! {
							@if [$($($prop)*)?] {
								// parse properties
								if let Some(index) = input.find("=") {
									match &input[..index] {
										$($(
											stringify!($prop) => {
												let span = arg.subspan((u32::try_from(index).unwrap() + 1)..);
												args! {
													@if [$($prop_ty)?] {
														match span.parse::<$($prop_ty)?>() {
															Ok(value) => {
																$prop = Some(value);
															}
															Err(reason) => {
																let (at, context) = doll.resolve_span(span.into());
																doll.diag($crate::ext::TagInputDiagnostic::InvalidProperty {
																	name: stringify!($prop),
																	reason: reason.to_string(),
																	at,
																	context,
																}.into());

																retain_ok = false;
															}
														}
													} else {
														$prop = Some(span);
													}
												};
												false
											}
										)*)?
										_ => true,
									}
								} else {
									true
								}
							} else {
								// no properties
								true
							}
						},
					});

					if !retain_ok {
						return None;
					}
				} else {}
			};
		}
	};

	{ @if [] $true:tt else $false:tt } => { $false };
	{ @if [$($tok:ident)+] $true:tt else $false:tt } => { $true };
	{ @if [$($tok:ty)+] $true:tt else $false:tt } => { $true };
}

/// tag input diagnostic
#[derive(Debug, ::thiserror::Error, ::miette::Diagnostic)]
pub enum TagInputDiagnostic {
	/// missing an argument
	#[error("missing argument #{num} `{name}`")]
	#[diagnostic(code(markdoll::tag::missing_arg))]
	MissingArgument {
		/// argument number
		num: usize,
		/// argument name
		name: &'static str,
		/// tag span
		#[label]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
	/// failed to parse an argument
	#[error("invalid argument #{num} `{name}`")]
	#[diagnostic(code(markdoll::tag::invalid_arg))]
	InvalidArgument {
		/// argument number
		num: usize,
		/// argument name
		name: &'static str,
		/// the reason it failed
		reason: String,
		/// argument span
		#[label("failed to parse: {}", .reason)]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
	/// failed to parse a property
	#[error("invalid property `{name}`")]
	#[diagnostic(code(markdoll::tag::invalid_prop))]
	InvalidProperty {
		/// property name
		name: &'static str,
		/// the reason it failed
		reason: String,
		/// property span
		#[label("failed to parse: {}", .reason)]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
	/// unused input
	#[error("tag does not use this input")]
	#[diagnostic(code(markdoll::tag::unused_input), severity(warning))]
	ExtraneousInput {
		/// input span
		#[label("extraneous input")]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
}

/// all the tags defined in the standard library
#[must_use]
pub fn all_tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	code::tags()
		.into_iter()
		.chain(common::tags())
		.chain(formatting::tags())
		.chain(links::tags())
		.chain(table::tags())
}
