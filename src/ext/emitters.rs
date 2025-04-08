use {
	crate::{emit::BuiltInEmitters, ext::TagEmitter},
	::core::{any::TypeId, mem::transmute},
	::hashbrown::HashMap,
};

/// hashmap of typeid->value, supporting up to 1 value per type
#[derive(Debug, Clone, Default)]
pub struct Emitters<Variant: EmittersVariant> {
	inner: HashMap<TypeId, (Variant, &'static str)>,
}

impl<Variant: EmittersVariant> Emitters<Variant> {
	/// create a new typemap
	pub fn new() -> Self {
		Self {
			inner: HashMap::with_capacity(1),
		}
	}

	/// put an already-boxed value into the map
	pub fn put<Target: 'static>(&mut self, value: Variant::Specific<Target>) {
		self.inner.insert(
			TypeId::of::<Target>(),
			(
				Variant::specific_to_generic(value),
				::core::any::type_name::<Target>(),
			),
		);
	}

	/// put an emitter into the map
	pub fn with<Target: 'static>(mut self, value: Variant::Specific<Target>) -> Self {
		self.put(value);
		self
	}

	/// take an emitter out of the map
	pub fn remove<Target: 'static>(&mut self) -> Option<Variant::Specific<Target>> {
		self.inner
			.remove(&TypeId::of::<Target>())
			.map(|value| Variant::generic_to_specific(value.0))
	}

	/// get a reference to a value in the map
	#[must_use]
	pub fn get<Target: 'static>(&self) -> Option<Variant::Specific<Target>> {
		self.inner
			.get(&TypeId::of::<Target>())
			.map(|value| Variant::generic_to_specific(value.0))
	}

	/// how many items are in the map
	#[must_use]
	pub fn len(&self) -> usize {
		self.inner.len()
	}

	/// whether the map is empty
	#[must_use]
	pub fn is_empty(&self) -> bool {
		self.inner.is_empty()
	}

	/// the names of the types that have values in this map
	#[must_use]
	pub fn type_names(&self) -> impl Iterator<Item = &'static str> + use<'_, Variant> {
		self.inner.values().map(|value| value.1)
	}
}

/// provides a way for [`Emitters`] to be able to freely transform the variant type between its generic and specific types
#[doc(hidden)]
pub unsafe trait EmittersVariant: Sized + Copy {
	type Specific<T>: Copy;

	fn specific_to_generic<Target: 'static>(specific: Self::Specific<Target>) -> Self;

	fn generic_to_specific<Target: 'static>(generic: Self) -> Self::Specific<Target>;
}

unsafe impl<Ctx> EmittersVariant for TagEmitter<Ctx> {
	type Specific<Target> = TagEmitter<Ctx, Target>;

	fn specific_to_generic<Target: 'static>(specific: Self::Specific<Target>) -> Self {
		unsafe { transmute(specific) }
	}

	fn generic_to_specific<Target: 'static>(generic: Self) -> Self::Specific<Target> {
		unsafe { transmute(generic) }
	}
}

unsafe impl<Ctx> EmittersVariant for BuiltInEmitters<Ctx> {
	type Specific<Target> = BuiltInEmitters<Ctx, Target>;

	fn specific_to_generic<Target: 'static>(specific: Self::Specific<Target>) -> Self {
		unsafe { transmute(specific) }
	}

	fn generic_to_specific<Target: 'static>(generic: Self) -> Self::Specific<Target> {
		unsafe { transmute(generic) }
	}
}
