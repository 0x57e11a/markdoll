use {
	crate::{
		args,
		diagnostics::DiagnosticKind,
		emit::html::HtmlEmit,
		ext::{Emitters, TagDefinition, TagEmitter},
		tree::{BlockItem, InlineItem, TagContent, TagInvocation, AST},
		MarkDoll,
	},
	::miette::{LabeledSpan, SourceSpan},
	::spanner::{Span, Spanned},
};

/// an error with tables
#[derive(Debug, ::thiserror::Error, ::miette::Diagnostic)]
pub enum TableDiagnostic {
	/// the table contained something that was not a row
	#[error("tables may only contain lists and `tr` tags")]
	#[diagnostic(code(markdoll::ext::table::non_row))]
	NonRow {
		/// the offending syntax
		#[label]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
	/// the table row contained something that was not a cell
	#[error("table rows may only contain lists and `tc` tags")]
	#[diagnostic(code(markdoll::ext::table::non_cell))]
	NonCell {
		/// the offending syntax
		#[label]
		at: SourceSpan,
		/// context
		#[label(collection)]
		context: Vec<LabeledSpan>,
	},
}

/// a table cell
#[derive(Debug)]
pub struct Cell {
	/// whether this cell is a head cell
	pub is_head: bool,
	/// how many rows to span
	pub rows: usize,
	/// how many columns to span
	pub cols: usize,
	/// content
	pub ast: AST,
}

/// a table row
#[derive(Debug)]
pub struct Row {
	/// whether this row is a head row, which is placed in the `head` section of the table
	pub is_head: bool,
	/// the cells on this row
	pub cells: Vec<Cell>,
}

/// a table
#[derive(Debug)]
pub struct Table {
	/// `<thead>` section
	pub head: Vec<Row>,
	/// `<tbody>` section
	pub body: Vec<Row>,
}

fn parse_row<Ctx>(doll: &mut MarkDoll<Ctx>, ast: AST) -> Vec<Cell> {
	#[track_caller]
	fn fail<Ctx>(doll: &mut MarkDoll<Ctx>, span: Span) {
		let (at, context) = doll.resolve_span(span);
		doll.diag(DiagnosticKind::Tag(Box::new(TableDiagnostic::NonCell {
			at,
			context,
		})));
	}

	let mut cells = Vec::new();

	for Spanned(span, child) in ast {
		match child {
			BlockItem::Inline(items) => {
				for Spanned(span, item) in items {
					match item {
						InlineItem::Split | InlineItem::Break => {}
						InlineItem::Tag(TagInvocation { content, .. }) => {
							if let Ok(cell) =
								(content as Box<dyn ::core::any::Any>).downcast::<Cell>()
							{
								cells.push(*cell);
							} else {
								fail(doll, span);
							}
						}
						_ => fail(doll, span),
					}
				}
			}
			BlockItem::List { ordered, items, .. } => {
				for item in items {
					cells.push(Cell {
						is_head: ordered,
						rows: 1,
						cols: 1,
						ast: item,
					});
				}
			}
			BlockItem::Section { .. } => fail(doll, span),
		}
	}

	cells
}

/// `table` tag
///
/// make tables
///
/// # content
///
/// multiple of the following
/// - [`tr`](TBLROW_TAG) tags
/// - ordered lists (header rows)
/// - unordered lists (body rows)
pub mod table {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "table",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;
				}

				#[track_caller]
				fn fail<Ctx>(doll: &mut MarkDoll<Ctx>, span: Span) {
					let (at, context) = doll.resolve_span(span);
					doll.diag(DiagnosticKind::Tag(Box::new(TableDiagnostic::NonRow {
						at,
						context,
					})));
				}

				let mut table = Table {
					head: Vec::new(),
					body: Vec::new(),
				};

				for Spanned(span, child) in doll.parse_embedded(text.into()) {
					match child {
						BlockItem::Inline(items) => {
							for Spanned(span, item) in items {
								match item {
									InlineItem::Split | InlineItem::Break => {}
									InlineItem::Tag(TagInvocation { content, .. }) => {
										if let Ok(row) =
											(content as Box<dyn ::core::any::Any>).downcast::<Row>()
										{
											if row.is_head {
												table.head.push(*row);
											} else {
												table.body.push(*row);
											}
										} else {
											fail(doll, span);
										}
									}
									_ => fail(doll, span),
								}
							}
						}
						BlockItem::List { ordered, items, .. } => {
							for item in items {
								let row = Row {
									is_head: ordered,
									cells: parse_row(doll, item),
								};

								if row.is_head {
									table.head.push(row);
								} else {
									table.body.push(row);
								}
							}
						}
						BlockItem::Section { .. } => fail(doll, span),
					}
				}

				Some(Box::new(table))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new().with(html::<Ctx>),
		}
	}

	/// emit to html
	pub fn html<Ctx>(
		doll: &mut MarkDoll<Ctx>,
		to: &mut HtmlEmit,
		ctx: &mut Ctx,
		content: &mut dyn TagContent,
		_: Span,
	) {
		fn write_cell<Ctx>(
			doll: &mut MarkDoll<Ctx>,
			to: &mut HtmlEmit,
			ctx: &mut Ctx,
			cell: &mut Cell,
		) {
			let kind = if cell.is_head { "th" } else { "td" };
			to.write.push_str(&format!("<{kind}"));

			if cell.rows != 1 {
				to.write.push_str(&format!(" rowspan='{}'", cell.rows));
			}
			if cell.cols != 1 {
				to.write.push_str(&format!(" colspan='{}'", cell.cols));
			}

			to.write.push('>');

			let inline_block = cell.ast.len() > 1;
			for Spanned(_, content) in &mut cell.ast {
				content.emit(doll, to, ctx, inline_block);
			}

			to.write.push_str(&format!("</{kind}>"));
		}

		let table = (content as &mut dyn ::core::any::Any)
			.downcast_mut::<Table>()
			.unwrap();

		to.write.push_str("<div class='doll-table'><table>");

		if !table.head.is_empty() {
			to.write.push_str("<thead>");

			for row in &mut table.head {
				to.write.push_str("<tr>");

				for cell in &mut row.cells {
					write_cell(doll, to, ctx, cell);
				}

				to.write.push_str("</tr>");
			}

			to.write.push_str("</thead>");
		}

		if !table.body.is_empty() {
			to.write.push_str("<tbody>");

			for row in &mut table.body {
				to.write.push_str("<tr>");

				for cell in &mut row.cells {
					write_cell(doll, to, ctx, cell);
				}

				to.write.push_str("</tr>");
			}

			to.write.push_str("</tbody>");
		}

		to.write.push_str("</table></div>");
	}
}

/// `tr` tag
///
/// rows inside of [`table`](TBL_TAG) tags
///
/// # flags
///
/// - `head`\
///   makes this row a header row
///
/// # content
///
/// multiple of the following
/// - [`tc`](TBLCELL_TAG) tags
/// - ordered lists (header cells)
/// - unordered lists (body cells)
///
/// # emitting
///
/// this tag will never be emitted when used properly, do not add an emitter to it
pub mod tr {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "tr",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					flags(head);
				}

				let ast = doll.parse_embedded(text.into());

				Some(Box::new(Row {
					is_head: head,
					cells: parse_row(doll, ast),
				}))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new(),
		}
	}
}

/// `tc` tag
///
/// cells inside of [`tr`](TBLROW_TAG) tags
///
/// # flags
///
/// - `head`\
///   makes this row a header row
///
/// # props
///
/// - `rowspan`\
///   the amount of rows this cell should span
/// - `colspan`\
///   the amount of columns this cell should span
///
/// # content
///
/// markdoll
///
/// # emitting
///
/// this tag will never be emitted when used properly, do not add an emitter to it
pub mod tc {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "tc",
			parse: |doll, args, text, tag_span| {
				args! {
					args;
					doll, tag_span;

					flags(head);
					props(rows: usize, cols: usize);
				}

				Some(Box::new(Cell {
					is_head: head,
					rows: rows.unwrap_or(1),
					cols: cols.unwrap_or(1),
					ast: doll.parse_embedded(text.into()),
				}))
			},
			emitters: Emitters::<TagEmitter<Ctx>>::new(),
		}
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	[table::tag::<Ctx>(), tr::tag::<Ctx>(), tc::tag::<Ctx>()]
}
