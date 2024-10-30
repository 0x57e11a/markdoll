use {
	crate::{
		args,
		ext::TagDefinition,
		tree::{BlockItem, InlineItem, TagInvocation, AST},
		MarkDoll,
	},
	alloc::{boxed::Box, vec::Vec},
};

#[derive(Debug)]
struct Cell {
	pub is_head: bool,
	pub rows: usize,
	pub cols: usize,
	pub content: AST,
}

#[derive(Debug)]
struct Row {
	pub is_head: bool,
	pub cells: Vec<Cell>,
}

#[derive(Debug)]
struct Table {
	pub head: Vec<Row>,
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
pub const TBL_TAG: TagDefinition = TagDefinition {
	key: "table",
	parse: Some(|doll, _, text| {
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
	emit: |doll, to, content| {
		fn write_cell(doll: &mut MarkDoll, to: &mut dyn core::fmt::Write, cell: &mut Cell) {
			let kind = if cell.is_head { "th" } else { "td" };
			write!(to, "<{kind}").unwrap();

			if cell.rows != 1 {
				write!(to, " rowspan='{}'", cell.rows).unwrap();
			}
			if cell.cols != 1 {
				write!(to, " colspan='{}'", cell.cols).unwrap();
			}

			to.write_str(">").unwrap();

			let block = cell.content.len() > 1;
			for content in &mut cell.content {
				content.emit(doll, to, block);
			}

			write!(to, "</{kind}>").unwrap();
		}

		let table = content.downcast_mut::<Table>().unwrap();

		to.write_str("<table>").unwrap();

		if !table.head.is_empty() {
			to.write_str("<thead>").unwrap();

			for row in &mut table.head {
				to.write_str("<tr>").unwrap();

				for cell in &mut row.cells {
					write_cell(doll, to, cell);
				}

				to.write_str("</tr>").unwrap();
			}

			to.write_str("</thead>").unwrap();
		}

		if !table.body.is_empty() {
			to.write_str("<tbody>").unwrap();

			for row in &mut table.body {
				to.write_str("<tr>").unwrap();

				for cell in &mut row.cells {
					write_cell(doll, to, cell);
				}

				to.write_str("</tr>").unwrap();
			}

			to.write_str("</tbody>").unwrap();
		}

		to.write_str("</table>").unwrap();
	},
};

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
pub const TBLROW_TAG: TagDefinition = TagDefinition {
	key: "tr",
	parse: Some(|doll, mut args, text| {
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
	emit: |doll, _, _| {
		doll.diag(true, usize::MAX, "tblrow outside of tbl");
	},
};

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
pub const TBLCELL_TAG: TagDefinition = TagDefinition {
	key: "tc",
	parse: Some(|doll, mut args, text| {
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
	emit: |doll, _, _| {
		doll.diag(true, usize::MAX, "table(cell) outside of table");
	},
};

/// all of this module's tags
pub const TAGS: &[TagDefinition] = &[TBL_TAG, TBLROW_TAG, TBLCELL_TAG];
