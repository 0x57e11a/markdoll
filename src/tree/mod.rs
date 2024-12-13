pub(crate) mod parser;

use crate::{emit::BuiltInEmitters, MarkDoll, TagDiagnosticTranslation};

/// block syntax tree
pub type AST = Vec<BlockItem>;

/// tag content, effectively just [`Any`](core::any::Any) with [`Debug`](core::fmt::Debug)
pub trait TagContent: ::downcast_rs::Downcast + ::core::fmt::Debug {}

impl<T: ::downcast_rs::Downcast + ::core::fmt::Debug> TagContent for T {}

::downcast_rs::impl_downcast!(TagContent);

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
	pub fn emit<To: 'static>(&mut self, doll: &mut MarkDoll, to: &mut To) {
		doll.diagnostic_translations
			.push(self.diagnostic_translation.take().unwrap());

		let def = doll
			.ext_system
			.tags
			.get(&*self.tag)
			.expect("tag not defined, this should've been handled by the parser");

		match def.emitter_for::<To>() {
			Some(emit) => emit(doll, to, &mut self.content),
			None => doll.diag(
				true,
				usize::MAX,
				if def.has_any_emitters() {
					"this tag does not support emitting for this emit target"
				} else {
					"this tag cannot be emitted"
				},
			),
		}

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
	pub fn emit<To: 'static>(&mut self, doll: &mut MarkDoll, to: &mut To, inline_block: bool) {
		let builtin_emitters = doll
			.builtin_emitters
			.get_ref::<BuiltInEmitters<To>>()
			.expect("no BuiltInEmitters defined for this emit target");

		match self {
			Self::Inline(segments) => {
				(builtin_emitters.inline)(doll, to, segments, inline_block);
			}
			Self::Section { name, children, .. } => {
				(builtin_emitters.section)(doll, to, name, children);
			}
			Self::List { ordered, items, .. } => {
				(builtin_emitters.list)(doll, to, *ordered, &mut items[..]);
			}
		}
	}
}
