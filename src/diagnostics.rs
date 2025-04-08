use {
	crate::{
		emit::EmitDiagnostic, ext::TagInputDiagnostic, tree::parser::LangDiagnostic, MarkDollSrc,
	},
	::miette::Diagnostic,
	::spanner::{Loc, Span, Spanner},
	::std::sync::Mutex,
	::tracing::{instrument, trace},
};

/// diagnostics
#[derive(::thiserror::Error, ::miette::Diagnostic, Debug)]
pub enum DiagnosticKind {
	/// language diagnostic
	#[error(transparent)]
	#[diagnostic(transparent)]
	Lang(
		#[from]
		#[diagnostic_source]
		LangDiagnostic,
	),
	/// emitting diagnostic
	#[error(transparent)]
	#[diagnostic(transparent)]
	Emit(
		#[from]
		#[diagnostic_source]
		EmitDiagnostic,
	),
	/// tag input diagnostic
	#[error(transparent)]
	#[diagnostic(transparent)]
	TagArgs(
		#[from]
		#[diagnostic_source]
		TagInputDiagnostic,
	),
	/// custom tag diagnostic
	#[error(transparent)]
	#[diagnostic(transparent)]
	Tag(
		#[from]
		#[diagnostic_source]
		Box<dyn Diagnostic + Send + Sync>,
	),
}

/// keeps track of how block-tags should translate their content
#[derive(Debug)]
pub struct TagDiagnosticTranslation {
	/// span this tag content covers in the source that contains it
	pub parent_span: Span,
	/// content of the tag
	pub span: Span,
	/// for each line of tag's content, the location in the parent source where that line begins
	pub lines_to_parent_line_starts: Mutex<Option<Vec<Loc>>>,
	/// how many levels of indentation before the content
	pub parent_indent: usize,
}

impl TagDiagnosticTranslation {
	/// transform a span to its parent span
	#[must_use]
	#[instrument(skip(spanner), ret)]
	pub fn to_parent(&self, spanner: &Spanner<MarkDollSrc>, span: Span) -> Span {
		let parent = spanner.lookup_buf(self.parent_span.start());
		let child = spanner.lookup_buf(self.span.start());

		trace!(parent_source = ?spanner.lookup_span(self.span), "source");

		let mut lock = self.lines_to_parent_line_starts.lock().unwrap();
		let translations = lock.get_or_insert_with(|| {
			let mut parent_lines = Vec::new();

			let mut start = 0;
			for line in spanner
				.lookup_span(self.parent_span)
				.chars()
				.collect::<Vec<_>>()
				.split(|ch| *ch == '\n')
				.take(child.len_lines())
			{
				let end = start + line.len() + 1;

				let mut ind = 0;

				for _ in 0..self.parent_indent {
					match line.get(ind) {
						Some('\t') => ind += 1,
						Some('-' | '=') => ind += 2,
						None => {}
						_ => unreachable!(),
					}
				}

				start += ind;

				parent_lines.push(self.parent_span.start() + start as u32);
				start = end;
			}

			parent_lines
		});

		let span_start_line = parent.line_col(self.parent_span.start());
		let span_end_line = parent.line_col(self.parent_span.end());

		trace!(?span_start_line, ?span_end_line);

		Span::new(
			{
				let location = child.line_col(span.start());
				trace!(?location, linestart = ?translations[location.line as usize], "start linecol");
				translations[location.line as usize] + location.col as u32
			},
			{
				let location = child.line_col(span.end());
				trace!(?location, lineend = ?translations[location.line as usize], "end linecol");
				translations[location.line as usize] + location.col as u32
			},
		)
	}
}
