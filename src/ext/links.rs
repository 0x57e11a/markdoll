use {
	crate::{
		args,
		emit::HtmlEmit,
		ext::TagDefinition,
		tree::{TagContent, AST},
		MarkDoll,
	},
	::alloc::format,
	alloc::{
		boxed::Box,
		string::{String, ToString},
	},
};

/// the link destination and visuals
#[derive(Debug)]
pub struct Link {
	/// the destination
	pub href: String,
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
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"link",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args(href: String);
					opt_args();
					flags();
					props();
				};

				Some(Box::new(Link {
					href,
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
		let link = content.downcast_mut::<Link>().unwrap();

		to.write.push_str(&format!(
			"<a href='{}'>",
			&html_escape::encode_safe(&link.href)
		));

		let inline_block = link.ast.len() > 1;
		for item in &mut link.ast {
			item.emit(doll, to, inline_block);
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
		pub src: String,
		pub alt: String,
	}

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"img",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args(src: String);
					opt_args();
					flags();
					props();
				};

				Some(Box::new(Image {
					src,
					alt: text.to_string(),
				}))
			}),
		)
		.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(_: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		let img = content.downcast_mut::<Image>().unwrap();

		to.write.push_str(&format!(
			"<img src='{}' alt='{}' />",
			&html_escape::encode_safe(&img.src),
			&html_escape::encode_safe(&img.alt)
		));
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
///
/// # implementation
///
/// when emitting to [`HtmlEmit`], defines the `ref-<id>` HTML id, replacing `<id>` with the `id` argument
pub mod definition {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"def",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args(href: String);
					opt_args();
					flags();
					props();
				};

				Some(Box::new(Link {
					href,
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
		let link = content.downcast_mut::<Link>().unwrap();

		let href = &html_escape::encode_safe(&link.href);
		to.write
			.push_str(&format!("<div class='doll-ref' id='ref-{href}'>[{href}]: "));

		let inline_block = link.ast.len() > 1;
		for item in &mut link.ast {
			item.emit(doll, to, inline_block);
		}

		to.write.push_str("</div>");
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
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"ref",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args(href: String);
					opt_args();
					flags();
					props();
				};

				if !text.is_empty() {
					doll.diag(true, usize::MAX, "cannot have content");
				}

				Some(Box::new(href))
			}),
		)
		.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(_: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		let href = content.downcast_ref::<String>().unwrap();

		to.write
			.push_str(&format!("<sup><a href='#ref-{href}'>[{href}]</a></sup>"));
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> [TagDefinition; 4] {
	[
		link::tag(),
		image::tag(),
		definition::tag(),
		reference::tag(),
	]
}
