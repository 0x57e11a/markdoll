/// `code`/`codeblock` tags
pub mod code;
/// `//` tag
pub mod common;
/// `em`/`quote` tags
pub mod formatting;
/// `link`/`def`/`ref` tags
pub mod links;
/// `table`/`tr`/`tc` tags
pub mod table;

use {
	crate::{tree::TagContent, typemap::TypeMap, MarkDoll},
	alloc::{boxed::Box, vec::Vec},
	hashbrown::HashMap,
};

/// the parsing signature tags use
pub type TagParser =
	fn(doll: &mut MarkDoll, args: Vec<&str>, text: &str) -> Option<Box<dyn TagContent>>;
/// the emitting signature tags use for a given `To`
pub type TagEmitter<To> = fn(doll: &mut MarkDoll, to: &mut To, content: &mut Box<dyn TagContent>);

/// defines a tag name, how to parse its contents, and how to emit it
#[derive(Debug, Clone)]
#[allow(
	clippy::type_complexity,
	reason = "type is never mentioned outside of this struct, simple functions"
)]
pub struct TagDefinition {
	/// the tag key
	pub key: &'static str,

	/// parse the tag contents
	///
	/// return None to avoid being placed into the AST and emitting
	pub parse: Option<TagParser>,

	/// emit the tag content
	emitters: TypeMap,
}

impl TagDefinition {
	/// create a new tag definition
	#[must_use]
	pub fn new(key: &'static str, parse: Option<TagParser>) -> TagDefinition {
		Self {
			key,
			parse,
			emitters: TypeMap::default(),
		}
	}

	/// set the emitter on this tag for an emit target
	pub fn set_emitter<To: 'static>(&mut self, emitter: TagEmitter<To>) {
		self.emitters.put(emitter);
	}

	/// set the emitter on this tag for an emit target, and return self for chaining
	#[must_use]
	pub fn with_emitter<To: 'static>(mut self, emitter: TagEmitter<To>) -> Self {
		self.set_emitter(emitter);
		self
	}

	/// retrieve the emitter on this tag for an emit target
	#[must_use]
	pub fn emitter_for<To: 'static>(&self) -> Option<TagEmitter<To>> {
		self.emitters.get_ref().copied()
	}

	/// whether this tag has any emitters
	#[must_use]
	pub fn has_any_emitters(&self) -> bool {
		!self.emitters.is_empty()
	}
}

/// helper macro to parse arguments into variables
///
/// ```rs
/// args! {
///     doll, args; // pass in the markdoll and args
///
///     args(arg1, arg2: usize); // parse required arguments, which may be parsed into another type, if applicable. ex: `(2)`
///     opt_args(oarg1, oarg2: usize); // parse optional arguments, which will be `Some` when present (and parsed into another type, if applicable), or `None` if not. ex: `(2)`
///     flags(flag1, flag2); // parse flags, which will be `true` when present and `false` when not. ex: `(flag2)`
///     props(oarg1, oarg2: usize); // parse named props, which will be `Some` when present (and parsed into another type, if applicable), or `None` if not. ex: `(oarg2=2)`
/// }
/// ```
#[macro_export]
macro_rules! args {
	{
		$doll:ident, $args:ident;

		args($($arg:ident$(: $arg_ty:ty)?),*);
		opt_args($($opt_arg:ident$(: $opt_arg_ty:ty)?),*);
		flags($($flag:ident),*);
		props($($prop:ident$(: $prop_ty:ty)?),*);
	} => {
		let _ = (&$doll, &$args);

		$(
			#[allow(unused, reason = "macro")]
			let mut $arg = if !$args.is_empty() {
				args! {
					if [$($arg_ty)?] {
						#[allow(irrefutable_let_patterns, reason = "macro")]
						if let Ok(value) = $args.remove(0).parse::<$($arg_ty)?>() {
							value
						} else {
							$doll.diag(true, usize::MAX, concat!("arg ", stringify!($arg), " invalid"));

							return None;
						}
					} else {
						$args.remove(0)
					}
				}
			} else {
				$doll.diag(true, usize::MAX, concat!("argument ", stringify!(person), " required"));

				return None;
			};
		)*

		$(
			#[allow(unused, reason = "macro")]
			let mut $opt_arg = if !$args.is_empty() {
				Some(args! {
					if [$($opt_arg_ty)?] {
						#[allow(irrefutable_let_patterns, reason = "macro")]
						if let Ok(value) = $args.remove(0).parse::<$($opt_arg_ty)?>() {
							value
						} else {
							$doll.diag(true, usize::MAX, concat!("arg ", stringify!($opt_arg), " invalid"));

							return None;
						}
					} else {
						$args.remove(0)
					}
				})
			} else {
				None
			};
		)*

		$(let mut $flag = false;)*

		$(
			let mut $prop;

			args! {
				if [$($prop_ty)?] {
					$prop = Option::<$($prop_ty)?>::None;
				} else {
					$prop = Option::<&str>::None;
				}
			}
		)*

		args! {
			if [$($flag)* $($prop)*] {
				#[allow(unused, reason = "macro")]
				let mut retain_ok = true;

				$args.retain(|arg| match *arg {
					$(
						stringify!($flag) => {
							$flag = true;
							false
						}
					)*
					#[allow(unused, reason = "macro")]
					input => args! {
						if [$($prop)*] {
							// parse properties
							if let Some(index) = input.find("=") {
								match &input[..index] {
									$(
										stringify!($prop) => {
											args! {
												if [$($prop_ty)?] {
													if let Ok(value) = input[(index + 1)..].parse::<$($prop_ty)?>() {
														$prop = Some(value);
													} else {
														$doll.diag(true, usize::MAX, concat!("prop ", stringify!(person), " invalid"));

														retain_ok = false;
													}
												} else {
													$prop = Some(&input[(index + 1)..]);
												}
											};
											false
										}
									)*
									_ => true,
								}
							} else {
								true
							}
						} else {
							// no properties
							true
						}
					},
				});

				if !retain_ok {
					return None;
				}
			} else {}
		};
	};

	{ if [] $true:tt else $false:tt } => { $false };
	{ if [$($tok:ident)+] $true:tt else $false:tt } => { $true };
	{ if [$($tok:ty)+] $true:tt else $false:tt } => { $true };
}

/// handles tag definitions
#[derive(Debug)]
pub struct ExtensionSystem {
	/// the tags registered
	pub tags: HashMap<&'static str, TagDefinition>,
}

impl ExtensionSystem {
	/// add a tag
	pub fn add_tag(&mut self, tag: TagDefinition) {
		self.tags.insert(tag.key, tag);
	}

	/// add multiple tags
	pub fn add_tags<const N: usize>(&mut self, tags: [TagDefinition; N]) {
		for tag in tags {
			self.add_tag(tag);
		}
	}
}
