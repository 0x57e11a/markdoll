#![doc = include_str!("../README.md")]
#![no_std]
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

extern crate alloc;

use {
	crate::{
		diagnostics::{Diagnostic, IndexedSrc, TagDiagnosticTranslation},
		emit::BuiltInEmitters,
		ext::ExtensionSystem,
		tree::{parser, AST},
		typemap::TypeMap,
	},
	::alloc::string::String,
	alloc::vec::Vec,
	hashbrown::HashMap,
};

macro_rules! t {
	($text:expr, $expr:expr) => {
		match $expr {
			value => {
				log::trace!("{}: {:#?}", $text, &value);
				value
			}
		}
	};
	($text:literal) => {
		log::trace!($text);
	};
	($expr:expr) => {
		$crate::t!(stringify!($expr), $expr)
	};
}

pub(crate) use t;

/// emitting/translating diagnostics
pub mod diagnostics;
/// emitting output and default [`BuiltInEmitters`]
pub mod emit;
/// the extension system and standard library
pub mod ext;
/// syntax trees and parser
pub mod tree;
/// storage by [`TypeId`](core::any::TypeId)
pub mod typemap;

/// markdoll's main context
#[derive(Debug)]
pub struct MarkDoll {
	/// the extension system, used to add tags
	pub ext_system: ExtensionSystem,

	pub(crate) builtin_emitters: TypeMap,

	/// whether the current operation is "ok"
	///
	/// this shouldn't really be set to `true` by anything except the language
	pub ok: bool,
	pub(crate) diagnostics: Vec<Diagnostic>,
	pub(crate) diagnostic_translations: Vec<TagDiagnosticTranslation>,
}

impl MarkDoll {
	/// construct an empty instance with no tags and the default [`BuiltInEmitters`]
	#[must_use]
	pub fn new() -> Self {
		Self {
			ext_system: ExtensionSystem {
				tags: HashMap::new(),
			},

			builtin_emitters: TypeMap::default(),

			ok: true,
			diagnostics: Vec::new(),
			diagnostic_translations: Vec::new(),
		}
	}

	/// set [`BuiltInEmitters`] for an emit target [`To`]
	pub fn set_emitters<To: 'static>(&mut self, emitters: BuiltInEmitters<To>) {
		self.builtin_emitters.put(emitters);
	}

	/// parse the input into an AST
	///
	/// # errors
	///
	/// if any error diagnostics are emitted, the resulting [`AST`] may be incomplete
	///
	/// # note
	///
	/// ensure that the `finish` method is called to reset the state *before* parsing a new file
	pub fn parse(&mut self, input: &str) -> Result<AST, AST> {
		if self.diagnostic_translations.is_empty() {
			self.diagnostic_translations.push(TagDiagnosticTranslation {
				src: input.into(),
				indexed: None,
				offset_in_parent: 0,
				tag_pos_in_parent: 0,
				indent: 0,
			});
		}
		let ok = self.ok;

		self.ok = true;
		let res = parser::parse(parser::Ctx::new(self, input, false));
		self.ok = ok;

		match res {
			Ok((_, ast)) => Ok(ast),
			Err((_, ast)) => Err(ast),
		}
	}

	/// parse a complete document into an AST, including frontmatter
	///
	/// # errors
	///
	/// if any error diagnostics are emitted, the resulting [`AST`] may be incomplete
	///
	/// # note
	///
	/// ensure that the `finish` method is called to reset the state *before* parsing a new file
	pub fn parse_document(
		&mut self,
		input: &str,
	) -> Result<(Option<String>, AST), (Option<String>, AST)> {
		if self.diagnostic_translations.is_empty() {
			self.diagnostic_translations.push(TagDiagnosticTranslation {
				src: input.into(),
				indexed: None,
				offset_in_parent: 0,
				tag_pos_in_parent: 0,
				indent: 0,
			});
		}
		let ok = self.ok;

		self.ok = true;
		let res = parser::parse(parser::Ctx::new(self, input, true));
		self.ok = ok;

		res
	}

	/// emit the given [`AST`] to an output, returning true if it was successful
	///
	/// # note
	///
	/// ensure that the `finish` method is called to reset the state *before* parsing a new file
	pub fn emit<To: 'static>(&mut self, ast: &mut AST, to: &mut To) -> bool {
		let ok = self.ok;

		self.ok = true;
		for node in ast {
			node.emit(self, to, true);
		}
		self.ok = ok;

		self.ok
	}

	/// ensure that this method is called after parsing a source file, otherwise diagnostics may malfunction
	pub fn finish(&mut self) -> Vec<Diagnostic> {
		self.ok = true;
		self.diagnostic_translations.clear();
		core::mem::take(&mut self.diagnostics)
	}

	/// emit a diagnostic, mapping the position accordingly
	///
	/// pass [`usize::MAX`] to `at` to emit at the tag currently containing this context
	#[track_caller]
	pub fn diag(&mut self, err: bool, mut at: usize, code: &'static str) {
		if err {
			self.ok = false;
		}

		t!("---- begin diag ----");
		t!(at);
		t!(&self.diagnostic_translations);

		let mut i = self.diagnostic_translations.len() - 1;
		while i > 0 {
			let [parent, trans] = &mut self.diagnostic_translations[i - 1..=i] else {
				unreachable!()
			};

			at = if at == usize::MAX {
				trans.tag_pos_in_parent
			} else if let Some(indexed) = &trans.indexed {
				t!(
					"indexed parent offset (prev indexed)",
					indexed.parent_offset(at)
				)
			} else {
				let indexed = IndexedSrc::index(
					&trans.src,
					&parent.src,
					trans.offset_in_parent,
					trans.indent,
				);
				let index = t!("indexed parent offset", indexed.parent_offset(at));
				trans.indexed = Some(indexed);
				index
			};

			i -= 1;
		}

		self.diagnostics.push(Diagnostic {
			err,
			at,
			code,
			#[cfg(debug_assertions)]
			src: core::panic::Location::caller(),
		});
	}
}

impl Default for MarkDoll {
	fn default() -> Self {
		Self::new()
	}
}
