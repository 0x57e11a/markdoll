use crate::ext::{Emitters, TagDefinition, TagEmitter};

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
	pub fn tag<Ctx: 'static>() -> TagDefinition {
		TagDefinition {
			key: "//",
			parse: |_, _, _, _| None,
			emitters: Emitters::<TagEmitter>::new(),
		}
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx: 'static>() -> impl IntoIterator<Item = TagDefinition> {
	[comment::tag::<Ctx>()]
}
