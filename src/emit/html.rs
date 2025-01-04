use {
	crate::{
		emit::BuiltInEmitters,
		tree::{InlineItem, AST},
		MarkDoll, MarkDollSrc,
	},
	::spanner::{Spanned, SrcSpan},
	::std::rc::Rc,
};

/// emit a code block with a given language
///
/// - `doll` - markdoll instance
/// - `emit` - emit target
/// - `lang` - language requested
/// - `src` - content
pub type CodeBlockFormatter =
	dyn Fn(&mut MarkDoll, &mut HtmlEmit, SrcSpan<MarkDollSrc>, SrcSpan<MarkDollSrc>);

/// emit to HTML
pub struct HtmlEmit {
	/// HTML buffer
	pub write: String,
	/// heading level, initialize this to 0
	pub section_level: usize,
	/// emit a code block with a given language
	///
	/// - `doll` - markdoll instance
	/// - `emit` - emit target
	/// - `lang` - language requested
	/// - `src` - content
	pub code_block_format: Rc<CodeBlockFormatter>,
}

impl HtmlEmit {
	pub const DEFAULT_EMITTERS: BuiltInEmitters<HtmlEmit> = {
		fn inline(
			doll: &mut MarkDoll,
			to: &mut HtmlEmit,
			segments: &mut [Spanned<InlineItem>],
			inline_block: bool,
		) {
			if inline_block {
				to.write.push_str("<div>");
			}

			for Spanned(_, segment) in segments {
				match segment {
					InlineItem::Split => to.write.push(' '),
					InlineItem::Break => to.write.push_str("<br />"),
					InlineItem::Text(text) => {
						to.write.push_str(&html_escape::encode_text(text));
					}
					InlineItem::Tag(tag) => tag.emit(doll, to),
				}
			}

			if inline_block {
				to.write.push_str("</div>");
			}
		}

		fn section(
			doll: &mut MarkDoll,
			to: &mut HtmlEmit,
			header: &mut [Spanned<InlineItem>],
			children: &mut AST,
		) {
			to.section_level += 1;

			let level = to.section_level;
			if level <= 6 {
				to.write
					.push_str(&format!("<section data-level='{level}'><h{level}>"));

				inline(doll, to, header, false);

				to.write.push_str(&format!("</h{level}><div>"));
			} else {
				to.write.push_str(&format!(
					"<section data-level='{level}'><div role='heading' aria-level='{level}'>",
				));

				inline(doll, to, header, false);

				to.write.push_str(&format!("</div><div>",));
			}

			let inline_block = children.len() > 1;
			for Spanned(_, child) in children {
				child.emit(doll, &mut *to, inline_block);
			}

			to.write.push_str("</div></section>");

			to.section_level -= 1;
		}

		fn list(doll: &mut MarkDoll, to: &mut HtmlEmit, ordered: bool, items: &mut [AST]) {
			let kind = if ordered { "ol" } else { "ul" };
			to.write.push_str(&format!("<{kind}>"));

			for item in items {
				to.write.push_str("<li>");

				let inline_block = item.len() > 1;
				for Spanned(_, child) in item {
					child.emit(doll, &mut *to, inline_block);
				}

				to.write.push_str("</li>");
			}

			to.write.push_str(&format!("</{kind}>"));
		}

		BuiltInEmitters {
			inline,
			section,
			list,
		}
	};
}

impl ::core::fmt::Debug for HtmlEmit {
	fn fmt(&self, fmt: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
		fmt.debug_struct("HtmlEmit")
			.field("write", &self.write)
			.field("section_level", &self.section_level)
			.finish()
	}
}

impl From<HtmlEmit> for String {
	fn from(html: HtmlEmit) -> String {
		html.write
	}
}
