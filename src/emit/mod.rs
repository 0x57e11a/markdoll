use {
	crate::{
		tree::{InlineItem, AST},
		MarkDoll,
	},
	::alloc::{format, string::String},
	::hashbrown::HashMap,
};

/// emit to HTML
pub struct HtmlEmit {
	/// HTML buffer
	pub write: String,
	/// heading level, initialize this to 0
	pub section_level: usize,
	/// defines how code block languages should be emitted
	pub code_block_format: HashMap<&'static str, fn(doll: &mut MarkDoll, to: &mut HtmlEmit, text: &str)>,
}

/// defines the behavior of built in [`BlockItem`](crate::tree::BlockItem)s
#[derive(Debug)]
pub struct BuiltInEmitters<To> {
	/// how to emit [`BlockItem::Inline`](crate::tree::BlockItem::Inline)
	pub inline: fn(doll: &mut MarkDoll, to: &mut To, segments: &mut [(usize, InlineItem)]),
	/// how to emit [`BlockItem::Section`](crate::tree::BlockItem::Section)
	pub section: fn(doll: &mut MarkDoll, to: &mut To, name: &str, children: &mut AST),
	/// how to emit [`BlockItem::List`](crate::tree::BlockItem::List)
	pub list: fn(doll: &mut MarkDoll, to: &mut To, ordered: bool, items: &mut [AST]),
}

impl BuiltInEmitters<HtmlEmit> {
	/// the default [`BlockItem::Inline`](crate::tree::BlockItem::Inline) emitter
	pub fn default_inline(
		doll: &mut MarkDoll,
		to: &mut HtmlEmit,
		segments: &mut [(usize, InlineItem)],
	) {
		to.write.push_str("<div>");

		for (_, segment) in segments {
			match segment {
				InlineItem::Split => to.write.push(' '),
				InlineItem::Break => to.write.push_str("<br />"),
				InlineItem::Text(text) => {
					to.write.push_str(&html_escape::encode_text(text));
				}
				InlineItem::Tag(tag) => tag.emit(doll, to),
			}
		}

		to.write.push_str("</div>");
	}

	/// the default [`BlockItem::Section`](crate::tree::BlockItem::Section) emitter
	pub fn default_section(
		doll: &mut MarkDoll,
		to: &mut HtmlEmit,
		name: &str,
		children: &mut AST,
	) {
		to.section_level += 1;

		let level = to.section_level;
		if level <= 6 {
			to.write.push_str(&format!(
				"<section data-level='{level}'><h{level}>{}</h{level}><div>",
				&html_escape::encode_text(name)
			));
		} else {
			to.write.push_str(&format!(
				
				"<section data-level='{level}'><div role='heading' aria-level='{level}'>{}</div><div>",
				&html_escape::encode_text(name)
			)
			);
		}

		for child in children {
			child.emit(doll, &mut *to);
		}

		to.write.push_str("</div></section>");

		to.section_level -= 1;
	}

	/// the default [`BlockItem::List`](crate::tree::BlockItem::List) emitter
	pub fn default_list(doll: &mut MarkDoll, to: &mut HtmlEmit, ordered: bool, items: &mut [AST]) {
		let kind = if ordered { "ol" } else { "ul" };
		to.write.push_str(&format!( "<{kind}>"));

		for item in items {
			to.write.push_str("<li>");

			for child in item {
				child.emit(doll, &mut *to);
			}

			to.write.push_str("</li>");
		}

		to.write.push_str(&format!( "</{kind}>"));
	}
}

impl<T> Clone for BuiltInEmitters<T> {
	fn clone(&self) -> Self { *self }
}

impl<T> Copy for BuiltInEmitters<T> {}

impl Default for BuiltInEmitters<HtmlEmit> {
	fn default() -> Self {
		Self {
			inline: Self::default_inline,
			section: Self::default_section,
			list: Self::default_list,
		}
	}
}
