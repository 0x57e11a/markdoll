//! defines the base for emitting to various targets

use {
	crate::{
		tree::{InlineItem, AST},
		MarkDoll,
	},
	::core::fmt::Display,
	::miette::{LabeledSpan, SourceSpan},
	::spanner::Spanned,
};

pub mod html;

/// how to emit [`BlockItem::Inline`](crate::tree::BlockItem::Inline)
pub type InlineEmitter<Ctx, To> = fn(
	doll: &mut MarkDoll<Ctx>,
	to: &mut To,
	ctx: &mut Ctx,
	segments: &mut [Spanned<InlineItem>],
	inline_block: bool,
);

/// how to emit [`BlockItem::Section`](crate::tree::BlockItem::Section)
pub type SectionEmitter<Ctx, To> = fn(
	doll: &mut MarkDoll<Ctx>,
	to: &mut To,
	ctx: &mut Ctx,
	header: &mut [Spanned<InlineItem>],
	children: &mut AST,
);

/// how to emit [`BlockItem::List`](crate::tree::BlockItem::List)
pub type ListEmitter<Ctx, To> = fn(
	doll: &mut MarkDoll<Ctx>,
	to: &mut To,
	ctx: &mut Ctx,
	is_ordered: bool,
	list_items: &mut [AST],
);

/// defines the behavior of built in [`BlockItem`](crate::tree::BlockItem)s
#[derive(Debug)]
pub struct BuiltInEmitters<Ctx, To = ()> {
	/// how to emit [`BlockItem::Inline`](crate::tree::BlockItem::Inline)
	pub inline: InlineEmitter<Ctx, To>,
	/// how to emit [`BlockItem::Section`](crate::tree::BlockItem::Section)
	pub section: SectionEmitter<Ctx, To>,
	/// how to emit [`BlockItem::List`](crate::tree::BlockItem::List)
	pub list: ListEmitter<Ctx, To>,
}

impl<Ctx, To> Clone for BuiltInEmitters<Ctx, To> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<Ctx, To> Copy for BuiltInEmitters<Ctx, To> {}

/// diagnostics emitted while emitting
#[derive(::thiserror::Error, ::miette::Diagnostic, Debug)]
pub enum EmitDiagnostic {
	/// tag cannot be emitted to this target
	#[error("tag cannot be emitted to this target")]
	#[diagnostic(code(markdoll::emit::tag_cannot_emit_to))]
	TagCannotEmitTo {
		/// failed emit target
		bad: &'static str,
		/// targets this tag supports
		#[help]
		acceptable: AcceptableTagEmitTargets,

		/// the tag
		#[label]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
}

/// helper to display what types this tag can emit to
#[derive(Debug, Clone)]
pub struct AcceptableTagEmitTargets(pub Vec<&'static str>);

impl Display for AcceptableTagEmitTargets {
	fn fmt(&self, fmt: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		if self.0.is_empty() {
			write!(fmt, "this tag cannot be emitted to any targets")
		} else {
			write!(fmt, "tag supports emission to:\n{}", self.0.join("\n"))
		}
	}
}
