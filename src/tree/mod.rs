pub(crate) mod parser;

use {
	crate::{emit::To, MarkDoll, TagDiagnosticTranslation},
	alloc::{boxed::Box, string::String, vec::Vec},
	downcast_rs::{impl_downcast, Downcast},
};

/// block syntax tree
pub type AST = Vec<BlockItem>;

/// tag content, effectively just [`Any`](core::any::Any) with [`Debug`](core::fmt::Debug)
pub trait TagContent: Downcast + core::fmt::Debug {}

impl<T: Downcast + core::fmt::Debug> TagContent for T {}

impl_downcast!(TagContent);

/// an invoked tag
#[derive(Debug)]
pub struct TagInvocation {
	/// the tag name
	pub tag: String,
	/// the arguments to the tag
	pub args: Vec<String>,
	/// the content returned by the tag
	pub content: Box<dyn TagContent>,
	pub(crate) diagnostic_translation: Option<TagDiagnosticTranslation>,
}

impl TagInvocation {
	/// emit into an output
	pub fn emit(&mut self, doll: &mut MarkDoll, to: To) {
		doll.diagnostic_translations
			.push(self.diagnostic_translation.take().unwrap());
		(doll
			.ext_system
			.tags
			.get(&*self.tag)
			.expect("tag not defined, this should've been handled by the parser")
			.emit)(doll, to, &mut self.content);
		self.diagnostic_translation = Some(doll.diagnostic_translations.pop().unwrap());
	}
}

/// an inline item, containing real content
#[derive(Debug)]
pub enum InlineItem {
	/// a line split, caused by a single unescaped newline
	Split,

	/// a line break, caused by an escaped newline
	Break,

	/// it's text.
	Text(String),

	/// a tag invocation
	Tag(TagInvocation),
}

/// a block item, containing structure or inline content
#[derive(Debug)]
pub enum BlockItem {
	/// inline items
	Inline(Vec<(usize, InlineItem)>),

	/// a section, containing a numerical level, heading content, and body
	Section {
		/// position of the & defining the section
		pos: usize,
		/// heading level, starts at 1
		level: usize,
		/// heading text
		name: String,
		/// content of the section
		children: AST,
	},

	/// an ordered or unordered list, containing several items
	List {
		/// position of the list start
		pos: usize,
		/// whether the list is ordered
		ordered: bool,
		/// the items
		items: Vec<AST>,
	},
}

impl BlockItem {
	/// emit into an output
	pub fn emit(&mut self, doll: &mut MarkDoll, to: To, inline_block: bool) {
		match self {
			Self::Inline(segments) => {
				(doll.builtin_emitters.inline)(doll, to, segments, inline_block);
			}
			Self::Section {
				level,
				name,
				children,
				..
			} => (doll.builtin_emitters.section)(doll, to, *level, name, children),
			Self::List { ordered, items, .. } => {
				(doll.builtin_emitters.list)(doll, to, *ordered, &mut items[..]);
			}
		}
	}
}
