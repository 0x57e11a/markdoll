//! `//` tags

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
	pub fn tag<Ctx>() -> TagDefinition<Ctx> {
		TagDefinition {
			key: "//",
			parse: |_, _, _, _| None,
			emitters: Emitters::<TagEmitter<Ctx>>::new(),
		}
	}
}

/// all of this module's tags
#[must_use]
pub fn tags<Ctx>() -> impl IntoIterator<Item = TagDefinition<Ctx>> {
	[comment::tag::<Ctx>()]
}
