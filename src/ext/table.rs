use {
	crate::{
		args,
		emit::HtmlEmit,
		ext::TagDefinition,
		tree::{BlockItem, InlineItem, TagContent, TagInvocation, AST},
		MarkDoll,
	},
	::alloc::format,
	alloc::{boxed::Box, vec::Vec},
};

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
	pub content: AST,
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

fn parse_row(doll: &mut MarkDoll, ast: AST) -> Vec<Cell> {
	#[track_caller]
	fn fail(doll: &mut MarkDoll, pos: usize) {
		doll.diag(true, pos, "tr may only lists and table cell tags");
	}

	let mut cells = Vec::new();

	for child in ast {
		match child {
			BlockItem::Inline(items) => {
				for (pos, item) in items {
					match item {
						InlineItem::Tag(TagInvocation { content, .. }) => {
							if let Ok(cell) = content.downcast::<Cell>() {
								cells.push(*cell);
							} else {
								fail(doll, pos);
							}
						}
						_ => fail(doll, pos),
					}
				}
			}
			BlockItem::List { ordered, items, .. } => {
				for item in items {
					cells.push(Cell {
						is_head: ordered,
						rows: 1,
						cols: 1,
						content: item,
					});
				}
			}
			BlockItem::Section { pos, .. } => fail(doll, pos),
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
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"table",
			Some(|doll, _, text| {
				#[track_caller]
				fn fail(doll: &mut MarkDoll, pos: usize) {
					doll.diag(
						true,
						pos,
						"table tags may only contain lists and table row tags",
					);
				}

				let ast = match doll.parse(text) {
					Ok(ast) => ast,
					Err(ast) => {
						doll.ok = false;
						ast
					}
				};

				let mut table = Table {
					head: Vec::new(),
					body: Vec::new(),
				};

				for child in ast {
					match child {
						BlockItem::Inline(items) => {
							for (pos, item) in items {
								match item {
									InlineItem::Tag(TagInvocation { content, .. }) => {
										if let Ok(row) = content.downcast::<Row>() {
											if row.is_head {
												table.head.push(*row);
											} else {
												table.body.push(*row);
											}
										} else {
											fail(doll, pos);
										}
									}
									_ => fail(doll, pos),
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
						BlockItem::Section { pos, .. } => fail(doll, pos),
					}
				}

				Some(Box::new(table))
			}),
		)
		.with_emitter::<HtmlEmit>(html)
	}

	/// emit to html
	pub fn html(doll: &mut MarkDoll, to: &mut HtmlEmit, content: &mut Box<dyn TagContent>) {
		fn write_cell(doll: &mut MarkDoll, to: &mut HtmlEmit, cell: &mut Cell) {
			let kind = if cell.is_head { "th" } else { "td" };
			to.write.push_str(&format!("<{kind}"));

			if cell.rows != 1 {
				to.write.push_str(&format!(" rowspan='{}'", cell.rows));
			}
			if cell.cols != 1 {
				to.write.push_str(&format!(" colspan='{}'", cell.cols));
			}

			to.write.push('>');

			for content in &mut cell.content {
				content.emit(doll, to);
			}

			to.write.push_str(&format!("</{kind}>"));
		}

		let table = content.downcast_mut::<Table>().unwrap();

		to.write.push_str("<table>");

		if !table.head.is_empty() {
			to.write.push_str("<thead>");

			for row in &mut table.head {
				to.write.push_str("<tr>");

				for cell in &mut row.cells {
					write_cell(doll, to, cell);
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
					write_cell(doll, to, cell);
				}

				to.write.push_str("</tr>");
			}

			to.write.push_str("</tbody>");
		}

		to.write.push_str("</table>");
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
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"tr",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args();
					opt_args();
					flags(head);
					props();
				}

				Some(Box::new(Row {
					is_head: head,
					cells: {
						let ast = match doll.parse(text) {
							Ok(ast) => ast,
							Err(ast) => {
								doll.ok = false;
								ast
							}
						};
						parse_row(doll, ast)
					},
				}))
			}),
		)
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
	pub fn tag() -> TagDefinition {
		TagDefinition::new(
			"tc",
			Some(|doll, mut args, text| {
				args! {
					doll, args;

					args();
					opt_args();
					flags(head);
					props(rows: usize, cols: usize);
				}

				Some(Box::new(Cell {
					is_head: head,
					rows: rows.unwrap_or(1),
					cols: cols.unwrap_or(1),
					content: match doll.parse(text) {
						Ok(ast) => ast,
						Err(ast) => {
							doll.ok = false;
							ast
						}
					},
				}))
			}),
		)
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> [TagDefinition; 3] {
	[table::tag(), tr::tag(), tc::tag()]
}
