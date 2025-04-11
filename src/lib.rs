#![doc = include_str!("../README.md")]
#![feature(downcast_unchecked)]
#![warn(
	clippy::pedantic,
	clippy::allow_attributes_without_reason,
	missing_docs
)]
#![allow(
	clippy::missing_panics_doc,
	reason = "lot of unwraps that shouldnt really be hit"
)]
#![allow(clippy::missing_errors_doc, reason = "capitalization :(")]
#![allow(
	clippy::match_wildcard_for_single_variants,
	reason = "future may add more tags"
)]
#![allow(
	clippy::match_same_arms,
	reason = "more confusing to merge in many cases"
)]
#![allow(clippy::wildcard_imports, reason = "used in parsing modules")]
#![allow(
	clippy::module_inception,
	reason = "tag names may share their module name, but it doesn't make sense to merge them"
)]

use {
	crate::{
		diagnostics::{DiagnosticKind, TagDiagnosticTranslation},
		emit::BuiltInEmitters,
		ext::{Emitters, TagDefinition},
		tree::{parser, AST},
	},
	::core::fmt::Debug,
	::hashbrown::HashMap,
	::miette::{Diagnostic, LabeledSpan, Severity, SourceSpan},
	::spanner::{BufferSource, Span, Spanned, Spanner},
	::std::sync::Arc,
	::tracing::{instrument, Level},
};
pub use {::miette, ::spanner, ::thiserror};

/// emitting/translating diagnostics
pub mod diagnostics;
/// emitting output and default [`BuiltInEmitters`]
pub mod emit;
/// the extension system and standard library
pub mod ext;
/// syntax trees and parser
pub mod tree;

/// the metadata of this [`MarkDollSrc`], describing where it came from
#[derive(Debug)]
pub enum SourceMetadata {
	/// this source originates from a file
	File {
		/// filename of the file
		filename: String,
		/// if applicable, the span that referenced this
		referenced_from: Option<Span>,
	},
	/// the content of a line-tag
	LineTag {
		/// what this content is derived from
		from: Span,
		/// whether this content is "verbatim" (exactly matches what's in its containing source)
		verbatim: bool,
	},
	/// the content of a block-tag
	BlockTag {
		/// translation
		translation: TagDiagnosticTranslation,
	},
	/// argument of a tag
	TagArgument {
		/// what this argument is derived from
		from: Span,
		/// whether this argument is "verbatim" (exactly matches what's in its containing source)
		verbatim: bool,
	},
}

/// markdoll source
#[derive(Debug)]
pub struct MarkDollSrc {
	/// metadata containing information about the source's origin
	pub metadata: SourceMetadata,
	/// contents of this source
	pub source: String,
}

impl BufferSource for MarkDollSrc {
	fn source(&self) -> &str {
		&self.source
	}

	fn name(&self) -> Option<&str> {
		Some(match &self.metadata {
			SourceMetadata::File { filename, .. } => filename,
			SourceMetadata::LineTag {
				verbatim: false, ..
			} => "<transformed line tag>",
			SourceMetadata::LineTag { verbatim: true, .. } => "<verbatim line tag>",
			SourceMetadata::BlockTag { .. } => "<block tag>",
			SourceMetadata::TagArgument {
				verbatim: false, ..
			} => "<transformed tag argument>",
			SourceMetadata::TagArgument { verbatim: true, .. } => "<verbatim tag argument>",
		})
	}
}

impl Default for MarkDollSrc {
	fn default() -> Self {
		Self {
			metadata: SourceMetadata::File {
				filename: "empty".to_string(),
				referenced_from: None,
			},
			source: String::new(),
		}
	}
}

/// markdoll's main context
#[derive(Debug)]
pub struct MarkDoll<Ctx = ()> {
	/// the tags registered
	pub tags: HashMap<&'static str, TagDefinition<Ctx>>,

	/// emitters for built-in items
	pub builtin_emitters: Emitters<BuiltInEmitters<Ctx, ()>>,

	/// whether the current operation is "ok"
	///
	/// this shouldn't really be set to `true` by anything except the language
	pub ok: bool,
	/// diagnostics from the current document
	pub diagnostics: Vec<DiagnosticKind>,
	/// source-mapping
	pub spanner: Spanner<MarkDollSrc>,
}

impl<Ctx> MarkDoll<Ctx> {
	/// construct an empty instance with no tags and the default [`BuiltInEmitters`]
	#[must_use]
	pub fn new() -> Self {
		Self {
			tags: HashMap::new(),

			builtin_emitters: Emitters::new(),

			ok: true,
			diagnostics: Vec::new(),
			spanner: Spanner::new(),
		}
	}

	/// add a tag
	pub fn add_tag(&mut self, tag: TagDefinition<Ctx>) {
		self.tags.insert(tag.key, tag);
	}

	/// add multiple tags
	pub fn add_tags(&mut self, tags: impl IntoIterator<Item = TagDefinition<Ctx>>) {
		for tag in tags {
			self.add_tag(tag);
		}
	}

	/// parse the input into an AST, used to parse the content of tags in an existing parse operation
	///
	/// returns the produced [`AST`]
	///
	/// # errors
	///
	/// if the operation does not succeed, the [`AST`] may be in an incomplete/incorrect state
	#[instrument(skip(self), level = Level::INFO)]
	pub fn parse_embedded(&mut self, src: Span) -> AST {
		let mut ctx = parser::ParseCtx::new(self, src);
		let (ok, ast) = parser::parse(&mut ctx);
		self.ok &= ok;
		ast
	}

	/// parse a complete document into an AST, including frontmatter
	///
	/// returns
	/// - whether the operation was successful
	/// - the diagnostics produced during the operation (may not be ampty on success)
	/// - the frontmatter
	/// - the [`AST`]
	#[instrument(skip(self), level = Level::INFO, ret)]
	pub fn parse_document(
		&mut self,
		filename: String,
		source: String,
		referenced_from: Option<Span>,
	) -> (bool, Vec<DiagnosticKind>, Option<String>, AST) {
		// stash state
		let old_ok = ::core::mem::replace(&mut self.ok, true);
		let old_diagnostics = ::core::mem::take(&mut self.diagnostics);

		// parse
		let buf = self.spanner.add(|_| MarkDollSrc {
			metadata: SourceMetadata::File {
				filename,
				referenced_from,
			},
			source,
		});
		let mut ctx = parser::ParseCtx::new(self, buf.span());
		let frontmatter = parser::frontmatter(&mut ctx);
		let (ok, ast) = parser::parse(&mut ctx);

		// restore stash
		let _ = ::core::mem::replace(&mut self.ok, old_ok);
		let diagnostics = ::core::mem::replace(&mut self.diagnostics, old_diagnostics);

		(ok, diagnostics, frontmatter, ast)
	}

	/// emit the given [`AST`] to an output
	///
	/// returns
	/// - whether the operation was successful
	/// - the diagnostics produced during the operation (may not be ampty on success)
	#[instrument(skip(self, ctx), level = Level::INFO)]
	pub fn emit<To: Debug + 'static>(
		&mut self,
		ast: &mut AST,
		to: &mut To,
		ctx: &mut Ctx,
	) -> (bool, Vec<DiagnosticKind>) {
		// stash state
		let old_ok = ::core::mem::replace(&mut self.ok, true);
		let old_diagnostics = ::core::mem::take(&mut self.diagnostics);

		// emit
		for Spanned(_, node) in ast {
			node.emit(self, to, ctx, true);
		}

		// restore stash
		let _ = ::core::mem::replace(&mut self.ok, old_ok);
		let diagnostics = ::core::mem::replace(&mut self.diagnostics, old_diagnostics);

		(self.ok, diagnostics)
	}

	/// finish a set of files and prepare to render diagnostics
	///
	/// returns the shared spanner to use with [`Report::with_source_code`](::miette::Report::with_source_code)
	pub fn finish(&mut self) -> Arc<Spanner<MarkDollSrc>> {
		Arc::new(::core::mem::take(&mut self.spanner))
	}

	/// emit a diagnostic, mapping the position accordingly
	#[track_caller]
	#[instrument(skip(self), level = Level::ERROR)]
	pub fn diag(&mut self, diagnostic: DiagnosticKind) {
		if let None | Some(Severity::Error) = diagnostic.severity() {
			self.ok = false;
		}

		::tracing::info!(origin = %::core::panic::Location::caller(), "rust origin");

		self.diagnostics.push(diagnostic);
	}

	/// returns (outer, inner) span
	#[instrument(skip(self), ret)]
	pub fn resolve_span(&self, mut span: Span) -> (SourceSpan, Vec<LabeledSpan>) {
		let mut init = span;
		let mut labels = Vec::new();

		loop {
			let file = &self.spanner.lookup_buf(span.start());
			span = match &file.src.metadata {
				SourceMetadata::File {
					referenced_from: None,
					..
				} => break,
				SourceMetadata::File {
					referenced_from: Some(ref_from),
					..
				} => {
					labels.push(LabeledSpan::new_with_span(
						Some("referenced by".to_string()),
						self.spanner.lookup_linear_index(ref_from.start())
							..self.spanner.lookup_linear_index(ref_from.end()),
					));
					*ref_from
				}
				SourceMetadata::TagArgument {
					from: new,
					verbatim: true,
				}
				| SourceMetadata::LineTag {
					from: new,
					verbatim: true,
				} => {
					let final_span = (new.start() + span.start().pos).with_len(span.len());
					if let Some(label) = labels.pop() {
						labels.push(LabeledSpan::new_with_span(
							label.label().map(ToString::to_string),
							self.spanner.lookup_linear_index(final_span.start())
								..self.spanner.lookup_linear_index(final_span.end()),
						));
					} else {
						init = final_span;
					}
					final_span
				}
				SourceMetadata::TagArgument {
					from: new,
					verbatim: false,
				}
				| SourceMetadata::LineTag {
					from: new,
					verbatim: false,
				} => {
					labels.push(LabeledSpan::new_with_span(
						Some("from here".to_string()),
						self.spanner.lookup_linear_index(new.start())
							..self.spanner.lookup_linear_index(new.end()),
					));
					*new
				}
				SourceMetadata::BlockTag {
					translation: trans, ..
				} => {
					let parent = trans.to_parent(&self.spanner, span);
					if let Some(label) = labels.pop() {
						labels.push(LabeledSpan::new_with_span(
							label.label().map(ToString::to_string),
							self.spanner.lookup_linear_index(parent.start())
								..self.spanner.lookup_linear_index(parent.end()),
						));
					} else {
						init = parent;
					}
					parent
				}
			}
		}

		(
			(self.spanner.lookup_linear_index(init.start())
				..self.spanner.lookup_linear_index(init.end()))
				.into(),
			labels,
		)
	}
}

impl<Ctx> Default for MarkDoll<Ctx> {
	fn default() -> Self {
		Self::new()
	}
}
