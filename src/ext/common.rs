use {
	crate::{
		diagnostics::DiagnosticKind,
		ext::{Emitters, TagDefinition, TagEmitter},
	},
	::miette::LabeledSpan,
};

/// `//` tag
///
/// exclude content from the output
///
/// # content
///
/// anything
pub mod comment {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition {
			key: "//",
			parse: |_, _, _, _| None,
			emitters: Emitters::<TagEmitter>::new(),
		}
	}
}

/// `!` tag
///
/// always error
pub mod error {
	use super::*;

	/// the tag
	#[must_use]
	pub fn tag() -> TagDefinition {
		TagDefinition {
			key: "!",
			parse: |doll, _, text, tag_span| {
				let (at, context) = doll.resolve_span(text.span());
				let mut labels = vec![LabeledSpan::new_primary_with_span(
					Some("error message".to_string()),
					at,
				)];
				labels.extend(context.into_iter());
				doll.diag(DiagnosticKind::Tag(Box::new(::miette::diagnostic!(
					severity = ::miette::Severity::Error,
					code = "markdoll::ext::common::error",
					labels = labels,
					"{text}",
					text = &*text,
				))));
				None
			},
			emitters: Emitters::<TagEmitter>::new(),
		}
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> [TagDefinition; 2] {
	[comment::tag(), error::tag()]
}
