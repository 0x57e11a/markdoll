use {
	crate::{
		tree::{BlockItem, InlineItem, TagContent, TagInvocation, AST},
		MarkDoll, TagDiagnosticTranslation,
	},
	alloc::{
		boxed::Box,
		rc::Rc,
		string::{String, ToString},
		vec::Vec,
	},
	log::error,
};

#[rustfmt::skip] // doing this so rust-analyzer doesnt merge it into the above import, making it invalid. see https://github.com/rust-lang/rust-analyzer/issues/17317
use alloc::vec;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndentKind {
	Standard,
	UnorderedList,
	OrderedList,
}

#[derive(Debug)]
enum StackPart {
	Root {
		children: AST,
	},

	List {
		pos: usize,
		ordered: bool,
		items: Vec<AST>,
	},
	Section {
		pos: usize,
		level: usize,
		name: String,
		children: AST,
	},
	TagBlockContent {
		tag: String,
		args: Vec<String>,
		text: String,
		tag_at: usize,
		offset_in_parent: usize,
		indent: usize,
	},
}

impl StackPart {
	#[must_use]
	pub fn unterminated(&self) -> &'static str {
		match self {
			Self::Root { .. } => "unterminated root",

			Self::List { .. } => "unterminated list",
			Self::Section { .. } => "unterminated section",
			Self::TagBlockContent { .. } => "unterminated tag block",
		}
	}

	#[must_use]
	pub fn can_gracefully_terminate(&self) -> bool {
		!matches!(self, Self::TagBlockContent { .. })
	}
}

#[derive(Debug)]
struct Stream {
	pub src: Vec<char>,
	pub index: usize,
}

impl Stream {
	#[allow(clippy::should_implement_trait, reason = "not bothering")]
	pub fn next(&mut self) -> Option<char> {
		if self.index < self.src.len() {
			self.index += 1;
			Some(self.src[self.index - 1])
		} else {
			None
		}
	}

	pub fn skip(&mut self) {
		self.index += 1;
	}

	pub fn back(&mut self) {
		self.index -= 1;
	}

	pub fn try_eat(&mut self, ch: char) -> bool {
		if self.lookahead(1) == Some(ch) {
			self.skip();

			true
		} else {
			false
		}
	}

	pub fn lookahead(&mut self, n: usize) -> Option<char> {
		self.src.get(self.index + n - 1).copied()
	}
}

#[derive(Debug)]
pub(crate) struct Ctx<'doll> {
	doll: &'doll mut MarkDoll,
	stream: Stream,
	stack: Vec<StackPart>,
	inline: Vec<(usize, InlineItem)>,
	warned_cr: bool,
}

impl<'doll> Ctx<'doll> {
	pub fn new(doll: &'doll mut MarkDoll, src: &str) -> Self {
		Self {
			doll,
			stream: Stream {
				src: src.chars().collect(),
				index: 0,
			},
			stack: {
				let mut stack = Vec::with_capacity(8);
				stack.push(StackPart::Root {
					children: Vec::new(),
				});
				stack
			},
			inline: Vec::new(),
			warned_cr: false,
		}
	}

	pub fn stack_terminate_top(&mut self) {
		if self.stack.is_empty() {
			return;
		}

		t!("terminating", self.stack.last());

		match self.stack.pop().expect("empty parse stack") {
			StackPart::Root { .. } => {
				unreachable!("attempt to terminate root")
			}
			StackPart::List {
				pos,
				ordered,
				items,
			} => {
				self.stack_push_block_to_top(BlockItem::List {
					pos,
					ordered,
					items,
				});
			}
			StackPart::Section {
				pos,
				level,
				name,
				children,
			} => {
				self.stack_push_block_to_top(BlockItem::Section {
					pos,
					level,
					name,
					children,
				});
			}
			StackPart::TagBlockContent {
				tag,
				args,
				mut text,
				tag_at,
				offset_in_parent,
				indent,
			} => {
				if text.ends_with('\n') {
					text.pop().unwrap();
				}

				let text = Rc::from(text);

				self.doll
					.diagnostic_translations
					.push(TagDiagnosticTranslation {
						src: Rc::clone(&text),
						indexed: None,
						offset_in_parent,
						tag_pos_in_parent: tag_at,
						indent,
					});

				if let Some(content) = tag::transform_content(self, &args, &text, &tag) {
					self.inline.push((
						tag_at,
						InlineItem::Tag(TagInvocation {
							tag,
							args,
							content,
							diagnostic_translation: Some(
								self.doll.diagnostic_translations.pop().unwrap(),
							),
						}),
					));
				} else {
					self.doll.diagnostic_translations.pop().unwrap();
				}
			}
		}
	}

	pub fn stack_push_block_to_top(&mut self, item: BlockItem) {
		match self.stack.last_mut().expect("empty parse stack") {
			StackPart::Root { children } | StackPart::Section { children, .. } => {
				children.push(item);
			}
			StackPart::List { items, .. } => items
				.last_mut()
				.expect("list does not have any items")
				.push(item),
			StackPart::TagBlockContent { .. } => {
				unreachable!("attempt to push block item into tag block-content")
			}
		}
	}

	pub fn flush_inline(&mut self) {
		if self.inline.is_empty() {
			t!("[[[NOT flushing inline items]]]");
			return;
		}

		t!("[[[flushing inline items]]]");
		t!(&self.inline);
		t!(&self.stack.last());

		if let (_, InlineItem::Split | InlineItem::Break) = self.inline.last().unwrap() {
			self.inline.pop().unwrap();
		}

		match self.stack.last_mut().unwrap() {
			StackPart::Root { children } | StackPart::Section { children, .. } => {
				children.push(BlockItem::Inline(core::mem::take(&mut self.inline)));
			}
			StackPart::List { items, .. } => items
				.last_mut()
				.unwrap()
				.push(BlockItem::Inline(core::mem::take(&mut self.inline))),
			StackPart::TagBlockContent { .. } => {
				error!("attempt to push onto tagblockcontent");
			}
		}
	}

	#[track_caller]
	pub fn err(&mut self, msg: &'static str) {
		self.doll.diag(true, self.stream.index - 1, msg);
	}

	#[track_caller]
	pub fn err_next(&mut self, msg: &'static str) {
		self.doll.diag(true, self.stream.index, msg);
	}

	/// `:neocat_floof_explode:`
	#[track_caller]
	pub fn warn_cr(&mut self) {
		if !self.warned_cr {
			self.doll.diag(false, 0, "markdoll does not support CRLF");
		}
	}

	#[must_use]
	pub fn find_parent_indent(&self) -> usize {
		let mut indent = 1;

		for TagDiagnosticTranslation { indent: parent, .. } in
			self.doll.diagnostic_translations.iter().rev()
		{
			indent += parent;
			indent = indent.saturating_sub(1);
		}

		indent
	}

	pub fn eat_until_newline(&mut self) -> bool {
		loop {
			match self.stream.next() {
				Some('\n') => return true,
				None => {
					self.err("unexpected EOI");
					return false;
				}
				_ => {}
			}
		}
	}

	pub fn eat_all(&mut self, desired: char) -> usize {
		let mut n = 0;

		loop {
			match self.stream.lookahead(1) {
				Some(ch) if ch == desired => {
					self.stream.skip();
					n += 1;
				}
				_ => break n,
			}
		}
	}
}

enum ParseResult<T = ()> {
	Ok(T),
	NextLine,
	Stop,
}

mod indent {
	use super::*;

	/// called before returning to normal parsing
	fn exit(ctx: &mut Ctx, indent_level: &mut usize) -> bool {
		if let n @ 1.. = ctx.eat_all(' ') {
			ctx.doll
				.diag(false, ctx.stream.index - n, "erroneous leading spaces");
		}

		// if parsing a block tag
		if let Some(StackPart::TagBlockContent { tag_at, .. }) = ctx.stack.last() {
			// and there's a closing bracket below its content indent
			if *indent_level + 2 <= ctx.stack.len() && ctx.stream.try_eat(']') {
				// if the indent isnt exactly one level below its content indent
				if *indent_level + 2 < ctx.stack.len() {
					ctx.doll
						.diag(true, *tag_at, "misaligned closing tag for this tag");
				}

				// terminate it
				ctx.stack_terminate_top();

				return false;
			}
		}

		// squish down to the indent level
		squimsh_to(ctx, *indent_level);

		true
	}

	/// squish stack parts down to the provided indent level
	///
	/// `:pinched_hand::neocat_melt:`
	fn squimsh_to(ctx: &mut Ctx, to: usize) {
		t!("[[[indent drop]]]", to);

		while ctx.stack.len() > to + 1 {
			let top = ctx.stack.last().unwrap();
			if top.can_gracefully_terminate() {
				t!("[[[terminate gracefully]]]");

				ctx.flush_inline();
				ctx.stack_terminate_top();
			} else {
				t!("[[[terminate non-gracefully]]]");

				ctx.err_next(top.unterminated());

				// forcibly terminate it anyways
				ctx.stack_terminate_top();
			}
		}
	}

	fn more(ctx: &mut Ctx, kind: IndentKind, indent_level: usize) {
		ctx.flush_inline();

		// more indent than the stack
		if kind == IndentKind::Standard {
			// cant come from nowhere
			ctx.err("unexpected indentation");
			ctx.stack.push(StackPart::Section {
				pos: ctx.stream.index - 1,
				level: indent_level + ctx.find_parent_indent(),
				name: "<invalid indentation>".to_string(),
				children: Vec::new(),
			});
		} else {
			t!("[[[new list]]]");
			ctx.stack.push(StackPart::List {
				pos: ctx.stream.index - 2,
				ordered: kind == IndentKind::OrderedList,
				items: vec![Vec::new()],
			});
		}
	}

	fn less(ctx: &mut Ctx, kind: IndentKind, indent_level: usize, last_significant: &mut bool) {
		match (&mut ctx.stack[indent_level], kind) {
			// new line in the same list element
			(
				StackPart::List { .. }
				| StackPart::Section { .. }
				| StackPart::TagBlockContent { .. },
				IndentKind::Standard,
			) => {
				t!("[[[new line in current]]]");
			}
			(
				StackPart::List { ordered, .. },
				IndentKind::OrderedList | IndentKind::UnorderedList,
			) => {
				t!("[[[new list symbol]]]");
				let new_ordered = kind == IndentKind::OrderedList;

				if new_ordered == *ordered && *last_significant {
					t!("[[[new list item]]]");
					t!("inline", &ctx.inline);

					ctx.flush_inline();
					squimsh_to(ctx, indent_level);

					let StackPart::List { items, .. } = &mut ctx.stack[indent_level] else {
						unreachable!()
					};
					items.push(Vec::new());
				} else {
					t!("[[[new list, flush/term]]]");
					ctx.flush_inline();
					squimsh_to(ctx, indent_level - 1);
					ctx.stack.push(StackPart::List {
						pos: ctx.stream.index - 2,
						ordered: new_ordered,
						items: vec![Vec::new()],
					});
				}
			}
			(StackPart::Section { .. }, IndentKind::OrderedList | IndentKind::UnorderedList) => {
				t!("[[[section end]]]");
				squimsh_to(ctx, indent_level - 1);
				ctx.stack.push(StackPart::List {
					pos: ctx.stream.index - 2,
					ordered: kind == IndentKind::OrderedList,
					items: vec![Vec::new()],
				});
			}
			(
				StackPart::TagBlockContent { .. },
				IndentKind::OrderedList | IndentKind::UnorderedList,
			) => {
				t!("[[[new list in section]]]");
				ctx.err("unexpected list (expected indent)");
			}
			_ => unreachable!(),
		}
	}

	pub fn parse(ctx: &mut Ctx, last_significant: &mut bool) -> ParseResult<usize> {
		let mut indent_level = 0;

		let tag_block_top = matches!(ctx.stack.last().unwrap(), StackPart::TagBlockContent { .. });

		'indent: loop {
			if let Some(StackPart::TagBlockContent { .. }) = ctx.stack.get(indent_level) {
				// tags handle parsing their own content, so cease parsing indents when getting past their indentation
				break 'indent;
			}

			match ctx.stream.lookahead(1) {
				Some('\n') => {
					//ctx.stream.advance();

					if !tag_block_top {
						*last_significant = false;

						t!("[[[flush insignificant]]");
						ctx.flush_inline();
					}

					break 'indent;
				}

				// handle indentation
				Some(ch @ ('\t' | '=' | '-')) => {
					// if not just plain indent, need to eat the indent after it (or dont eat anything if no indent after it)
					if ch != '\t' {
						if !matches!(ctx.stream.lookahead(2), Some('\t' | '\n')) {
							if !exit(ctx, &mut indent_level) {
								return ParseResult::NextLine;
							}

							break 'indent;
						}

						ctx.stream.skip();
					}

					ctx.stream.skip();
					indent_level += 1;

					let kind = match ch {
						'\t' => IndentKind::Standard,
						'=' => IndentKind::OrderedList,
						'-' => IndentKind::UnorderedList,
						_ => unreachable!(),
					};

					t!("indent", kind);

					if tag_block_top {
						t!("[[[tag block top]]]");

						match kind {
							IndentKind::Standard => {}
							IndentKind::OrderedList | IndentKind::UnorderedList => {
								ctx.err("cannot start list item mid-tag");
							}
						}
					} else if indent_level + 1 > ctx.stack.len() {
						t!("[[[higher indent]]]");

						more(ctx, kind, indent_level);
					} else {
						t!("[[[lower indent]]]");

						less(ctx, kind, indent_level, last_significant);
					}
				}

				Some(ch) => {
					if ch == '\r' {
						ctx.warn_cr();
					}

					if !exit(ctx, &mut indent_level) {
						return ParseResult::NextLine;
					}

					*last_significant = true;

					break 'indent;
				}

				None => return ParseResult::Stop,
			}
		}

		ParseResult::Ok(indent_level)
	}
}

mod tag {
	use super::*;

	/// parse inline tag text
	fn parse_inline_text(ctx: &mut Ctx) -> Option<Rc<str>> {
		let mut text = String::with_capacity(16);
		let mut stack: usize = 0;

		loop {
			match ctx.stream.next() {
				Some('\n') => {
					ctx.err("unexpected newline");
					break;
				}

				Some('\t') => {
					ctx.err("unexpected indentation");
				}

				Some('\\') => match ctx.stream.next() {
					Some('\n') => {
						ctx.err("cannot escape newline in this context");
						break;
					}

					Some('\t') => {
						ctx.err("cannot escape indentation in this context");
					}

					Some(ch) => {
						if ch == '\r' {
							ctx.warn_cr();
						}
						text.push(ch);
					}

					None => {
						ctx.err("unexpected EOI");
						return None;
					}
				},

				Some('[') => {
					text.push('[');
					stack += 1;
				}
				Some(']') => {
					if stack > 0 {
						text.push(']');
						stack -= 1;
					} else {
						break;
					}
				}

				Some(ch) => {
					if ch == '\r' {
						ctx.warn_cr();
					}
					text.push(ch);
				}

				None => {
					ctx.err("unexpected EOI");
					return None;
				}
			}
		}

		Some(Rc::from(text))
	}

	/// transform tag text to actual content
	pub fn transform_content(
		ctx: &mut Ctx,
		args: &[String],
		text: &str,
		tag: &String,
	) -> Option<Box<dyn TagContent>> {
		if let Some(def) = ctx.doll.ext_system.tags.get(&**tag) {
			if let Some(parse) = def.parse {
				(parse)(
					ctx.doll,
					args.iter().map(|string| &**string).collect(),
					text,
				)
			} else {
				ctx.doll
					.diag(true, usize::MAX, "tag does not support content");
				None
			}
		} else {
			ctx.doll.diag(true, usize::MAX, "tag not defined");
			None
		}
	}

	fn parse_arg(ctx: &mut Ctx) -> ParseResult<String> {
		let mut arg = String::new();

		'arg: loop {
			match ctx.stream.next() {
				Some('\n') => {
					ctx.err("unexpected newline");
					break 'arg;
				}

				Some('\t') => {
					ctx.err("unexpected indentation");
				}

				Some(')') => break 'arg,

				Some('\\') => match ctx.stream.next() {
					Some('\n') => {
						ctx.err("cannot escape newline in this context");
						break 'arg;
					}

					Some('\t') => ctx.err("cannot escape indentation in this context"),

					Some(ch) => {
						if ch == '\r' {
							ctx.warn_cr();
						}
						arg.push(ch);
					}

					None => {
						ctx.err("unexpected EOI");
						return ParseResult::Stop;
					}
				},

				Some(ch) => {
					if ch == '\r' {
						ctx.warn_cr();
					}
					arg.push(ch);
				}

				None => {
					ctx.err("unexpected EOI");
					return ParseResult::Stop;
				}
			}
		}

		ParseResult::Ok(arg)
	}

	fn parse_content(
		ctx: &mut Ctx,
		tag: String,
		args: Vec<String>,
		start: usize,
		indent_level: usize,
	) -> ParseResult {
		match ctx.stream.lookahead(1) {
			Some('\n') => {
				ctx.stream.skip();

				ctx.err("unexpected newline");

				return ParseResult::Ok(());
			}

			Some(':') => {
				ctx.stream.skip();

				ctx.stack.push(StackPart::TagBlockContent {
					tag,
					args,
					text: String::new(),
					tag_at: start,
					offset_in_parent: ctx.stream.index + 1,
					indent: indent_level + 1,
				});

				match ctx.stream.next() {
					Some('\n') => {}

					// not a newline
					Some(ch) => {
						if ch == '\r' {
							ctx.warn_cr();
						}

						ctx.err("expected newline");

						// eat characters until newline
						if !ctx.eat_until_newline() {
							ctx.err("unexpected EOI");
							return ParseResult::Stop;
						}
					}

					None => {
						ctx.err("unexpected EOI");
						return ParseResult::Stop;
					}
				}

				return ParseResult::NextLine;
			}

			Some(ch) => {
				if ch == '\r' {
					ctx.warn_cr();
				}

				let offset_in_parent = ctx.stream.index;

				if let Some(text) = parse_inline_text(ctx) {
					ctx.doll
						.diagnostic_translations
						.push(TagDiagnosticTranslation {
							src: Rc::clone(&text),
							indexed: None,
							offset_in_parent,
							tag_pos_in_parent: start,
							indent: 0,
						});

					if let Some(content) = transform_content(ctx, &args, &text, &tag) {
						ctx.inline.push((
							start,
							InlineItem::Tag(TagInvocation {
								tag,
								args,
								content,
								diagnostic_translation: Some(
									ctx.doll.diagnostic_translations.pop().unwrap(),
								),
							}),
						));
					} else {
						ctx.doll.diagnostic_translations.pop().unwrap();
					}
				}
			}

			None => {
				ctx.err("unexpected EOI");
				return ParseResult::Stop;
			}
		}

		ParseResult::Ok(())
	}

	pub fn parse(ctx: &mut Ctx, indent_level: usize) -> ParseResult {
		let start = ctx.stream.index;
		let mut tag = String::with_capacity(16);
		let mut args = Vec::new();

		'tag: loop {
			match ctx.stream.next() {
				Some('\n') => {
					ctx.err("unexpected newline");
					break 'tag;
				}

				Some('\t') => {
					ctx.err("unexpected indentation");
				}

				Some('(') => match parse_arg(ctx) {
					ParseResult::Ok(arg) => args.push(arg),
					ParseResult::NextLine => return ParseResult::NextLine,
					ParseResult::Stop => return ParseResult::Stop,
				},

				Some(':') => match parse_content(ctx, tag, args, start, indent_level) {
					ParseResult::Ok(()) => break 'tag,
					ParseResult::NextLine => return ParseResult::NextLine,
					ParseResult::Stop => return ParseResult::Stop,
				},

				Some(']') => {
					ctx.doll
						.diagnostic_translations
						.push(TagDiagnosticTranslation {
							src: Rc::default(),
							indexed: None,
							offset_in_parent: ctx.stream.index - 1,
							tag_pos_in_parent: start,
							indent: 0,
						});

					if let Some(content) = transform_content(ctx, &args, "", &tag) {
						ctx.inline.push((
							start,
							InlineItem::Tag(TagInvocation {
								tag,
								args,
								content,
								diagnostic_translation: Some(
									ctx.doll.diagnostic_translations.pop().unwrap(),
								),
							}),
						));
					} else {
						ctx.doll.diagnostic_translations.pop().unwrap();
					}
					break 'tag;
				}

				Some(ch) => {
					if ch == '\r' {
						ctx.warn_cr();
					}
					tag.push(ch);
				}

				None => {
					ctx.err("unexpected EOI");
					return ParseResult::Stop;
				}
			}
		}

		ParseResult::Ok(())
	}
}

/// parse input
///
/// # Errors
///
/// if any error diagnostics are emitted
#[allow(clippy::too_many_lines, reason = "its not that big")]
pub(crate) fn parse(mut ctx: Ctx) -> Result<AST, AST> {
	t!("---- begin parse ----");

	// significance tracks functionally-empty lines, splitting paragraphs at functionally-empty lines
	let mut last_significant = false;

	'main: loop {
		t!("---- new line ----");

		// parse indentation
		let indent_level: usize = match indent::parse(&mut ctx, &mut last_significant) {
			ParseResult::Ok(indent_level) => indent_level,
			ParseResult::NextLine => continue 'main,
			ParseResult::Stop => break 'main,
		};

		t!("line indentation", indent_level);

		// decide what to do based off of stack top
		match ctx.stack.last_mut().unwrap() {
			// normal
			StackPart::Root { .. } | StackPart::List { .. } | StackPart::Section { .. } => {
				// section heads must be the start of their line
				if ctx.stream.try_eat('&') {
					let start = ctx.stream.index - 1;

					t!("[[[flush section]]]");
					ctx.flush_inline();

					let mut name = String::with_capacity(16);

					'name: loop {
						match ctx.stream.next() {
							Some('\n') => break 'name,

							Some('\t') => {
								ctx.err("unexpected indentation");
							}

							Some(ch) => {
								if ch == '\r' {
									ctx.warn_cr();
								}
								name.push(ch);
							}

							None => break 'main,
						}
					}

					ctx.stack.push(StackPart::Section {
						pos: start,
						level: indent_level + ctx.find_parent_indent() + 1,
						name,
						children: Vec::new(),
					});

					continue 'main;
				}

				// line loop
				loop {
					match ctx.stream.next() {
						Some('\n') => continue 'main,

						Some('\t') => ctx.err("unexpected indentation"),

						// parse a tag invocation
						Some('[') => match tag::parse(&mut ctx, indent_level) {
							ParseResult::Ok(()) => {}
							ParseResult::NextLine => continue 'main,
							ParseResult::Stop => break 'main,
						},

						// start parsing text
						Some(ch) => {
							if ch == '\r' {
								ctx.warn_cr();
							}

							ctx.stream.back();

							let start = ctx.stream.index;

							let mut text = String::new();

							'text: loop {
								match ctx.stream.next() {
									Some('\n') => {
										ctx.inline.push((start, InlineItem::Text(text)));
										ctx.inline.push((ctx.stream.index, InlineItem::Split));

										continue 'main;
									}

									Some('\t') => ctx.err("unexpected indentation"),

									// escape sequence
									Some('\\') => match ctx.stream.next() {
										Some('\n') => {
											ctx.inline.push((start, InlineItem::Text(text)));
											ctx.inline.push((ctx.stream.index, InlineItem::Break));

											continue 'main;
										}

										Some('\t') => {
											ctx.err("cannot escape indentation in this context");
										}

										Some(ch) => {
											if ch == '\r' {
												ctx.warn_cr();
											}

											text.push(ch);
										}

										None => {
											ctx.err("unexpected EOI");
											ctx.inline.push((start, InlineItem::Text(text)));
											break 'main;
										}
									},

									// back up, let the tag parser handle this
									Some('[') => {
										ctx.inline.push((start, InlineItem::Text(text)));
										ctx.stream.back();
										break 'text;
									}

									Some(ch) => {
										if ch == '\r' {
											ctx.warn_cr();
										}
										text.push(ch);
									}

									None => {
										ctx.inline.push((start, InlineItem::Text(text)));
										break 'main;
									}
								}
							}
						}

						None => break 'main,
					}
				}
			}
			// special, just insert content straight into the tag
			StackPart::TagBlockContent { text: content, .. } => {
				let mut warn_cr = false;

				'line: loop {
					match ctx.stream.next() {
						Some('\n') => {
							content.push('\n');
							break 'line;
						}

						Some(ch) => {
							content.push(ch);

							if ch == '\r' {
								warn_cr = true;
							}
						}

						None => break 'main,
					}
				}

				if warn_cr {
					ctx.warn_cr();
				}
			}
		}
	}

	while ctx.stack.len() > 1 {
		let top = ctx.stack.last().unwrap();
		if top.can_gracefully_terminate() {
			t!("[[[flush/term gracefully]]]");
			ctx.flush_inline();
			ctx.stack_terminate_top();
		} else {
			ctx.err(top.unterminated());
			t!("[[[flush/term non-gracefully]]]");
			ctx.flush_inline();
			ctx.stack_terminate_top();
		}
	}

	ctx.flush_inline();

	let Some(StackPart::Root { children: ast }) = ctx.stack.pop() else {
		unreachable!()
	};

	t!("---- end parse ----");

	if ctx.doll.ok {
		Ok(ast)
	} else {
		Err(ast)
	}
}
