//! `em`/`quote` tags

use {
	crate::{
		args,
		emit::html::HtmlEmit,
		ext::{Emitters, TagDefinition, TagEmitter},
		tree::{TagContent, AST},
		MarkDoll,
	},
	::spanner::{Span, Spanned},
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
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "em",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					flags(i, b, u, s, h, q);
				};

				Some(Box::new(Emphasis {
					italic: i || (!b && !u && !s && !h && !q),
					bold: b,
					underline: u,
					strikethrough: s,
					highlight: h,
					quote: q,
					ast: doll.parse_embedded(text.into()),
				}))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		ctx: &mut Ctx,
		content: &mut dyn TagContent,
		_: Span,
	) {
		let em = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Emphasis>()
			.unwrap();

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

		let inline_block = em.ast.len() > 1;
		for Spanned(_, item) in &mut em.ast {
			item.emit(doll, to, ctx, inline_block);
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
	pub struct Quote {
		/// citation for this quote
		pub cite: Option<AST>,
		/// content
		pub ast: AST,
	}

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "quote",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					opt_args(cite);
				}

				Some(Box::new(Quote {
					cite: cite.map(|cite| doll.parse_embedded(cite.into())),
					ast: doll.parse_embedded(text.into()),
				}))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		ctx: &mut Ctx,
		content: &mut dyn TagContent,
		_: Span,
	) {
		let quote = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Quote>()
			.unwrap();

		to.write.push_str("<figure class='doll-quote'>");

		if let Some(cite) = &mut quote.cite {
			to.write.push_str("<figcaption>");

			let inline_block = cite.len() > 1;
			for Spanned(_, item) in cite {
				item.emit(doll, to, ctx, inline_block);
			}

			to.write.push_str("</figcaption>");
		}

		to.write.push_str("<blockquote>");

		let inline_block = quote.ast.len() > 1;
		for Spanned(_, item) in &mut quote.ast {
			item.emit(doll, to, ctx, inline_block);
		}

		to.write.push_str("</blockquote></figure>");
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	[emphasis::tag::<Ctx>(), quote::tag::<Ctx>()]
}
