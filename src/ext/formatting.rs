use {
	crate::{
		args,
		emit::HtmlEmit,
		ext::TagDefinition,
		tree::{TagContent, AST},
		MarkDoll,
	},
	::alloc::format,
	alloc::{boxed::Box, string::String},
};

/// `em` tag
///
/// add emphasis to content
///
/// # flags
///
/// can be combined for multiple emphasis
///
/// - i\
///   italics via `<em>`\
///   **default if no flags are specified**
/// - b\
///   bold via `<strong>`
/// - u\
///   underline via `<u>`
/// - s\
///   strikethrough via `<s>`
/// - h\
///   highlight via `<mark>`
/// - q\
///   quote via `<q>`
///
/// # content
///
/// markdoll
pub mod emphasis {
	use super::*;

	/// holds content and flags for each formatting method
	#[derive(Debug)]
	#[allow(clippy::struct_excessive_bools, reason = "required")]
	pub struct Emphasis {
		/// whether the content should be italicized
		pub italic: bool,
		/// whether the content should be bolded
		pub bold: bool,
		/// whether the content should be underlined
		pub underline: bool,
		/// whether the content should be struck out
		pub strikethrough: bool,
		/// whether the content should be highlighted
		pub highlight: bool,
		/// whether the content should be in quotes
		pub quote: bool,
		/// the content
		pub ast: AST,
	}

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"em",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args();
					opt_args();
					flags(i, b, u, s, h, q);
					props();
				};

				Some(Box::new(Emphasis {
					italic: i || (!b && !u && !s && !h && !q),
					bold: b,
					underline: u,
					strikethrough: s,
					highlight: h,
					quote: q,
					ast: match doll.parse(text) {
						Ok(ast) => ast,
						Err(ast) => {
							doll.ok = false;
							ast
						}
					},
				}))
			}),
		)
		.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(doll: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		let em = content.downcast_mut::<Emphasis>().unwrap();

		if em.italic {
			to.write.push_str("<em>");
		}

		if em.bold {
			to.write.push_str("<strong>");
		}

		if em.underline {
			to.write.push_str("<u>");
		}

		if em.strikethrough {
			to.write.push_str("<s>");
		}

		if em.highlight {
			to.write.push_str("<mark>");
		}

		if em.quote {
			to.write.push_str("<q>");
		}

		for item in &mut em.ast {
			item.emit(doll, to);
		}

		if em.quote {
			to.write.push_str("</q>");
		}

		if em.highlight {
			to.write.push_str("</mark>");
		}

		if em.strikethrough {
			to.write.push_str("</s>");
		}

		if em.underline {
			to.write.push_str("</u>");
		}

		if em.bold {
			to.write.push_str("</strong>");
		}

		if em.italic {
			to.write.push_str("</em>");
		}
	}
}

/// `quote` tag
///
/// insert a block quote
///
/// # arguments
///
/// - `cite` (optional)\
///   the citation to use
///
/// # content
///
/// markdoll
pub mod quote {
	use super::*;

	/// the content of the quote, and optionally a citation
	#[derive(Debug)]
	struct Quote {
		pub cite: Option<String>,
		pub ast: AST,
	}

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"quote",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args();
					opt_args(cite: String);
					flags();
					props();
				}

				Some(Box::new(Quote {
					cite,
					ast: match doll.parse(text) {
						Ok(ast) => ast,
						Err(ast) => {
							doll.ok = false;
							ast
						}
					},
				}))
			}),
		)
		.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(doll: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		let quote = content.downcast_mut::<Quote>().unwrap();

		to.write.push_str("<figure class='doll-quote'>");

		if let Some(cite) = &quote.cite {
			to.write.push_str(&format!(
				"<figcaption>{}</figcaption>",
				&html_escape::encode_text(cite)
			));
		}

		to.write.push_str("<blockquote>");

		for item in &mut quote.ast {
			item.emit(doll, to);
		}

		to.write.push_str("</blockquote></figure>");
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> [TagDefinition; 2] {
	[emphasis::tag(), quote::tag()]
}
