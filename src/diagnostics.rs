#[cfg(feature = "ariadne")]
use ariadne::{Label, Report, ReportKind};
use {
	alloc::{rc::Rc, vec::Vec},
	core::cmp::Ordering,
};

/// an issue in the source
#[derive(Debug)]
pub struct Diagnostic {
	/// whether this diagnostic should indicate invalid state
	pub err: bool,
	/// the location of this diagnostic in the source
	pub at: usize,
	/// the content of this diagnostic
	pub code: &'static str,
	#[cfg(debug_assertions)]
	#[allow(unused, reason = "conditional")]
	pub(crate) src: &'static core::panic::Location<'static>,
}

#[derive(Debug)]
pub(crate) struct TagDiagnosticTranslation {
	pub src: Rc<str>,
	pub indexed: Option<IndexedSrc>,
	pub offset_in_parent: usize,
	pub tag_pos_in_parent: usize,
	pub indent: usize,
}

#[derive(Debug)]
pub(crate) struct IndexedSrc {
	lines: Vec<(usize, usize)>,
	parent_lines: Vec<usize>,
}

impl IndexedSrc {
	#[must_use]
	pub fn index(src: &str, parent: &str, offset_in_parent: usize, indent: usize) -> Self {
		let lines = {
			let mut lines = Vec::new();

			let mut start = 0;
			for line in src.split('\n') {
				let end = start + line.len() + 1;
				lines.push((start, end - 1));
				start = end;
			}

			lines
		};

		let parent_lines = {
			let mut parent_lines = Vec::new();

			let mut start = 0;
			for line in parent[offset_in_parent.min(parent.len().saturating_sub(1))..]
				.split('\n')
				.take(lines.len())
			{
				let end = start + line.len() + 1;

				let chars = line.chars().collect::<Vec<char>>();
				let mut ind = 0;

				for _ in 0..indent {
					match t!("char", chars.get(ind)) {
						Some('\t') => ind += 1,
						Some('-' | '=') => ind += 2,
						None => {}
						_ => unreachable!(),
					}
				}

				start += ind;

				parent_lines.push(offset_in_parent + start);
				start = end;
			}

			parent_lines
		};

		Self {
			lines,
			parent_lines,
		}
	}

	#[must_use]
	pub fn parent_offset(&self, index: usize) -> usize {
		let mut offset_within_line = 0;
		let mut last = 0;

		t!("parent_offset index", index);

		self.parent_lines[self
			.lines
			.binary_search_by(|(line_start, line_end)| {
				if *line_end < index {
					return Ordering::Less;
				}

				if *line_start > index {
					return Ordering::Greater;
				}

				offset_within_line = index - line_start;
				last = *line_end;

				Ordering::Equal
			})
			.unwrap_or(last)]
			+ offset_within_line
	}
}

/// render [`Diagnostic`]s to ariadne [`Reports`](ariadne::Report)
#[must_use]
#[allow(
	clippy::range_plus_one,
	reason = "does not account for RangeInclusive not being accepted"
)]
#[cfg(feature = "ariadne")]
pub fn render(diagnostics: &[Diagnostic]) -> Vec<Report<'static>> {
	diagnostics
		.iter()
		.map(|diag| {
			#[allow(unused_mut, reason = "conditional")]
			let mut builder = Report::build(ReportKind::Error, (), diag.at)
				.with_message(diag.code)
				.with_label(
					Label::new(diag.at..diag.at + 1)
						.with_color(ariadne::Color::Magenta)
						.with_message(diag.code),
				);

			#[cfg(debug_assertions)]
			builder.set_note(alloc::format!("originated from {}", diag.src));

			builder.finish()
		})
		.collect()
}
