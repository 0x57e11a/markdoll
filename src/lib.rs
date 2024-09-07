#![no_std]
#![warn(clippy::pedantic, clippy::allow_attributes_without_reason)]
#![allow(
	clippy::missing_panics_doc,
	reason = "lot of unwraps that shouldnt really be hit"
)]
#![allow(
	clippy::match_wildcard_for_single_variants,
	reason = "future may add more tags"
)]
#![allow(
	clippy::match_same_arms,
	reason = "more confusing to merge in many cases"
)]

use {
	crate::{
		diagnostics::{Diagnostic, IndexedSrc, TagDiagnosticTranslation},
		emit::{BuiltInEmitters, To},
		ext::ExtensionSystem,
		tree::{parser, AST},
	},
	alloc::vec::Vec,
	core::panic::Location,
	hashbrown::HashMap,
};

#[macro_export]
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

extern crate alloc;

pub mod diagnostics;
pub mod emit;
pub mod ext;
pub mod tree;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct MarkDoll {
	pub ext_system: ExtensionSystem,
	pub builtin_emitters: BuiltInEmitters,

	pub(crate) ok: bool,
	pub(crate) diagnostics: Vec<Diagnostic>,
	pub(crate) diagnostic_translations: Vec<TagDiagnosticTranslation>,
}

impl MarkDoll {
	#[must_use]
	pub fn new() -> Self {
		Self {
			ext_system: ExtensionSystem {
				tags: HashMap::new(),
			},
			builtin_emitters: BuiltInEmitters::default(),

			ok: true,
			diagnostics: Vec::new(),
			diagnostic_translations: Vec::new(),
		}
	}

	pub fn begin(&mut self, input: &str) {
		self.diagnostic_translations.push(TagDiagnosticTranslation {
			src: input.into(),
			indexed: None,
			offset_in_parent: 0,
			tag_pos_in_parent: 0,
			indent: 0,
		});
	}

	/// parse the input
	///
	/// # Errors
	///
	/// if any error diagnostics are emitted, the resulting [`AST`] may be incomplete
	pub fn parse(&mut self, input: &str) -> Result<AST, AST> {
		let ok = self.ok;

		self.ok = true;
		let ast = parser::parse(parser::Ctx::new(self, input));
		self.ok = ok;

		ast
	}

	pub fn emit(&mut self, ast: &mut AST, to: To) -> bool {
		let ok = self.ok;

		self.ok = true;
		for node in ast {
			node.emit(self, to, true);
		}
		self.ok = ok;

		self.ok
	}

	pub fn finish(&mut self) -> Vec<Diagnostic> {
		self.ok = true;
		self.diagnostic_translations.clear();
		core::mem::take(&mut self.diagnostics)
	}

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
			} else {
				/*
				let line = 'line: {
					let mut start = 0;

					for (i, line) in trans.src.lines().enumerate() {
						start += line.len() + 1;

						if start >= at {
							break 'line i;
						}
					}

					0
				};

				trans.offset_in_parent + line * trans.indent + at
				*/

				if let Some(indexed) = &trans.indexed {
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
				}
			};

			i -= 1;
		}

		self.diagnostics.push(Diagnostic {
			err,
			at,
			code,
			#[cfg(debug_assertions)]
			src: Location::caller(),
		});
	}
}

impl Default for MarkDoll {
	fn default() -> Self {
		Self::new()
	}
}
