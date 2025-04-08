use {
	crate::{
		args,
		emit::html::HtmlEmit,
		ext::{Emitters, TagDefinition, TagEmitter},
		tree::TagContent,
		MarkDoll,
	},
	::spanner::Span,
};

/// `code` tag
///
/// generates inline code
///
/// # content
///
/// anything
pub mod code {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "code",
			parse: |_, _, text, _| Some(Box::new(Span::from(text))),
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		_: &mut Ctx,
		content: &mut Box<dyn TagContent>,
		_: Span,
	) {
		to.write.push_str(&format!(
			"<code>{}</code>",
			doll.spanner
				.lookup_span(*content.downcast_ref::<Span>().unwrap())
		));
	}
}

/// `codeblock` tag
///
/// emits code blocks
///
/// # content
///
/// anything
pub mod codeblock {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "codeblock",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;
				};

				Some(Box::new(text))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		_: &mut Ctx,
		content: &mut Box<dyn TagContent>,
		_: Span,
	) {
		let code = content.downcast_ref::<Span>().unwrap();

		to.write.push_str(&format!(
			"<div class='doll-code-block'><pre>{}</pre></div>",
			&html_escape::encode_safe(&*doll.spanner.lookup_span(*code))
		));
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	[code::tag::<Ctx>(), codeblock::tag::<Ctx>()]
}
