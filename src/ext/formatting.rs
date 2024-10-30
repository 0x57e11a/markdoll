use {
	crate::{args, ext::TagDefinition, tree::AST},
	alloc::{boxed::Box, string::String},
};

#[derive(Debug)]
#[allow(clippy::struct_excessive_bools, reason = "required")]
struct Emphasis {
	pub italic: bool,
	pub bold: bool,
	pub underline: bool,
	pub strikethrough: bool,
	pub highlight: bool,
	pub quote: bool,
	pub ast: AST,
}

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
pub const EMPHASIS_TAG: TagDefinition = TagDefinition {
	key: "em",
	parse: Some(|doll, mut args, text| {
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
	emit: |doll, to, content| {
		let em = content.downcast_mut::<Emphasis>().unwrap();

		if em.italic {
			to.write_str("<em>").unwrap();
		}

		if em.bold {
			to.write_str("<strong>").unwrap();
		}

		if em.underline {
			to.write_str("<u>").unwrap();
		}

		if em.strikethrough {
			to.write_str("<s>").unwrap();
		}

		if em.highlight {
			to.write_str("<mark>").unwrap();
		}

		if em.quote {
			to.write_str("<q>").unwrap();
		}

		let block = em.ast.len() > 1;
		for item in &mut em.ast {
			item.emit(doll, to, block);
		}

		if em.quote {
			to.write_str("</q>").unwrap();
		}

		if em.highlight {
			to.write_str("</mark>").unwrap();
		}

		if em.strikethrough {
			to.write_str("</s>").unwrap();
		}

		if em.underline {
			to.write_str("</u>").unwrap();
		}

		if em.bold {
			to.write_str("</strong>").unwrap();
		}

		if em.italic {
			to.write_str("</em>").unwrap();
		}
	},
};

#[derive(Debug)]
struct Quote {
	pub cite: Option<String>,
	pub ast: AST,
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
pub const QUOTE_TAG: TagDefinition = TagDefinition {
	key: "quote",
	parse: Some(|doll, mut args, text| {
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
	emit: |doll, to, content| {
		let quote = content.downcast_mut::<Quote>().unwrap();

		to.write_str("<figure class='doll-quote'>").unwrap();

		if let Some(cite) = &quote.cite {
			write!(
				to,
				"<figcaption>{}</figcaption>",
				&html_escape::encode_text(cite)
			)
			.unwrap();
		}

		to.write_str("<blockquote>").unwrap();

		let block = quote.ast.len() > 1;
		for item in &mut quote.ast {
			item.emit(doll, to, block);
		}

		to.write_str("</blockquote></figure>").unwrap();
	},
};

/// all of this module's tags
pub const TAGS: &[TagDefinition] = &[EMPHASIS_TAG, QUOTE_TAG];
