//! syntax trees and parsing

pub(crate) mod parser;

use {
	crate::{
		diagnostics::DiagnosticKind,
		emit::{AcceptableTagEmitTargets, EmitDiagnostic},
		MarkDoll,
	},
	::spanner::{Span, Spanned},
};

/// tag content, effectively just [`Any`](core::any::Any) with [`Debug`](core::fmt::Debug)
pub trait TagContent: ::core::any::Any + ::core::fmt::Debug {}

impl<T: ::core::any::Any + ::core::fmt::Debug> TagContent for T {}

/// an invoked tag
#[derive(Debug)]
pub struct TagInvocation {
	/// the tag name
	pub name: Span,
	/// the content returned by the tag
	pub content: Box<dyn TagContent>,
}

impl TagInvocation {
	/// emit into an output
	pub fn emit<Ctx, To: 'static>(&mut self, doll: &mut MarkDoll<Ctx>, to: &mut To, ctx: &mut Ctx) {
		let def = doll
			.tags
			.get(&*doll.spanner.lookup_span(self.name))
			.expect("tag not defined, this should've been handled by the parser");

		if let Some(emit) = def.emitters.get::<To>() {
			emit(doll, to, ctx, &mut *self.content, self.name);
		} else {
			let acceptable = AcceptableTagEmitTargets(def.emitters.type_names().collect());
			let (at, context) = doll.resolve_span(self.name);
			doll.diag(DiagnosticKind::Emit(EmitDiagnostic::TagCannotEmitTo {
				at,
				context,
				bad: ::core::any::type_name::<To>(),
				acceptable,
			}));
		}
	}
}

/// block syntax tree
pub type AST = Vec<Spanned<BlockItem>>;

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
	Inline(Vec<Spanned<InlineItem>>),

	/// a section, heading content and body
	Section {
		/// heading text
		header: Vec<Spanned<InlineItem>>,
		/// content of the section
		children: AST,
	},

	/// an ordered or unordered list, containing several items
	List {
		/// whether the list is ordered
		ordered: bool,
		/// the items
		items: Vec<AST>,
	},
}

impl BlockItem {
	/// emit into an output
	pub fn emit<Ctx, To: 'static>(
		&mut self,
		doll: &mut MarkDoll<Ctx>,
		to: &mut To,
		ctx: &mut Ctx,
		inline_block: bool,
	) {
		let builtin_emitters = doll
			.builtin_emitters
			.get::<To>()
			.expect("no BuiltInEmitters defined for this emit target");

		match self {
			Self::Inline(segments) => {
				(builtin_emitters.inline)(doll, to, ctx, segments, inline_block);
			}
			Self::Section {
				header: name,
				children,
				..
			} => {
				(builtin_emitters.section)(doll, to, ctx, name, children);
			}
			Self::List { ordered, items, .. } => {
				(builtin_emitters.list)(doll, to, ctx, *ordered, &mut items[..]);
			}
		}
	}
}
