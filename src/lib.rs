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

#[derive(Debug)]
pub enum SourceMetadata {
	File(String),
	LineTag(Span),
	BlockTag(TagDiagnosticTranslation),
	TagArgument(Span),
}

#[derive(Debug)]
pub struct MarkDollSrc {
	pub metadata: SourceMetadata,
	pub source: String,
}

impl BufferSource for MarkDollSrc {
	fn source(&self) -> &str {
		&self.source
	}

	fn name(&self) -> Option<&str> {
		Some(match &self.metadata {
			SourceMetadata::File(filename) => filename,
			SourceMetadata::LineTag(_) => "<line tag>",
			SourceMetadata::BlockTag(_) => "<block tag>",
			SourceMetadata::TagArgument(_) => "<tag argument>",
		})
	}
}

/// markdoll's main context
#[derive(Debug)]
pub struct MarkDoll {
	/// the tags registered
	pub tags: HashMap<&'static str, TagDefinition>,

	pub builtin_emitters: Emitters<BuiltInEmitters<()>>,

	/// whether the current operation is "ok"
	///
	/// this shouldn't really be set to `true` by anything except the language
	pub ok: bool,
	pub(crate) diagnostics: Vec<DiagnosticKind>,
	pub spanner: Spanner<MarkDollSrc>,
}

impl MarkDoll {
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
	pub fn add_tag(&mut self, tag: TagDefinition) {
		self.tags.insert(tag.key, tag);
	}

	/// add multiple tags
	pub fn add_tags<const N: usize>(&mut self, tags: [TagDefinition; N]) {
		for tag in tags {
			self.add_tag(tag);
		}
	}

	/// parse the input into an AST, used to parse the content of tags in an existing parse operation
	///
	/// returns whether the fragment parsed successfully, and the produced [`AST`]
	///
	/// # errors
	///
	/// if the operation does not succeed, the [`AST`] may be in an incomplete/incorrect state
	#[instrument(skip(self), level = Level::INFO)]
	pub fn parse_embedded(&mut self, src: Span) -> (bool, AST) {
		// stash state
		let old_ok = ::core::mem::replace(&mut self.ok, true);

		// parse
		let mut ctx = parser::Ctx::new(self, src);
		let (ok, ast) = parser::parse(&mut ctx);

		// restore stash
		let _ = ::core::mem::replace(&mut self.ok, old_ok);

		(ok, ast)
	}

	/// parse a complete document into an AST, including frontmatter
	///
	/// returns
	/// - whether the operation was successful
	/// - the diagnostics produced during the operation (may not be ampty on success)
	/// - the frontmatter
	/// - the [AST]
	#[instrument(skip(self), level = Level::INFO, ret)]
	pub fn parse_document(
		&mut self,
		filename: String,
		source: String,
	) -> (bool, Vec<DiagnosticKind>, Option<String>, AST) {
		// stash state
		let old_ok = ::core::mem::replace(&mut self.ok, true);
		let old_diagnostics = ::core::mem::replace(&mut self.diagnostics, Vec::new());

		// parse
		let buf = self.spanner.add(|_| MarkDollSrc {
			metadata: SourceMetadata::File(filename),
			source,
		});
		let mut ctx = parser::Ctx::new(self, buf.span());
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
	#[instrument(skip(self), level = Level::INFO)]
	pub fn emit<To: Debug + 'static>(
		&mut self,
		ast: &mut AST,
		to: &mut To,
	) -> (bool, Vec<DiagnosticKind>) {
		// stash state
		let old_ok = ::core::mem::replace(&mut self.ok, true);
		let old_diagnostics = ::core::mem::replace(&mut self.diagnostics, Vec::new());

		// emit
		for Spanned(_, node) in ast {
			node.emit(self, to, true);
		}

		// restore stash
		let _ = ::core::mem::replace(&mut self.ok, old_ok);
		let diagnostics = ::core::mem::replace(&mut self.diagnostics, old_diagnostics);

		(self.ok, diagnostics)
	}

	/// finish a set of files and prepare to render diagnostics
	///
	/// returns the shared spanner to use with [Report::with_source_code](::miette::Report::with_source_code)
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
	pub fn resolve_span(&mut self, mut span: Span) -> (SourceSpan, Vec<LabeledSpan>) {
		let mut init = span;
		let mut labels = Vec::new();

		loop {
			let file = &self.spanner.lookup_buf(span.start());
			span = match &file.src.metadata {
				SourceMetadata::File(_) => break,
				SourceMetadata::TagArgument(new) | SourceMetadata::LineTag(new) => {
					labels.push(LabeledSpan::new_with_span(
						Some("⇣ originates from".to_string()),
						self.spanner.lookup_linear_index(new.start())
							..self.spanner.lookup_linear_index(new.end()),
					));
					*new
				}
				SourceMetadata::BlockTag(trans) => {
					let parent = trans.to_parent(&self.spanner, span);
					if let Some(label) = labels.pop() {
						labels.push(LabeledSpan::new_with_span(
							label.label().map(ToString::to_string),
							self.spanner.lookup_linear_index(parent.start())
								..self.spanner.lookup_linear_index(parent.end()),
						));
					} else {
						init = parent
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

impl Default for MarkDoll {
	fn default() -> Self {
		Self::new()
	}
}
