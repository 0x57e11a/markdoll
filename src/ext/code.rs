use {
	crate::{args, emit::HtmlEmit, ext::TagDefinition, tree::TagContent, MarkDoll},
	::alloc::format,
	alloc::{
		boxed::Box,
		string::{String, ToString},
	},
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
		TagDefinition::new("code", Some(|_, _, text| Some(Box::new(text.to_string()))))
			.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(_: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		to.write.push_str(&format!(
			"<code>{}</code>",
			content.downcast_ref::<String>().unwrap()
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
		pub lang: Option<String>,
		/// the text
		pub text: String,
	}

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"codeblock",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args();
					opt_args(lang: String);
					flags();
					props();
				};

				Some(Box::new(Block {
					lang,
					text: text.to_string(),
				}))
			}),
		)
		.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(doll: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		let code = content.downcast_ref::<Block>().unwrap();

		if let Some(lang) = &code.lang {
			(to.code_block_format.clone())(doll, to, lang, &code.text);
		} else {
			to.write.push_str(&format!(
				"<div class='doll-code-block'><pre>{}</pre></div>",
				&html_escape::encode_text(&code.text)
			));
		}
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> [TagDefinition; 2] {
	[code::tag(), codeblock::tag()]
}
