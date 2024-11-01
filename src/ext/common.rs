use crate::ext::TagDefinition;

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
		TagDefinition::new("//", Some(|_, _, _| None))
	}
}

/// all of this module's tags
#[must_use]
pub fn tags() -> [TagDefinition; 1] {
	[comment::tag()]
}
