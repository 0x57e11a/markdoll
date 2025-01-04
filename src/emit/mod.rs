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
pub struct BuiltInEmitters<To> {
	/// how to emit [`BlockItem::Inline`](crate::tree::BlockItem::Inline)
	pub inline: fn(
		doll: &mut MarkDoll,
		to: &mut To,
		segments: &mut [Spanned<InlineItem>],
		inline_block: bool,
	),
	/// how to emit [`BlockItem::Section`](crate::tree::BlockItem::Section)
	pub section: fn(
		doll: &mut MarkDoll,
		to: &mut To,
		header: &mut [Spanned<InlineItem>],
		children: &mut AST,
	),
	/// how to emit [`BlockItem::List`](crate::tree::BlockItem::List)
	pub list: fn(doll: &mut MarkDoll, to: &mut To, ordered: bool, items: &mut [AST]),
}

impl<T> Clone for BuiltInEmitters<T> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<T> Copy for BuiltInEmitters<T> {}

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
		if !self.0.is_empty() {
			write!(fmt, "tag supports emission to:\n{}", self.0.join("\n"))
		} else {
			write!(fmt, "this tag cannot be emitted to any targets")
		}
	}
}
