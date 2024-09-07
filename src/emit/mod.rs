use crate::{
	tree::{InlineItem, AST},
	MarkDoll,
};

/// the output to emit to
pub type To<'a> = &'a mut dyn core::fmt::Write;

/// defines the behavior of built in [`BlockItem`](crate::tree::BlockItem)s
#[derive(Debug)]
pub struct BuiltInEmitters {
	/// how to emit [`BlockItem::Inline`](crate::tree::BlockItem::Inline)
	pub inline:
		fn(doll: &mut MarkDoll, to: To, segments: &mut [(usize, InlineItem)], inline_block: bool),
	/// how to emit [`BlockItem::Section`](crate::tree::BlockItem::Section)
	pub section: fn(doll: &mut MarkDoll, to: To, level: usize, name: &str, children: &mut AST),
	/// how to emit [`BlockItem::List`](crate::tree::BlockItem::List)
	pub list: fn(doll: &mut MarkDoll, to: To, ordered: bool, items: &mut [AST]),
}

impl BuiltInEmitters {
	/// the default [`BlockItem::Inline`](crate::tree::BlockItem::Inline) emitter
	pub fn default_inline(
		doll: &mut MarkDoll,
		to: To,
		segments: &mut [(usize, InlineItem)],
		inline_block: bool,
	) {
		if inline_block {
			to.write_str("<div>").unwrap();
		}

		for (_, segment) in segments {
			match segment {
				InlineItem::Split => to.write_char(' ').unwrap(),
				InlineItem::Break => to.write_str("<br />").unwrap(),
				InlineItem::Text(text) => {
					to.write_str(&html_escape::encode_text(text)).unwrap();
				}
				InlineItem::Tag(tag) => tag.emit(doll, to),
			}
		}

		if inline_block {
			to.write_str("</div>").unwrap();
		}
	}

	/// the default [`BlockItem::Section`](crate::tree::BlockItem::Section) emitter
	pub fn default_section(
		doll: &mut MarkDoll,
		to: To,
		level: usize,
		name: &str,
		children: &mut AST,
	) {
		if level <= 6 {
			write!(
				to,
				"<section data-level='{level}'><h{level}>{}</h{level}><div>",
				&html_escape::encode_text(name)
			)
			.unwrap();
		} else {
			write!(
				to,
				"<section data-level='{level}'><div role='heading' aria-level='{level}'>{}</div><div>",
				&html_escape::encode_text(name)
			)
			.unwrap();
		}

		let block = children.len() > 1;
		for child in children {
			child.emit(doll, to, block);
		}

		to.write_str("</div></section>").unwrap();
	}

	/// the default [`BlockItem::List`](crate::tree::BlockItem::List) emitter
	pub fn default_list(doll: &mut MarkDoll, to: To, ordered: bool, items: &mut [AST]) {
		let kind = if ordered { "ol" } else { "ul" };
		write!(to, "<{kind}>").unwrap();

		for item in items {
			to.write_str("<li>").unwrap();

			let block = item.len() > 1;
			for child in item {
				child.emit(doll, to, block);
			}

			to.write_str("</li>").unwrap();
		}

		write!(to, "</{kind}>").unwrap();
	}
}

impl Default for BuiltInEmitters {
	fn default() -> Self {
		Self {
			inline: Self::default_inline,
			section: Self::default_section,
			list: Self::default_list,
		}
	}
}
