use {
	crate::{
		tree::{InlineItem, AST},
		MarkDoll,
	},
	alloc::string::String,
	hashbrown::HashMap,
};

pub type To<'a> = &'a mut dyn core::fmt::Write;

#[derive(Debug)]
pub struct BuiltInEmitters {
	pub inline:
		fn(doll: &mut MarkDoll, to: To, segments: &mut [(usize, InlineItem)], inline_block: bool),
	pub section: fn(doll: &mut MarkDoll, to: To, level: usize, name: &str, children: &mut AST),
	pub list: fn(doll: &mut MarkDoll, to: To, ordered: bool, items: &mut [AST]),
	pub code_block: HashMap<String, fn(doll: &mut MarkDoll, to: To, text: &str)>,
}

impl BuiltInEmitters {
	/// the default [`BlockItem::Inline`](crate::tree::BlockItem::Inline) emitter
	///
	/// # Panics
	///
	/// if it could not write to the writer
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
	///
	/// # Panics
	///
	/// if it could not write to the writer
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
				"<section data-level='{level}'><h{level}>{}</h{level}>",
				&html_escape::encode_text(name)
			)
			.unwrap();
		} else {
			write!(
				to,
				"<section data-level='{level}'><div role='heading' aria-level='{level}'>{}</div>",
				&html_escape::encode_text(name)
			)
			.unwrap();
		}

		let block = children.len() > 1;
		for child in children {
			child.emit(doll, to, block);
		}

		to.write_str("</section>").unwrap();
	}

	/// the default [`BlockItem::List`](crate::tree::BlockItem::List) emitter
	///
	/// # Panics
	///
	/// if it could not write to the writer
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
			code_block: HashMap::new(),
			inline: Self::default_inline,
			section: Self::default_section,
			list: Self::default_list,
		}
	}
}
