//! `link`/`image`/`anchor`/`def`/`ref` tags

use {
	crate::{
		args,
		emit::html::HtmlEmit,
		ext::{Emitters, TagDefinition, TagEmitter},
		tree::{TagContent, AST},
		MarkDoll,
	},
	::core::fmt::Write,
	::spanner::{Span, Spanned},
};

/// the link destination and visuals
#[derive(Debug)]
pub struct Link {
	/// the destination
	pub href: Span,
	/// the visuals
	pub ast: AST,
}

/// `link` tag
///
/// link to something
///
/// # arguments
///
/// - `href`\
///   the url to link to
///
/// # content
///
/// markdoll, used as the content of the link
pub mod link {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "link",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					args(href);
				};

				Some(Box::new(Link {
					href: href.into(),
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
		let link = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Link>()
			.unwrap();

		write!(
			to.write,
			"<a href='{}'>",
			&html_escape::encode_safe(&*doll.spanner.lookup_span(link.href))
		)
		.unwrap();

		let inline_block = link.ast.len() > 1;
		for Spanned(_, item) in &mut link.ast {
			item.emit(doll, to, ctx, inline_block);
		}

		to.write.push_str("</a>");
	}
}

/// `img` tag
///
/// insert an image
///
/// # arguments
///
/// - `src`\
///   the url to source the image from
///
/// # content
///
/// text, alt text of the image
pub mod image {
	use super::*;

	/// the image source and alt text
	#[derive(Debug)]
	struct Image {
		pub src: Span,
		pub alt: Span,
	}

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "img",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					args(src);
				};

				Some(Box::new(Image {
					src: src.into(),
					alt: text.into(),
				}))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		_: &mut Ctx,
		content: &mut dyn TagContent,
		_: Span,
	) {
		let img = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Image>()
			.unwrap();

		write!(
			to.write,
			"<img src='{}' alt='{}' />",
			&html_escape::encode_safe(&*doll.spanner.lookup_span(img.src)),
			&html_escape::encode_safe(&*doll.spanner.lookup_span(img.alt))
		)
		.unwrap();
	}
}

/// `anchor` tag
///
/// define an anchor to be used with the [`anchor`](REF_TAG) tag
///
/// # arguments
///
/// - `id`\
///   the id that `ref` tags should use
pub mod anchor {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "anchor",
			parse: |doll, args, _, tag_span| {
				args! {
					args;
					doll, tag_span;

					args(href);
				};

				Some(Box::new(Span::from(href)))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		_: &mut Ctx,
		content: &mut dyn TagContent,
		_: Span,
	) {
		let href = (content as &mut dyn ::core::any::Any)
			.downcast_ref::<Span>()
			.unwrap();

		write!(
			to.write,
			"<span class='doll-def' id='{href}'></span>",
			href = &html_escape::encode_safe(&*doll.spanner.lookup_span(*href))
		)
		.unwrap();
	}
}

/// `def` tag
///
/// define an anchor to be used with the [`ref`](REF_TAG) tag
///
/// # arguments
///
/// - `id`\
///   the id that `ref` tags should use
///
/// # content
///
/// markdoll
pub mod definition {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "def",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					args(href);
				};

				Some(Box::new(Link {
					href: href.into(),
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
		let link = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Link>()
			.unwrap();

		write!(
			to.write,
			"<span class='doll-def' id='{href}'><span class='doll-def-header'>[{href}]:</span>",
			href = &html_escape::encode_safe(&*doll.spanner.lookup_span(link.href))
		)
		.unwrap();

		let inline_block = link.ast.len() > 1;
		to.write.push_str(if inline_block {
			"<div class='doll-def-body'>"
		} else {
			" <span class='doll-def-body'>"
		});
		for Spanned(_, item) in &mut link.ast {
			item.emit(doll, to, ctx, inline_block);
		}
		to.write
			.push_str(if inline_block { "</div>" } else { "</span>" });

		to.write.push_str("</span>");
	}
}

/// `ref` tag
///
/// reference an anchor from a [`def`](DEF_TAG) tag
///
/// # arguments
///
/// - `id`\
///   the id that the corresponding `def` has
///
/// # implementation
///
/// when emitting to [`HtmlEmit`], links to the `ref-<id>` HTML id, replacing `<id>` with the `id` argument
pub mod reference {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "ref",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					args(href);
				};

				Some(Box::new(Link {
					href: href.into(),
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
		let link = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Link>()
			.unwrap();

		let id = doll.spanner.lookup_span(link.href);
		let id = ::html_escape::encode_safe(&*id);

		write!(to.write, "<sup class='doll-ref'><a href='#{id}'>[").unwrap();

		if link.ast.is_empty() {
			to.write.push_str(&id);
		} else {
			let inline_block = link.ast.len() > 1;
			for Spanned(_, item) in &mut link.ast {
				item.emit(doll, to, ctx, inline_block);
			}
		}

		to.write.push_str("]</a></sup>");
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	[
		link::tag::<Ctx>(),
		image::tag::<Ctx>(),
		definition::tag::<Ctx>(),
		anchor::tag::<Ctx>(),
		reference::tag::<Ctx>(),
	]
}
