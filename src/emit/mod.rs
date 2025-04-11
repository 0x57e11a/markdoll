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

/// defines the behavior of built in [`BlockItem`](crate::tree::BlockItem)s
#[derive(Debug)]
pub struct BuiltInEmitters<Ctx, To = ()> {
	/// how to emit [`BlockItem::Inline`](crate::tree::BlockItem::Inline)
	pub inline: fn(
		doll: &mut MarkDoll<Ctx>,
		to: &mut To,
		ctx: &mut Ctx,
		segments: &mut [Spanned<InlineItem>],
		inline_block: bool,
	),
	/// how to emit [`BlockItem::Section`](crate::tree::BlockItem::Section)
	pub section: fn(
		doll: &mut MarkDoll<Ctx>,
		to: &mut To,
		ctx: &mut Ctx,
		header: &mut [Spanned<InlineItem>],
		children: &mut AST,
	),
	/// how to emit [`BlockItem::List`](crate::tree::BlockItem::List)
	pub list:
		fn(doll: &mut MarkDoll<Ctx>, to: &mut To, ctx: &mut Ctx, ordered: bool, items: &mut [AST]),
}

impl<Ctx, To> Clone for BuiltInEmitters<Ctx, To> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<Ctx, To> Copy for BuiltInEmitters<Ctx, To> {}

#[derive(::thiserror::Error, ::miette::Diagnostic, Debug)]
pub enum EmitDiagnostic {
	#[error("tag cannot be emitted to this target")]
	#[diagnostic(code(markdoll::emit::tag_cannot_emit_to))]
	TagCannotEmitTo {
		#[label]
		at: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,

		bad: &'static str,
		#[help]
		acceptable: AcceptableTagEmitTargets,
	},
}

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
