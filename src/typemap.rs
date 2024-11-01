use {
	alloc::{alloc::dealloc, boxed::Box},
	core::{alloc::Layout, any::TypeId, mem::transmute, ptr::drop_in_place},
	hashbrown::HashMap,
};

#[derive(Debug, Clone)]
struct Entry {
	ptr: *mut (),
	drop: unsafe fn(*mut ()),
	layout: Layout,
}

/// hashmap of typeid->value, supporting up to 1 value per type
#[derive(Debug, Clone, Default)]
pub struct TypeMap {
	inner: HashMap<TypeId, Entry>,
}

impl TypeMap {
	/// put a value into the map
	pub fn put<Variant: Sized + Clone + 'static>(&mut self, value: Variant) {
		self.put_boxed(Box::new(value));
	}

	/// put an already-boxed value into the map
	pub fn put_boxed<Variant: Clone + 'static>(&mut self, value: Box<Variant>) {
		self.inner.insert(TypeId::of::<Variant>(), unsafe {
			Entry {
				ptr: Box::into_raw(value).cast(),
				drop: transmute::<unsafe fn(*mut Variant), unsafe fn(*mut ())>(
					drop_in_place::<Variant>,
				),
				layout: Layout::new::<Variant>(),
			}
		});
	}

	/// take a value out of the map
	pub fn remove<Variant: Sized + Clone + 'static>(&mut self) -> Option<Variant> {
		self.remove_boxed().map(|value| *value)
	}

	/// take a boxed value out of the map
	pub fn remove_boxed<Variant: Sized + Clone + 'static>(&mut self) -> Option<Box<Variant>> {
		self.inner
			.remove(&TypeId::of::<Variant>())
			.map(|value| unsafe { Box::from_raw(value.ptr.cast::<Variant>()) })
	}

	/// get a reference to a value in the map
	#[must_use]
	pub fn get_ref<Variant: Clone + 'static>(&self) -> Option<&Variant> {
		self.inner
			.get(&TypeId::of::<Variant>())
			.map(|value| unsafe { &*value.ptr.cast() })
	}

	/// get a mutable reference to a value in the map
	#[must_use]
	pub fn get_mut<Variant: Clone + 'static>(&mut self) -> Option<&mut Variant> {
		self.inner
			.get_mut(&TypeId::of::<Variant>())
			.map(|value| unsafe { &mut *value.ptr.cast() })
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
}

impl Drop for TypeMap {
	fn drop(&mut self) {
		for entry in self.inner.values() {
			unsafe {
				(entry.drop)(entry.ptr);
				dealloc(entry.ptr.cast(), entry.layout);
			}
		}
	}
}
