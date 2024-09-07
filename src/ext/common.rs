use crate::ext::TagDefinition;

/// `//` tag
///
/// exclude content from the output
///
/// # content
///
/// anything
pub const COMMENT_TAG: TagDefinition = TagDefinition {
	key: "//",
	parse: Some(|_, _, _| None),
	emit: |_, _, _| {},
};