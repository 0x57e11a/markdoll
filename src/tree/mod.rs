pub mod parser;

use {
	crate::{emit::To, MarkDoll, TagDiagnosticTranslation},
	alloc::{boxed::Box, string::String, vec::Vec},
	downcast_rs::{impl_downcast, Downcast},
};

pub type AST = Vec<BlockItem>;

pub trait TagContent: Downcast + core::fmt::Debug {}

impl<T: Downcast + core::fmt::Debug> TagContent for T {}

impl_downcast!(TagContent);

#[derive(Debug)]
pub struct TagInvocation {
	pub tag: String,
	pub args: Vec<String>,
	pub content: Box<dyn TagContent>,
	pub diagnostic_translation: Option<TagDiagnosticTranslation>,
}

impl TagInvocation {
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

#[derive(Debug)]
pub enum InlineItem {
	/// a line split
	Split,

	/// a line break
	Break,

	/// it's text.
	Text(String),

	/// a tag invocation
	Tag(TagInvocation),
}

#[derive(Debug)]
pub enum BlockItem {
	/// inline items
	Inline(Vec<(usize, InlineItem)>),

	/// a section, containing a numerical level, heading content, and body
	Section {
		pos: usize,
		level: usize,
		name: String,
		children: AST,
	},

	/// an ordered or unordered list, containing several items
	List {
		pos: usize,
		ordered: bool,
		items: Vec<AST>,
	},
}

impl BlockItem {
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
