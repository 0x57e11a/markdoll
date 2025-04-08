use {
	crate::{
		tree::{BlockItem, InlineItem, TagContent, TagInvocation, AST},
		MarkDoll, MarkDollSrc, SourceMetadata, TagDiagnosticTranslation,
	},
	::miette::{LabeledSpan, SourceSpan},
	::spanner::{Loc, Locd, Span, Spanned, SpannerExt, SrcSpan},
	::std::sync::Mutex,
	::tracing::{instrument, span::EnteredSpan, trace, trace_span, Level},
};

#[derive(::thiserror::Error, ::miette::Diagnostic, Debug)]
pub enum LangDiagnostic {
	// chars
	#[error("unexpected {unexpected}")]
	#[diagnostic(code(markdoll::lang::unexpected))]
	Unexpected {
		#[label("expected any of: {expected}", expected = expected.join(", "))]
		primary: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,
		unexpected: &'static str,
		expected: &'static [&'static str],
	},
	#[error("markdoll does not support carriage returns")]
	#[diagnostic(
		code(markdoll::lang::crlf_explode),
		help("consider using LF line endings, rather than CR/CRLF")
	)]
	CarriageReturn {
		#[label("carriage return here")]
		primary: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},

	// tag
	#[error("misaligned closing bracket in block tag")]
	#[diagnostic(
		code(markdoll::lang::tag::misaligned_closing_brace),
		help(
			"the closing bracket should be aligned on the same indentation as the opening bracket"
		)
	)]
	MisalignedClosingBracket {
		#[label("closing bracket")]
		primary: SourceSpan,
		#[label("opening bracket")]
		opened: SourceSpan,
		// no context, because it cant appear in tag line/argument
	},
	#[error("misaligned list")]
	#[diagnostic(
		code(markdoll::lang::tag::misaligned_list),
		help("try aligning this list to the correct indentation within the block tag")
	)]
	MisalignedList {
		#[label("cannot start list item in a block tag like this")]
		primary: SourceSpan,
		#[label("tag starts here")]
		tag: SourceSpan,
		// no context, because it cant appear in tag line/argument
	},
	#[error("misaligned content")]
	#[diagnostic(
		code(markdoll::lang::tag::misaligned_content),
		help("try aligning this content to the correct indentation within the block tag")
	)]
	MisalignedContent {
		#[label("this content is invalid")]
		primary: SourceSpan,
		#[label("tag starts here")]
		tag: SourceSpan,
		// no context, because it cant appear in tag line/argument
	},
	#[error("cannot escape {what}")]
	#[diagnostic(code(markdoll::lang::tag::cannot_escape))]
	CannotEscape {
		#[label]
		primary: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,
		what: &'static str,
	},
	#[error("cannot escape {what} in {here}")]
	#[diagnostic(code(markdoll::lang::tag::cannot_escape_here))]
	CannotEscapeHere {
		#[label]
		primary: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,
		what: &'static str,
		here: &'static str,
	},
	#[error("undefined tag")]
	#[diagnostic(code(markdoll::lang::tag::undefined_tag))]
	UndefinedTag {
		#[label("this tag is unknown to markdoll")]
		primary: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},

	// misc
	#[error("suspicious whitespace")]
	#[diagnostic(code(markdoll::lang::sus_spaces), severity(Warning))]
	SuspiciousWhitespace {
		#[label("these are spaces, which are not treated like indentation")]
		primary: SourceSpan,
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
}

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
		ordered: bool,
		items: Vec<AST>,
	},
	SectionHeader {
		header: Vec<Spanned<InlineItem>>,
	},
	Section {
		header: Vec<Spanned<InlineItem>>,
		children: AST,
	},
	BlockTag {
		name: Span,
		args: Vec<Span>,
		text: String,
		parent_span: Span,
		indent: usize,
	},
}

#[tyfling::debug("{src:?} at {}", *at - src.start())]
struct Stream {
	pub src: SrcSpan<MarkDollSrc>,
	pub at: Loc,
}

impl Stream {
	pub fn char_at(&self, i: u32) -> Option<char> {
		self.src[i as usize..].chars().next()
	}

	#[allow(clippy::should_implement_trait, reason = "not bothering")]
	pub fn next(&mut self) -> Option<char> {
		if self.at - self.src.start() < self.src.len() as u32 {
			let ch = self.lookahead(1);
			self.at += ch.unwrap().len_utf8() as u32;
			ch
		} else {
			None
		}
	}

	pub fn skip(&mut self) {
		self.at = self.lookahead_loc(2).unwrap();
	}

	pub fn back(&mut self) {
		self.at = self.lookahead_loc(0).unwrap();
	}

	pub fn try_eat(&mut self, ch: char) -> bool {
		if self.lookahead(1) == Some(ch) {
			self.skip();

			true
		} else {
			false
		}
	}

	pub fn eat_all(&mut self, desired: char) -> u32 {
		let mut n = 0;

		loop {
			match self.lookahead(1) {
				Some(ch) if ch == desired => {
					self.skip();
					n += 1;
				}
				_ => break n,
			}
		}
	}

	pub fn lookahead_loc(&self, mut n: i32) -> Option<Loc> {
		n -= 1;
		let mut offset = self.at - self.src.start();

		if n > 0 {
			for _ in 0..n {
				if offset >= self.src.len() as u32 {
					return None;
				}

				offset += 1;
				while offset < self.src.len() as u32 && !self.src.is_char_boundary(offset as usize)
				{
					offset += 1;
				}

				if !self.src.is_char_boundary(offset as usize) {
					return None;
				}
			}
		} else if n <= 0 {
			for _ in n..0 {
				if offset < 1 {
					return None;
				}

				offset -= 1;
				while offset > 0 && !self.src.is_char_boundary(offset as usize) {
					offset -= 1;
				}
			}
		}

		Some(self.src.start() + offset)
	}

	pub fn lookahead(&self, n: i32) -> Option<char> {
		self.char_at(self.lookahead_loc(n)? - self.src.start())
	}

	pub fn span_of(&mut self, i: Loc) -> Span {
		if let Some(ch) = self.char_at(i - self.src.start()) {
			Span::new(i, i + ch.len_utf8() as u32)
		} else {
			Span::new(i, i + 1)
		}
	}

	pub fn lookahead_span(&mut self, n: i32) -> Span {
		self.span_of(self.lookahead_loc(n).unwrap())
	}

	pub fn eof_span(&mut self) -> Span {
		(self.src.end() - 1).with_len(0)
	}

	pub fn tr(&mut self) -> StreamTransaction {
		StreamTransaction(self.at, trace_span!("transaction", ?self).entered())
	}

	#[allow(clippy::unused_self, reason = "consistency")]
	pub fn tr_commit(&mut self, _: StreamTransaction) {
		trace!("committed transaction");
	}

	pub fn tr_cancel(&mut self, trans: StreamTransaction) {
		trace!(returning_to = ?trans.0, "canceled transaction");
		self.at = trans.0;
	}
}

#[tyfling::debug("transaction{f0:?}")]
struct StreamTransaction(Loc, EnteredSpan);

#[tyfling::debug(
	"{stream:?}\n[{}] <- [{}]",
	stack.iter().map(|Locd(_, item)| match item {
		StackPart::Root { .. } => "root",
		StackPart::List { .. } => "list",
		StackPart::SectionHeader { .. } => "sectionheader",
		StackPart::Section { .. } => "section",
		StackPart::BlockTag { .. } => "tag-block-content",
	}).collect::<Vec<&'static str>>().join(", "),
	inline.iter().map(|Spanned(_, segment)| match segment {
		InlineItem::Split => "split",
		InlineItem::Break => "break",
		InlineItem::Text(_) => "text",
		InlineItem::Tag(_) => "tag",
	}).collect::<Vec<&'static str>>().join(", "),
)]
pub(crate) struct ParseCtx<'doll, Ctx> {
	doll: &'doll mut MarkDoll<Ctx>,
	stream: Stream,
	stack: Vec<Locd<StackPart>>,
	inline: Vec<Spanned<InlineItem>>,
}

impl<'doll, Ctx> ParseCtx<'doll, Ctx> {
	pub fn new(doll: &'doll mut MarkDoll<Ctx>, span: Span) -> Self {
		let src_span = doll.spanner.lookup_span(span);
		Self {
			doll,
			stream: Stream {
				src: src_span,
				at: span.start(),
			},
			stack: {
				let mut stack = Vec::with_capacity(8);
				stack.push(
					StackPart::Root {
						children: Vec::new(),
					}
					.locd(span.start()),
				);
				stack
			},
			inline: Vec::new(),
		}
	}

	pub fn stack_indent(&self) -> usize {
		let mut count = 0;

		for part in &self.stack {
			count += match part.1 {
				StackPart::Root { .. } => 0,
				StackPart::List { .. } => 1,
				StackPart::SectionHeader { .. } => 0,
				StackPart::Section { .. } => 1,
				StackPart::BlockTag { .. } => 1,
			};
		}

		count
	}

	#[instrument(name = "ctx.stack_terminate_top", level = Level::DEBUG)]
	#[track_caller]
	pub fn stack_terminate_top(&mut self) {
		if self.stack.is_empty() {
			return;
		}

		trace!(self.stack.last = ?self.stack.last());

		let Locd(start, top) = self.stack.pop().expect("empty parse stack");
		match top {
			StackPart::Root { .. } => {
				unreachable!("attempt to terminate root")
			}
			StackPart::List { ordered, items } => {
				self.stack_push_block_to_top(
					BlockItem::List { ordered, items }.spanned(Span::new(start, self.stream.at)),
				);
			}
			StackPart::SectionHeader { header } => {
				self.stack.push(
					StackPart::Section {
						header,
						children: Vec::new(),
					}
					.locd(start),
				);
			}
			StackPart::Section { header, children } => {
				self.stack_push_block_to_top(
					BlockItem::Section { header, children }
						.spanned(Span::new(start, self.stream.at)),
				);
			}
			StackPart::BlockTag {
				name,
				args,
				mut text,
				parent_span,
				indent,
			} => {
				if !text.is_empty() {
					assert_eq!(text.pop().unwrap(), '\n');
				}
				let file = self.doll.spanner.add(|start| MarkDollSrc {
					metadata: SourceMetadata::BlockTag {
						translation: TagDiagnosticTranslation {
							span: Span::new(start, start + text.len() as u32), // todo
							lines_to_parent_line_starts: Mutex::new(None),
							parent_span,
							parent_indent: indent,
						},
					},
					source: text,
				});

				if let Some(content) = tag::transform_content(self, name, &args, file.span()) {
					self.inline
						.push(InlineItem::Tag(TagInvocation { name, content }).spanned(name));
				}
			}
		}
	}

	#[instrument(name = "ctx.stack_push_block_to_top", level = Level::DEBUG)]
	pub fn stack_push_block_to_top(&mut self, item: Spanned<BlockItem>) {
		let Locd(_, top) = self.stack.last_mut().expect("empty parse stack");
		match top {
			StackPart::Root { children } => {
				children.push(item);
			}
			StackPart::SectionHeader { .. } => {
				unreachable!("attempt to push block item onto section header");
			}
			StackPart::Section { children, .. } => {
				children.push(item);
			}
			StackPart::List { items, .. } => items
				.last_mut()
				.expect("list does not have any items")
				.push(item),
			StackPart::BlockTag { .. } => {
				unreachable!("attempt to push block item onto block tag");
			}
		}
	}

	#[instrument(name = "ctx.flush_inline", level = Level::DEBUG)]
	#[track_caller]
	pub fn flush_inline(&mut self) {
		if self.inline.is_empty() {
			return;
		}

		if let Spanned(_, InlineItem::Split | InlineItem::Break) = self.inline.last().unwrap() {
			self.inline.pop().unwrap();
		}

		let mut inline = ::core::mem::take(&mut self.inline);
		let span = if !inline.is_empty() {
			inline.first().unwrap().0.union(&inline.last().unwrap().0)
		} else {
			return;
		};

		let Locd(_, top) = self.stack.last_mut().expect("empty parse stack");
		match top {
			StackPart::Root { children } => {
				children.push(BlockItem::Inline(inline).spanned(span));
			}
			StackPart::SectionHeader { header, .. } => {
				header.append(&mut inline);
			}
			StackPart::Section { children, .. } => {
				children.push(BlockItem::Inline(inline).spanned(span));
			}
			StackPart::List { items, .. } => items
				.last_mut()
				.unwrap()
				.push(BlockItem::Inline(inline).spanned(span)),
			StackPart::BlockTag { .. } => {
				unreachable!("attempt to flush inline onto block tag");
			}
		}
	}

	#[track_caller]
	pub fn diag(&mut self, diagnostic: LangDiagnostic) {
		self.doll.diag(diagnostic.into());
	}

	/// `:neocat_floof_explode:`
	#[instrument(level = Level::ERROR)]
	pub fn crlf_explode(&mut self) {
		let (primary, context) = self.doll.resolve_span(self.stream.lookahead_span(0));
		self.diag(LangDiagnostic::CarriageReturn {
			primary,
			context: context,
		});

		self.stream.at = self.stream.src.end();
	}

	#[instrument(name = "ctx.eat_until_newline", level = Level::TRACE, ret)]
	pub fn eat_until_newline(&mut self) -> bool {
		loop {
			match self.stream.next() {
				Some('\n') => return true,
				None => {
					let (primary, context) = self.doll.resolve_span(self.stream.at.with_len(0));
					self.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "end of input",
						expected: &["newline"],
					});
					return false;
				}
				_ => {}
			}
		}
	}
}

#[derive(Debug)]
enum ParseResult<T = ()> {
	Ok(T),
	NextLine,
	Stop,
}

#[instrument(name = "frontmatter", level = Level::DEBUG, ret)]
pub fn frontmatter<Ctx>(ctx: &mut ParseCtx<Ctx>) -> Option<String> {
	let tr = ctx.stream.tr();

	trace!("attempting to eat starting dashes");

	let n = ctx.stream.eat_all('-');
	if n != 3 {
		ctx.stream.tr_cancel(tr);

		trace!(found = n, "did not match");

		return None;
	}

	trace!("ate");

	if !ctx.stream.try_eat('\n') {
		ctx.stream.tr_cancel(tr);
		return None;
	}

	let mut frontmatter = String::new();

	loop {
		match ctx.stream.next() {
			Some('\n') => {
				if ctx.stream.eat_all('-') == 3 {
					ctx.stream.tr_commit(tr);

					match ctx.stream.next() {
						Some('\n') => {}

						Some(ch) => {
							if ch == '\r' {
								ctx.crlf_explode();
							}

							let (at, context) = ctx.doll.resolve_span(Span::new(
								ctx.stream.lookahead_loc(0).unwrap(),
								loop {
									match ctx.stream.next() {
										Some('\n') | None => {
											break ctx.stream.lookahead_loc(0).unwrap()
										}
										_ => {}
									}
								},
							));
							ctx.diag(LangDiagnostic::Unexpected {
								primary: at,
								context: context,
								unexpected: "chars",
								expected: &["newline"],
							});
						}

						None => {}
					}

					return Some(frontmatter);
				}

				frontmatter.push('\n');
			}
			Some(ch) => frontmatter.push(ch),
			None => {
				ctx.stream.tr_cancel(tr);
				return None;
			}
		}
	}
}

mod indent {
	use super::*;

	/// called before returning to normal parsing
	#[instrument(name = "indent::resume_standard_parsing", level = Level::DEBUG, ret)]
	fn resume_standard_parsing<Ctx>(ctx: &mut ParseCtx<Ctx>, indent_level: &mut usize) -> bool {
		let tr = ctx.stream.tr();
		let start = ctx.stream.at;
		if let 1.. = ctx.stream.eat_all(' ') {
			let (primary, context) = ctx
				.doll
				.resolve_span(Span::new(start, ctx.stream.lookahead_loc(1).unwrap()));
			ctx.diag(LangDiagnostic::SuspiciousWhitespace { primary, context });
		}
		ctx.stream.tr_cancel(tr);

		// if parsing a block tag
		if let Locd(start, StackPart::BlockTag { .. }) =
			ctx.stack.last().expect("empty parse stack")
		{
			let stack_indent = ctx.stack_indent();
			// and there's a closing bracket below its content indent
			if *indent_level + 1 <= stack_indent && ctx.stream.try_eat(']') {
				// if the indent isnt exactly one level below its content indent
				if *indent_level + 1 < stack_indent {
					let (opened, _) = ctx.doll.resolve_span(ctx.stream.span_of(*start));
					let (primary, _) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::MisalignedClosingBracket { opened, primary });
				}

				// terminate it
				ctx.stack_terminate_top();
			}
		}

		// squish down to the indent level
		squimsh_to(ctx, *indent_level);

		true
	}

	/// squish stack parts down to the provided indent level
	///
	/// `:pinched_hand::neocat_melt:`
	#[instrument(name = "indent::squimsh_to", level = Level::WARN)]
	fn squimsh_to<Ctx>(ctx: &mut ParseCtx<Ctx>, to: usize) {
		trace!(ctx.stack.len = ctx.stack.len());
		while ctx.stack_indent() > to {
			if let Locd(start, StackPart::BlockTag { .. }) = ctx.stack.last().unwrap() {
				let start = *start;
				let line_start = ctx.stream.lookahead_loc(1).unwrap();
				loop {
					match ctx.stream.lookahead(1) {
						Some('\n') => break,
						None => break,
						_ => ctx.stream.skip(),
					}
				}
				let (primary, _) = ctx.doll.resolve_span(Span::new(line_start, ctx.stream.at));
				let (tag, _) = ctx.doll.resolve_span(start.with_len(1));
				ctx.diag(LangDiagnostic::MisalignedContent { primary, tag });

				// forcibly terminate it anyways
				//ctx.stack_terminate_top();
				//ctx.flush_inline();

				return;
			} else {
				ctx.flush_inline();
				// gracefully terminate
				ctx.stack_terminate_top();
			}
		}
	}

	/// more indentation than current
	#[instrument(name = "indent::more", level = Level::DEBUG)]
	fn more<Ctx>(ctx: &mut ParseCtx<Ctx>, indent: Spanned<IndentKind>) {
		ctx.flush_inline();

		if indent.1 == IndentKind::Standard {
			if let Locd(_, StackPart::SectionHeader { .. }) = ctx.stack.last().unwrap() {
				ctx.stack_terminate_top();
			} else {
				// cant come from nowhere
				let (primary, context) = ctx.doll.resolve_span(indent.0);
				ctx.diag(LangDiagnostic::Unexpected {
					primary,
					context,
					unexpected: "indentation",
					expected: &["`&`"],
				});
				ctx.stack.push(
					StackPart::Section {
						header: vec![
							InlineItem::Text("<invalid indentation>".to_string()).spanned(indent.0)
						],
						children: Vec::new(),
					}
					.locd(indent.0.start()),
				);
			}
		} else {
			ctx.stack.push(
				StackPart::List {
					ordered: indent.1 == IndentKind::OrderedList,
					items: vec![Vec::new()],
				}
				.locd(indent.0.start()),
			);
		}
	}

	/// less indentation than current
	#[instrument(name = "indent::less", level = Level::DEBUG)]
	fn less<Ctx>(
		ctx: &mut ParseCtx<Ctx>,
		indent: Spanned<IndentKind>,
		indent_level: usize,
		last_significant: &mut bool,
	) {
		let Locd(_, stack_top) = &mut ctx.stack[indent_level];
		let Spanned(indent_span, indent_kind) = indent;
		match (stack_top, indent_kind) {
			(
				StackPart::List { .. } | StackPart::Section { .. } | StackPart::BlockTag { .. },
				IndentKind::Standard,
			) => {
				trace!("newline in list element");
			}
			(
				StackPart::List { ordered, .. },
				IndentKind::OrderedList | IndentKind::UnorderedList,
			) => {
				let new_ordered = indent_kind == IndentKind::OrderedList;

				if new_ordered == *ordered && *last_significant {
					trace!("new list item");

					ctx.flush_inline();
					squimsh_to(ctx, indent_level);

					let Locd(_, StackPart::List { items, .. }) = &mut ctx.stack[indent_level]
					else {
						unreachable!()
					};
					items.push(Vec::new());
				} else {
					trace!(kind_change = new_ordered == *ordered, "end list + new list");

					ctx.flush_inline();
					squimsh_to(ctx, indent_level - 1);
					ctx.stack.push(
						StackPart::List {
							ordered: new_ordered,
							items: vec![Vec::new()],
						}
						.locd(indent_span.start()),
					);
				}
			}
			(StackPart::Section { .. }, IndentKind::OrderedList | IndentKind::UnorderedList) => {
				trace!("end section via list");

				squimsh_to(ctx, indent_level - 1);
				ctx.stack.push(
					StackPart::List {
						ordered: indent_kind == IndentKind::OrderedList,
						items: vec![Vec::new()],
					}
					.locd(indent_span.start()),
				);
			}
			(StackPart::BlockTag { .. }, IndentKind::OrderedList | IndentKind::UnorderedList) => {
				unreachable!()
			}
			_ => unreachable!(),
		}
	}

	#[instrument(name = "indent::parse", level = Level::DEBUG, ret)]
	pub fn parse<Ctx>(ctx: &mut ParseCtx<Ctx>, last_significant: &mut bool) -> ParseResult<usize> {
		let mut indent_level = 0;

		let tag_block_top = if let Locd(loc, StackPart::BlockTag { .. }) = ctx.stack.last().unwrap()
		{
			Some(loc.with_len(1))
		} else {
			None
		};

		loop {
			if let Some(Locd(_, StackPart::BlockTag { .. })) = ctx.stack.get(indent_level) {
				// tags handle parsing their own content, so cease parsing indents when getting past their indentation
				break;
			}

			match ctx.stream.lookahead(1) {
				Some('\n') => {
					if !tag_block_top.is_some() {
						*last_significant = false;

						trace!("flush insignificant");
						ctx.flush_inline();
					}

					break;
				}

				// handle indentation
				Some(ch @ ('\t' | '=' | '-')) => {
					// if not just plain indent, need to eat the indent after it (or dont eat anything if no indent after it)
					if ch != '\t' {
						if !matches!(ctx.stream.lookahead(2), Some('\t' | '\n')) {
							if !resume_standard_parsing(ctx, &mut indent_level) {
								return ParseResult::NextLine;
							}

							break;
						}

						ctx.stream.skip();
					}

					ctx.stream.skip();
					indent_level += 1;

					let indent = match ch {
						'\t' => Spanned(
							ctx.stream.lookahead_loc(0).unwrap().with_len(1),
							IndentKind::Standard,
						),
						'=' => Spanned(
							ctx.stream.lookahead_loc(-1).unwrap().with_len(2),
							IndentKind::OrderedList,
						),
						'-' => Spanned(
							ctx.stream.lookahead_loc(-1).unwrap().with_len(2),
							IndentKind::UnorderedList,
						),
						_ => unreachable!(),
					};

					trace!("indent {:?}", indent);

					if let Some(tag) = tag_block_top {
						match indent.1 {
							IndentKind::Standard => {}
							IndentKind::OrderedList | IndentKind::UnorderedList => {
								let (primary, _) = ctx.doll.resolve_span(indent.0);
								let (tag, _) = ctx.doll.resolve_span(tag);
								ctx.diag(LangDiagnostic::MisalignedList { primary, tag });
							}
						}
					} else if indent_level > ctx.stack_indent() {
						more(ctx, indent);
					} else {
						less(ctx, indent, indent_level, last_significant);
					}
				}

				Some(ch) => {
					if ch == '\r' {
						ctx.crlf_explode();
					}

					if !resume_standard_parsing(ctx, &mut indent_level) {
						return ParseResult::NextLine;
					}

					*last_significant = true;

					break;
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
	#[instrument(name = "tag::parse_inline_text", level = Level::DEBUG)]
	fn parse_inline_text<Ctx>(ctx: &mut ParseCtx<Ctx>) -> Option<(Span, Loc)> {
		let start = ctx.stream.lookahead_loc(1);
		let mut text = String::with_capacity(32);
		let mut bracket_stack: usize = 0;

		loop {
			match ctx.stream.next() {
				Some('\n') => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "newline",
						expected: &["tag content", "`]`", "`\\`"],
					});
					break;
				}

				Some('\t') => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "indentation",
						expected: &["tag content", "`]`", "`\\`"],
					});
				}

				Some('\\') => match ctx.stream.next() {
					Some('\n') => {
						let (primary, context) = ctx.doll.resolve_span(
							ctx.stream
								.lookahead_span(-1)
								.union(&ctx.stream.lookahead_span(0)),
						);
						ctx.diag(LangDiagnostic::CannotEscapeHere {
							primary,
							context,
							what: "newline",
							here: "line tag",
						});
					}

					Some('\t') => {
						let (primary, context) = ctx.doll.resolve_span(
							ctx.stream
								.lookahead_span(-1)
								.union(&ctx.stream.lookahead_span(0)),
						);
						ctx.diag(LangDiagnostic::CannotEscape {
							primary,
							context,
							what: "indentation",
						});
					}

					Some(ch) => {
						if ch == '\r' {
							ctx.crlf_explode();
						}
						text.push(ch);
					}

					None => {
						let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "end of input",
							expected: &["char"],
						});
						return None;
					}
				},

				Some('[') => {
					text.push('[');
					bracket_stack += 1;
				}
				Some(']') => {
					if bracket_stack > 0 {
						text.push(']');
						bracket_stack -= 1;
					} else {
						break;
					}
				}

				Some(ch) => {
					if ch == '\r' {
						ctx.crlf_explode();
					}
					text.push(ch);
				}

				None => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "end of input",
						expected: &["tag content", "`]`", "`\\`"],
					});
					return None;
				}
			}
		}

		let from = Span::new(
			start.unwrap(), // unwrap here because lookahead
			ctx.stream.lookahead_loc(0).unwrap(),
		);
		let verbatim = text == ctx.doll.spanner.lookup_src(from);

		Some((
			ctx.doll
				.spanner
				.add(|_| MarkDollSrc {
					metadata: SourceMetadata::LineTag { from, verbatim },
					source: text,
				})
				.span(),
			ctx.stream.lookahead_loc(-1).unwrap(),
		))
	}

	/// transform tag text to actual content
	#[instrument(name = "tag::transform_content", level = Level::DEBUG)]
	pub fn transform_content<Ctx>(
		ctx: &mut ParseCtx<Ctx>,
		tag: Span,
		args: &[Span],
		text: Span,
	) -> Option<Box<dyn TagContent>> {
		if let Some(def) = ctx.doll.tags.get(&*ctx.doll.spanner.lookup_span(tag)) {
			(def.parse)(
				ctx.doll,
				args.iter()
					.map(|span| ctx.doll.spanner.lookup_span(*span))
					.collect(),
				ctx.doll.spanner.lookup_span(text),
				tag,
			)
		} else {
			let (primary, context) = ctx.doll.resolve_span(tag);
			ctx.diag(LangDiagnostic::UndefinedTag { primary, context });
			None
		}
	}

	#[instrument(name = "tag::parse_arg", level = Level::DEBUG)]
	fn parse_arg<Ctx>(ctx: &mut ParseCtx<Ctx>) -> ParseResult<Span> {
		let start = ctx.stream.lookahead_loc(0).unwrap();
		let mut arg = String::new();
		let mut paren_stack: usize = 0;

		'arg: loop {
			match ctx.stream.next() {
				Some('\n') => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "newline",
						expected: &["argument", "`)`", "`\\`"],
					});
					break 'arg;
				}

				Some('\t') => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "indentation",
						expected: &["argument", "`)`", "`\\`"],
					});
				}

				Some('\\') => match ctx.stream.next() {
					Some(ch @ '\n') | Some(ch @ '\t') => {
						let (primary, context) = ctx.doll.resolve_span(
							ctx.stream
								.lookahead_span(-1)
								.union(&ctx.stream.lookahead_span(0)),
						);
						ctx.diag(LangDiagnostic::CannotEscape {
							primary,
							context,
							what: match ch {
								'\n' => "newline",
								'\t' => "indentation",
								_ => unreachable!(),
							},
						});
					}

					Some(ch) => {
						if ch == '\r' {
							ctx.crlf_explode();
						}
						arg.push(ch);
					}

					None => {
						let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "end of input",
							expected: &["char"],
						});
						return ParseResult::Stop;
					}
				},

				Some('(') => {
					arg.push('(');
					paren_stack += 1;
				}
				Some(')') => {
					if paren_stack > 0 {
						arg.push(')');
						paren_stack -= 1;
					} else {
						break;
					}
				}

				Some(ch) => {
					if ch == '\r' {
						ctx.crlf_explode();
					}
					arg.push(ch);
				}

				None => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "end of input",
						expected: &["argument", "`)`", "`\\`"],
					});
					return ParseResult::Stop;
				}
			}
		}

		let from = Span::new(start + 1, ctx.stream.lookahead_loc(0).unwrap());
		let verbatim = arg == ctx.doll.spanner.lookup_src(from);

		ParseResult::Ok(
			ctx.doll
				.spanner
				.add(|_| MarkDollSrc {
					metadata: SourceMetadata::TagArgument { from, verbatim },
					source: arg,
				})
				.span(),
		)
	}

	#[instrument(name = "tag::parse_content", level = Level::DEBUG)]
	fn parse_content<Ctx>(
		ctx: &mut ParseCtx<Ctx>,
		tag_start: Loc,
		name: Span,
		args: Vec<Span>,
		indent_level: usize,
	) -> ParseResult {
		match ctx.stream.lookahead(1) {
			Some('\n') => {
				ctx.stream.skip();

				let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
				ctx.diag(LangDiagnostic::Unexpected {
					primary,
					context,
					unexpected: "newline",
					expected: &["tag content", "`:`", "`]`", "`\\`"],
				});
				return ParseResult::Ok(());
			}

			Some(':') => {
				ctx.stream.skip();

				let parent_span = ctx.stream.lookahead_span(2);
				ctx.stack.push(
					StackPart::BlockTag {
						name,
						args,
						text: String::new(),
						parent_span,
						indent: indent_level + 1,
					}
					.locd(tag_start),
				);

				match ctx.stream.next() {
					Some('\n') => {}

					// not a newline
					Some(ch) => {
						if ch == '\r' {
							ctx.crlf_explode();
						}

						let (primary, context) =
							ctx.doll.resolve_span(ctx.stream.lookahead_span(1));
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "char",
							expected: &["newline"],
						});

						// eat characters until newline
						if !ctx.eat_until_newline() {
							return ParseResult::Stop;
						}
					}

					None => {
						let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "end of input",
							expected: &["newline"],
						});
						return ParseResult::Stop;
					}
				}

				return ParseResult::NextLine;
			}

			Some(ch) => {
				if ch == '\r' {
					ctx.crlf_explode();
				}

				if let Some((text, tag_end)) = parse_inline_text(ctx) {
					if let Some(content) = transform_content(ctx, name, &args, text) {
						ctx.inline.push(
							InlineItem::Tag(TagInvocation { name, content })
								.spanned(Span::new(tag_start, tag_end)),
						);
					}
				}
			}

			None => {
				let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
				ctx.diag(LangDiagnostic::Unexpected {
					primary,
					context,
					unexpected: "end of input",
					expected: &["tag content", "`]`", "`\\`"],
				});
				return ParseResult::Stop;
			}
		}

		ParseResult::Ok(())
	}

	#[instrument(name = "tag::parse", level = Level::DEBUG, ret)]
	pub fn parse<Ctx>(ctx: &mut ParseCtx<Ctx>, indent_level: usize) -> ParseResult {
		let start = ctx.stream.lookahead_loc(0).unwrap();
		let mut args = Vec::new();

		let name = Span::new(
			start + 1,
			loop {
				match ctx.stream.next() {
					Some('\n') => {
						let (primary, context) =
							ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "newline",
							expected: &["tag name", "`(`", "`:`", "`]`"],
						});
						return ParseResult::NextLine;
					}

					Some('\t') => {
						let (primary, context) =
							ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "indentation",
							expected: &["tag name", "`(`", "`:`", "`]`"],
						});
					}

					Some('(') | Some(':') | Some(']') => {
						ctx.stream.back();
						break ctx.stream.lookahead_loc(1).unwrap();
					}

					Some(ch) => {
						if ch == '\r' {
							ctx.crlf_explode();
						}
					}

					None => {
						let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
						ctx.diag(LangDiagnostic::Unexpected {
							primary,
							context,
							unexpected: "end of input",
							expected: &["tag name", "`(`", "`:`", "`]`"],
						});
						return ParseResult::Stop;
					}
				}
			},
		);

		trace!(?name);

		trace!(name = ctx.doll.spanner.lookup_src(name), span = ?name, "parsed name");

		loop {
			match ctx.stream.next() {
				Some('\n') => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "newline",
						expected: &["`(`", "`:`", "`]`"],
					});
					break;
				}

				Some('\t') => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "indentation",
						expected: &["`(`", "`:`", "`]`"],
					});
				}

				Some('(') => match parse_arg(ctx) {
					ParseResult::Ok(arg) => args.push(arg),
					ParseResult::NextLine => return ParseResult::NextLine,
					ParseResult::Stop => return ParseResult::Stop,
				},

				Some(':') => match parse_content(ctx, start, name, args, indent_level) {
					ParseResult::Ok(()) => break,
					ParseResult::NextLine => return ParseResult::NextLine,
					ParseResult::Stop => return ParseResult::Stop,
				},

				Some(']') => {
					if let Some(content) =
						transform_content(ctx, name, &args, name.end().with_len(0))
					{
						ctx.inline.push(
							InlineItem::Tag(TagInvocation {
								name: name,
								content,
							})
							.spanned(Span::new(start, ctx.stream.lookahead_loc(0).unwrap())),
						);
					}

					break;
				}

				Some(_) => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "char",
						expected: &["`(`", "`:`", "`]`"],
					});
					return ParseResult::NextLine;
				}

				None => {
					let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
					ctx.diag(LangDiagnostic::Unexpected {
						primary,
						context,
						unexpected: "end of input",
						expected: &["`(`", "`:`", "`]`"],
					});
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
pub(crate) fn parse<Ctx>(ctx: &mut ParseCtx<Ctx>) -> (bool, AST) {
	// significance tracks functionally-empty lines, splitting paragraphs at functionally-empty lines
	let mut last_significant = false;

	'main: loop {
		let traced_span = trace_span!("line").entered();

		// parse indentation
		let indent_level: usize = match indent::parse(ctx, &mut last_significant) {
			ParseResult::Ok(indent_level) => indent_level,
			ParseResult::NextLine => continue 'main,
			ParseResult::Stop => break 'main,
		};

		if let Locd(_, StackPart::SectionHeader { .. }) = ctx.stack.last_mut().unwrap() {
			ctx.stack_terminate_top();
		}

		// decide what to do based off of stack top
		let Locd(_, top) = ctx.stack.last_mut().unwrap();
		match top {
			// normal
			StackPart::Root { .. } | StackPart::List { .. } | StackPart::Section { .. } => {
				// section heads must be the start of their line
				if ctx.stream.try_eat('&') {
					let start = ctx.stream.lookahead_loc(0).unwrap();

					ctx.flush_inline();

					ctx.stack
						.push(StackPart::SectionHeader { header: Vec::new() }.locd(start));
				}

				// line loop
				loop {
					match ctx.stream.next() {
						Some('\n') => {
							if let Some(Spanned(_, InlineItem::Text(_) | InlineItem::Tag(_))) =
								ctx.inline.last()
							{
								ctx.inline
									.push(InlineItem::Split.spanned(ctx.stream.lookahead_span(0)));
							}

							continue 'main;
						}

						Some('\t') => {
							let (primary, context) =
								ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
							ctx.diag(LangDiagnostic::Unexpected {
								primary,
								context,
								unexpected: "indentation",
								expected: &["text", "newline", "`[`", "`\\`"],
							});
						}

						// parse a tag invocation
						Some('[') => match tag::parse(ctx, indent_level) {
							ParseResult::Ok(()) => {}
							ParseResult::NextLine => continue 'main,
							ParseResult::Stop => break 'main,
						},

						// start parsing text
						Some(ch) => {
							if ch == '\r' {
								ctx.crlf_explode();
							}

							ctx.stream.back();

							let start = ctx.stream.at;

							let mut text = String::new();

							let traced_span = trace_span!("text").entered();
							'text: loop {
								match ctx.stream.next() {
									Some('\n') => {
										trace!(text, "newline in text, ending text segment");
										ctx.inline.push(InlineItem::Text(text).spanned(Span::new(
											start,
											ctx.stream.lookahead_loc(-1).unwrap(),
										)));
										ctx.inline.push(
											InlineItem::Split.spanned(ctx.stream.lookahead_span(0)),
										);

										continue 'main;
									}

									Some('\t') => {
										let (primary, context) =
											ctx.doll.resolve_span(ctx.stream.lookahead_span(0));
										ctx.diag(LangDiagnostic::Unexpected {
											primary,
											context,
											unexpected: "indentation",
											expected: &["text", "newline", "`[`", "`\\`"],
										});
									}

									// escape sequence
									Some('\\') => match ctx.stream.next() {
										Some('\t') => {
											let (primary, context) = ctx.doll.resolve_span(
												ctx.stream
													.lookahead_span(-1)
													.union(&ctx.stream.lookahead_span(0)),
											);
											ctx.diag(LangDiagnostic::CannotEscape {
												primary,
												context,
												what: "indentation",
											});
										}

										Some('\n') => {
											if !text.is_empty() {
												ctx.inline.push(InlineItem::Text(text).spanned(
													Span::new(
														start,
														ctx.stream.lookahead_loc(-2).unwrap(),
													),
												));
											}
											ctx.inline.push(
												InlineItem::Break.spanned(
													ctx.stream
														.lookahead_span(-1)
														.union(&ctx.stream.lookahead_span(0)),
												),
											);

											continue 'main;
										}

										Some(ch) => {
											if ch == '\r' {
												ctx.crlf_explode();
											}

											trace!(?ch, "escape code");
											text.push(ch);
										}

										None => {
											ctx.inline.push(InlineItem::Text(text).spanned(
												Span::new(
													start,
													ctx.stream.lookahead_loc(-1).unwrap(),
												),
											));
											let (primary, context) =
												ctx.doll.resolve_span(ctx.stream.eof_span());
											ctx.diag(LangDiagnostic::Unexpected {
												primary,
												context,
												unexpected: "end of input",
												expected: &["char"],
											});
											break 'main;
										}
									},

									// back up, let the tag parser handle this
									Some('[') => {
										ctx.inline.push(InlineItem::Text(text).spanned(Span::new(
											start,
											ctx.stream.lookahead_loc(-1).unwrap(),
										)));
										ctx.stream.back();
										break 'text;
									}

									Some(ch) => {
										if ch == '\r' {
											ctx.crlf_explode();
										}
										text.push(ch);
									}

									None => {
										ctx.inline.push(
											InlineItem::Text(text)
												.spanned(Span::new(start, ctx.stream.at)),
										);
										break 'main;
									}
								}
							}
							traced_span.exit();
						}

						None => break 'main,
					}
				}
			}
			// special, just insert content straight into the tag
			StackPart::BlockTag {
				text, parent_span, ..
			} => {
				let mut warn_cr = false;

				let traced_span = trace_span!("continue_tag_block_content").entered();
				'line: loop {
					match ctx.stream.next() {
						Some('\n') => {
							text.push('\n');
							*parent_span = Span::new(
								parent_span.start(),
								ctx.stream.lookahead_loc(0).unwrap(),
							);
							break 'line;
						}

						Some(ch) => {
							text.push(ch);

							if ch == '\r' {
								warn_cr = true;
							}
						}

						None => {
							*parent_span = Span::new(
								parent_span.start(),
								ctx.stream.lookahead_loc(-1).unwrap(),
							);
							break 'main;
						}
					}
				}
				traced_span.exit();

				if warn_cr {
					ctx.crlf_explode();
				}
			}
			StackPart::SectionHeader { .. } => unreachable!(),
		}

		traced_span.exit();
	}

	{
		let traced = trace_span!("final_squimsh").entered();

		while ctx.stack.len() > 1 {
			trace!(?ctx.stack);
			let Locd(_, top) = ctx.stack.last().unwrap();
			let graceful = !matches!(top, StackPart::BlockTag { .. });
			trace!(?top, graceful, "terminating",);

			if graceful {
				ctx.flush_inline();
				ctx.stack_terminate_top();
			} else {
				let (primary, context) = ctx.doll.resolve_span(ctx.stream.eof_span());
				ctx.diag(LangDiagnostic::Unexpected {
					primary,
					context,
					unexpected: "end of input",
					expected: &["`]` to close tag"],
				});

				// force it anyways
				ctx.stack_terminate_top();
				ctx.flush_inline();
			}
		}

		drop(traced);
	}

	ctx.flush_inline();

	let Some(Locd(_, StackPart::Root { children: ast })) = ctx.stack.pop() else {
		unreachable!()
	};

	(ctx.doll.ok, ast)
}
