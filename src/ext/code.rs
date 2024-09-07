use {
	crate::{args, ext::TagDefinition},
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
pub const CODE_TAG: TagDefinition = TagDefinition {
	key: "code",
	parse: Some(|_, _, text| Some(Box::new(text.to_string()))),
	emit: |_, to, content| {
		write!(
			to,
			"<code>{}</code>",
			content.downcast_ref::<String>().unwrap()
		)
		.unwrap();
	},
};

#[derive(Debug)]
struct Block {
	pub lang: Option<String>,
	pub text: String,
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
pub const CODEBLOCK_TAG: TagDefinition = TagDefinition {
	key: "codeblock",
	parse: Some(|doll, mut args, text| {
		args! {
			doll, args;

			args();
			opt_args(lang: String);
			flags(i, b, u, s, h, q);
			props();
		};

		Some(Box::new(Block {
			lang,
			text: text.to_string(),
		}))
	}),
	emit: |doll, to, content| {
		let code = content.downcast_ref::<Block>().unwrap();

		if let Some(lang) = &code.lang {
			write!(
				to,
				"<div class='doll-code-block' data-lang='{}'><pre>",
				&html_escape::encode_safe(&lang)
			)
			.unwrap();

			if let Some(emitter) = doll.code_block.get(lang) {
				(emitter)(doll, to, &code.text);
			} else {
				write!(
					to,
					"<div class='doll-code-block'><pre>{}</pre></div>",
					&html_escape::encode_text(&code.text)
				)
				.unwrap();
				//doll.diag(true, usize::MAX, "language does not exist");
			}

			to.write_str("</pre></div>").unwrap();
		} else {
			write!(
				to,
				"<div class='doll-code-block'><pre>{}</pre></div>",
				&html_escape::encode_text(&code.text)
			)
			.unwrap();
		}
	},
};

/// all of this module's tags
pub const TAGS: &[TagDefinition] = &[CODE_TAG, CODEBLOCK_TAG];
