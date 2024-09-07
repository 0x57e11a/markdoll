use {
	crate::{emit::To, tree::TagContent, MarkDoll},
	alloc::{boxed::Box, vec::Vec},
	hashbrown::HashMap,
};

#[derive(Debug)]
#[allow(
	clippy::type_complexity,
	reason = "type is never mentioned outside of this struct, simple functions"
)]
pub struct TagDefinition {
	/// The tag key
	pub key: &'static str,

	/// Parse the tag. Return None to avoid being placed into the AST and emitting.
	pub parse:
		Option<fn(doll: &mut MarkDoll, arg: Vec<&str>, text: &str) -> Option<Box<dyn TagContent>>>,

	/// Emit the tag content
	pub emit: fn(doll: &mut MarkDoll, to: To, content: &mut Box<dyn TagContent>),
}

impl TagDefinition {
	pub fn parse_ast(doll: &mut MarkDoll, _: Vec<&str>, text: &str) -> Option<Box<dyn TagContent>> {
		if let Ok(ast) = doll.parse(text) {
			Some(Box::new(ast))
		} else {
			doll.ok = false;
			None
		}
	}
}

#[macro_export]
macro_rules! args {
	{
		$doll:ident, $args:ident;

		on_fail($on_fail:expr);

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

							#[allow(clippy::unused_unit, reason = "macro")]
							return $on_fail;
						}
					} else {
						$args.remove(0)
					}
				}
			} else {
				$doll.diag(true, usize::MAX, concat!("argument ", stringify!(person), " required"));

				#[allow(clippy::unused_unit, reason = "macro")]
				return $on_fail;
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

							#[allow(clippy::unused_unit, reason = "macro")]
							return $on_fail;
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
					#[allow(clippy::unused_unit, reason = "macro")]
					return $on_fail;
				}
			} else {}
		};
	};

	{ if [] $true:tt else $false:tt } => { $false };
	{ if [$($tok:ident)+] $true:tt else $false:tt } => { $true };
	{ if [$($tok:ty)+] $true:tt else $false:tt } => { $true };
}

#[derive(Debug)]
pub struct ExtensionSystem {
	pub tags: HashMap<&'static str, TagDefinition>,
}

impl ExtensionSystem {
	pub fn add_tag(&mut self, tag: TagDefinition) {
		self.tags.insert(tag.key, tag);
	}
}
