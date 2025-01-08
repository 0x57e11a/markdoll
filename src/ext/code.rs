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
	pub fn tag() -> TagDefinition {
		TagDefinition {
			key: "code",
			parse: |_, _, text, _| Some(Box::new(Span::from(text))),
			emitters: Emitters::<TagEmitter>::new().with(html),
		}
	}

	/// emit to html
	pub fn html(
		doll: &mut MarkDoll,
		to: &mut HtmlEmit,
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
/// # arguments
///
/// - (optional) `lang`:\
///   the language code to highlight, modify [`doll.builtin_emitters.code_block`](crate::emit::BuiltInEmitters::code_block) to define behavior
///
/// # content
///
/// anything
pub mod codeblock {
	use super::*;

	/// represents the language and content
	#[derive(Debug)]
	pub struct Block {
		/// the language
		pub lang: Option<Span>,
		/// the text
		pub text: Span,
	}

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition {
			key: "codeblock",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					args();
					opt_args(lang);
					flags();
					props(waw, wawa: usize);
				};

				Some(Box::new(Block {
					lang: lang.map(Into::into),
					text: text.into(),
				}))
			},
			emitters: Emitters::<TagEmitter>::new().with(html),
		}
	}

	/// emit to html
	pub fn html(
		doll: &mut MarkDoll,
		to: &mut HtmlEmit,
		content: &mut Box<dyn TagContent>,
		_: Span,
	) {
		let code = content.downcast_ref::<Block>().unwrap();

		to.write.push_str(&format!(
			"<div class='doll-code-block'><pre>{}</pre></div>",
			&html_escape::encode_text(&*doll.spanner.lookup_span(code.text))
		));
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> impl IntoIterator<Item = TagDefinition> {
	[code::tag(), codeblock::tag()]
}
