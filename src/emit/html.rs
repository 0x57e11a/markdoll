//! the html emit target

use {
	crate::{emit::BuiltInEmitters, tree::InlineItem},
	::core::fmt::Write,
	::spanner::Spanned,
};

/// emit to HTML
#[derive(Debug, Default)]
pub struct HtmlEmit {
	/// HTML buffer
	pub write: String,
	/// heading level, initialize this to 0
	pub section_level: usize,
}

impl HtmlEmit {
	/// the default emitters for html
	#[must_use]
	pub const fn default_emitters<Ctx>() -> BuiltInEmitters<Ctx, Self> {
		BuiltInEmitters {
			inline: |doll, to, ctx, segments, inline_block| {
				if inline_block {
					to.write.push_str("<div class='doll-inline'>");
				}

				for Spanned(_, segment) in segments {
					match segment {
						InlineItem::Split => to.write.push(' '),
						InlineItem::Break => to.write.push_str("<br />"),
						InlineItem::Text(text) => {
							write!(to.write, "<span>{}</span>", &html_escape::encode_safe(text))
								.unwrap();
						}
						InlineItem::Tag(tag) => tag.emit(doll, to, ctx),
					}
				}

				if inline_block {
					to.write.push_str("</div>");
				}
			},
			section: |doll, to, ctx, header, children| {
				to.section_level += 1;

				let level = to.section_level;
				if level <= 6 {
					write!(to.write, "<h{level}>").unwrap();

					(doll.builtin_emitters.get().unwrap().inline)(doll, to, ctx, header, false);

					write!(to.write, "</h{level}>").unwrap();
				} else {
					write!(to.write, "<div role='heading' aria-level='{level}'>",).unwrap();

					(doll.builtin_emitters.get().unwrap().inline)(doll, to, ctx, header, false);

					to.write.push_str("</div>");
				}

				write!(
					to.write,
					"<section class='doll-section' data-level='{level}'>"
				)
				.unwrap();

				for Spanned(_, child) in children {
					child.emit(doll, to, ctx, true);
				}

				to.write.push_str("</section>");

				to.section_level -= 1;
			},
			list: |doll, to, ctx, ordered, items| {
				let kind = if ordered { "ol" } else { "ul" };
				write!(to.write, "<{kind}>").unwrap();

				for item in items {
					to.write.push_str("<li>");

					let inline_block = item.len() > 1;
					for Spanned(_, child) in item {
						child.emit(doll, to, ctx, inline_block);
					}

					to.write.push_str("</li>");
				}

				write!(to.write, "</{kind}>").unwrap();
			},
		}
	}
}

impl From<HtmlEmit> for String {
	fn from(html: HtmlEmit) -> String {
		html.write
	}
}
