use {
	crate::{args, ext::TagDefinition, tree::AST},
	alloc::{
		boxed::Box,
		string::{String, ToString},
	},
};

#[derive(Debug)]
struct Link {
	pub href: String,
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
pub const LINK_TAG: TagDefinition = TagDefinition {
	key: "link",
	parse: Some(|doll, mut args, text| {
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
	emit: |doll, to, content| {
		let link = content.downcast_mut::<Link>().unwrap();

		write!(to, "<a href='{}'>", &html_escape::encode_safe(&link.href)).unwrap();

		let block = link.ast.len() > 1;
		for item in &mut link.ast {
			item.emit(doll, to, block);
		}

		to.write_str("</a>").unwrap();
	},
};

#[derive(Debug)]
struct Image {
	pub src: String,
	pub alt: String,
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
pub const IMG_TAG: TagDefinition = TagDefinition {
	key: "img",
	parse: Some(|doll, mut args, text| {
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
	emit: |_, to, content| {
		let img = content.downcast_mut::<Image>().unwrap();

		write!(
			to,
			"<img src='{}' alt='{}' />",
			&html_escape::encode_safe(&img.src),
			&html_escape::encode_safe(&img.alt)
		)
		.unwrap();
	},
};

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
/// defines the `ref-<id>` HTML id, replacing `<id>` with the `id` argument
pub const DEF_TAG: TagDefinition = TagDefinition {
	key: "def",
	parse: Some(|doll, mut args, text| {
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
	emit: |doll, to, content| {
		let link = content.downcast_mut::<Link>().unwrap();

		let href = &html_escape::encode_safe(&link.href);
		write!(to, "<div class='doll-ref' id='ref-{href}'>[{href}]: ").unwrap();

		let block = link.ast.len() > 1;
		for item in &mut link.ast {
			item.emit(doll, to, block);
		}

		to.write_str("</div>").unwrap();
	},
};

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
/// links to the `ref-<id>` HTML id, replacing `<id>` with the `id` argument
pub const REF_TAG: TagDefinition = TagDefinition {
	key: "ref",
	parse: Some(|doll, mut args, text| {
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
	emit: |_, to, content| {
		let href = content.downcast_ref::<String>().unwrap();

		write!(to, "<sup><a href='#ref-{href}'>[{href}]</a></sup>").unwrap();
	},
};

/// all of this module's tags
pub const TAGS: &[TagDefinition] = &[LINK_TAG, IMG_TAG, DEF_TAG, REF_TAG];
