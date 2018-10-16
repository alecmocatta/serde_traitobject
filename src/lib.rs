//! Serializable trait objects.
//!
//! **[Crates.io](https://crates.io/crates/serde_traitobject) │ [Repo](https://github.com/alecmocatta/serde_traitobject)**
//!
//! This library enables the serialization of trait objects such that they can be sent between other processes running the same binary.
//!
//! For example, if you have multiple forks of a process, or the same binary running on each of a cluster of machines, this library would help you to send trait objects between them.
//!
//! The heart of this crate is the [Serialize] and [Deserialize] traits. They are automatically implemented for all `T: serde::Serialize` and all `T: serde::de::DeserializeOwned` respectively.
//!
//! Any trait can be made (de)serializable when made into a trait object by simply adding them as supertraits:
//!
//! ```
//! # extern crate serde;
//! #[macro_use]
//! extern crate serde_derive;
//! extern crate serde_traitobject;
//!
//! # fn main() {
//! trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
//! 	fn my_method(&self);
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
//!
//! // Woohoo, `Message` is now serializable!
//! # }
//! ```
//!
//! There are two ways to use serde_traitobject to handle the (de)serialization:
//!  * `#[serde(with = "serde_traitobject")]` [field attribute](https://serde.rs/attributes.html) on a boxed trait object, which instructs serde to use the [serialize](serialize()) and [deserialize](deserialize()) functions;
//!  * The [Box], [Rc] and [Arc] structs, which are simple wrappers around their stdlib counterparts that automatically handle (de)serialization without needing the above annotation;
//!
//! Additionally, there are several convenience traits implemented that extend their stdlib counterparts:
//!
//!  * [Any], [Debug], [Display], [Error], [Fn], [FnBox], [FnMut], [FnOnce]
//!
//! These are automatically implemented on all (de)serializable implementors of their stdlib counterparts:
//!
//! ```
//! #[macro_use]
//! extern crate serde_derive;
//! extern crate serde_json;
//! extern crate serde_traitobject as s;
//! # extern crate serde;
//!
//! # fn main() {
//! use std::any::Any;
//!
//! #[derive(Serialize, Deserialize, Debug)]
//! struct MyStruct {
//! 	foo: String,
//! 	bar: usize,
//! }
//!
//! let my_struct = MyStruct {
//! 	foo: String::from("abc"),
//! 	bar: 123,
//! };
//!
//! let erased: s::Box<dyn s::Any> = s::Box::new(my_struct);
//!
//! let serialized = serde_json::to_string(&erased).unwrap();
//! let deserialized: s::Box<dyn s::Any> = serde_json::from_str(&serialized).unwrap();
//!
//! let downcast: Box<MyStruct> = Box::<dyn Any>::downcast(deserialized.into_any()).unwrap();
//!
//! println!("{:?}", downcast);
//! # assert_eq!(format!("{:?}", downcast), "MyStruct { foo: \"abc\", bar: 123 }");
//! // MyStruct { foo: "abc", bar: 123 }
//! # }
//! ```
//!
//! # Note
//!
//! This crate works by wrapping the vtable pointer with [relative::Vtable](https://docs.rs/relative) such that it can safely be sent between processes.
//!
//! This currently requires Rust nightly.

#![doc(html_root_url = "https://docs.rs/serde_traitobject/0.1.1")]
#![feature(
	unboxed_closures,
	fn_traits,
	core_intrinsics,
	coerce_unsized,
	unsize,
	specialization,
	trivial_bounds,
	fnbox
)]
#![warn(
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	trivial_numeric_casts,
	unused_extern_crates,
	unused_import_braces,
	unused_qualifications,
	unused_results,
	clippy::pedantic
)] // from https://github.com/rust-unofficial/patterns/blob/master/anti_patterns/deny-warnings.md
#![allow(
	where_clauses_object_safety,
	clippy::inline_always,
	clippy::doc_markdown
)]

extern crate erased_serde;
extern crate metatype;
extern crate relative;
extern crate serde;

mod convenience;

use relative::Vtable;
use serde::ser::SerializeTuple;
use std::{boxed, fmt, intrinsics, marker, mem, ptr};

pub use convenience::*;

/// Any trait with this as a supertrait can be serialized as a trait object.
///
/// It is automatically implemented for all `T: serde::Serialize`, i.e. you should not implement it manually.
///
/// To use, simply add it as a supertrait to your trait:
/// ```
/// # extern crate serde;
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate serde_traitobject;
///
/// # fn main() {
/// trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// 	fn my_method(&self);
/// }
/// # }
/// ```
///
/// Now your trait object is serializable!
/// ```
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate serde_traitobject;
/// #
/// # fn main() {
/// # trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// # 	fn my_method(&self);
/// # }
/// #[derive(Serialize, Deserialize)]
/// struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
///
/// // Woohoo, `Message` is now serializable!
/// # }
/// ```
///
/// Any implementers of `MyTrait` would now have to themselves implement `serde::Serialize` and `serde::de::DeserializeOwned`. This would typically be through serde_derive, like:
/// ```
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate serde_traitobject;
/// # fn main() {
/// # trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// # 	fn my_method(&self);
/// # }
/// # #[derive(Serialize, Deserialize)]
/// # struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
/// 	foo: String,
/// }
///
/// impl MyTrait for MyStruct {
/// 	fn my_method(&self) {
/// 		println!("foo: {}", self.foo);
/// 	}
/// }
/// # }
/// ```
pub trait Serialize: serialize::Sealed {}
impl<T: serde::ser::Serialize + ?Sized> Serialize for T {}

/// Any trait with this as a supertrait can be deserialized as a boxed trait object.
///
/// It is automatically implemented for all `T: serde::de::DeserializeOwned`, i.e. you should not implement it manually.
///
/// To use, simply add it as a supertrait to your trait:
/// ```
/// # extern crate serde;
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate serde_traitobject;
///
/// # fn main() {
/// trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// 	fn my_method(&self);
/// }
/// # }
/// ```
///
/// Now your trait object is serializable!
/// ```
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate serde_traitobject;
/// #
/// # fn main() {
/// # trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// # 	fn my_method(&self);
/// # }
/// #[derive(Serialize, Deserialize)]
/// struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
///
/// // Woohoo, `Message` is now serializable!
/// # }
/// ```
///
/// Any implementers of `MyTrait` would now have to themselves implement `serde::Serialize` and `serde::de::DeserializeOwned`. This would typically be through serde_derive, like:
/// ```
/// # extern crate serde;
/// # #[macro_use]
/// # extern crate serde_derive;
/// # extern crate serde_traitobject;
/// # fn main() {
/// # trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// # 	fn my_method(&self);
/// # }
/// # #[derive(Serialize, Deserialize)]
/// # struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
/// 	foo: String,
/// }
///
/// impl MyTrait for MyStruct {
/// 	fn my_method(&self) {
/// 		println!("foo: {}", self.foo);
/// 	}
/// }
/// # }
/// ```
pub trait Deserialize: deserialize::Sealed {}
impl<T: serde::de::DeserializeOwned> Deserialize for T {}
impl Deserialize for str {}
impl<T: serde::de::DeserializeOwned> Deserialize for [T] {}

mod serialize {
	use super::*;
	pub trait Sealed: erased_serde::Serialize {
		fn serialize_sized<S>(&self, S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized;
	}
	impl<T: serde::ser::Serialize + ?Sized> Sealed for T {
		#[inline(always)]
		default fn serialize_sized<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized,
		{
			unreachable!()
		}
	}
	impl<T: serde::ser::Serialize> Sealed for T {
		#[inline(always)]
		fn serialize_sized<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized,
		{
			serde::ser::Serialize::serialize(self, serializer)
		}
	}
	impl Sealed for str {
		#[inline(always)]
		fn serialize_sized<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized,
		{
			unreachable!()
		}
	}
	impl<T: serde::ser::Serialize> Sealed for [T] {
		#[inline(always)]
		fn serialize_sized<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized,
		{
			unreachable!()
		}
	}
}
mod deserialize {
	use super::*;
	pub trait Sealed {
		/// Unsafe as it `ptr::write`s into `&mut self`, assuming it to be uninitialized
		unsafe fn deserialize_erased(
			&mut self, deserializer: &mut erased_serde::Deserializer,
		) -> Result<(), erased_serde::Error>;
		fn deserialize_box<'de, D>(deserializer: D) -> Result<boxed::Box<Self>, D::Error>
		where
			D: serde::Deserializer<'de>,
			Self: Sized;
	}
	impl<T: serde::de::DeserializeOwned> Sealed for T {
		#[inline(always)]
		unsafe fn deserialize_erased(
			&mut self, deserializer: &mut erased_serde::Deserializer,
		) -> Result<(), erased_serde::Error> {
			erased_serde::deserialize(deserializer).map(|x| ptr::write(self, x))
		}
		#[inline(always)]
		fn deserialize_box<'de, D>(deserializer: D) -> Result<boxed::Box<Self>, D::Error>
		where
			D: serde::Deserializer<'de>,
			Self: Sized,
		{
			serde::de::Deserialize::deserialize(deserializer).map(boxed::Box::new)
		}
	}
	impl Sealed for str {
		#[inline(always)]
		unsafe fn deserialize_erased(
			&mut self, _deserializer: &mut erased_serde::Deserializer,
		) -> Result<(), erased_serde::Error> {
			unreachable!()
		}
		#[inline(always)]
		fn deserialize_box<'de, D>(_deserializer: D) -> Result<boxed::Box<Self>, D::Error>
		where
			D: serde::Deserializer<'de>,
			Self: Sized,
		{
			unreachable!()
		}
	}
	impl<T: serde::de::DeserializeOwned> Sealed for [T] {
		#[inline(always)]
		unsafe fn deserialize_erased(
			&mut self, _deserializer: &mut erased_serde::Deserializer,
		) -> Result<(), erased_serde::Error> {
			unreachable!()
		}
		#[inline(always)]
		fn deserialize_box<'de, D>(_deserializer: D) -> Result<boxed::Box<Self>, D::Error>
		where
			D: serde::Deserializer<'de>,
			Self: Sized,
		{
			unreachable!()
		}
	}
}

/// Using a struct + trait to leverage specialisation to respectively handle concrete, slices and traitobjects.
struct Serializer<T: Serialize + ?Sized + 'static>(marker::PhantomData<fn(T)>);
trait SerializerTrait<T: Serialize + ?Sized> {
	fn serialize<S>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer;
}
impl<T: Serialize> SerializerTrait<T> for Serializer<T> {
	#[inline(always)]
	fn serialize<S>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		t.serialize_sized(serializer)
	}
}
impl SerializerTrait<str> for Serializer<str> {
	#[inline(always)]
	fn serialize<S>(t: &str, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::ser::Serialize::serialize(t, serializer)
	}
}
impl<T: serde::ser::Serialize> SerializerTrait<[T]> for Serializer<[T]> {
	#[inline(always)]
	fn serialize<S>(t: &[T], serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::ser::Serialize::serialize(t, serializer)
	}
}
impl<T: Serialize + ?Sized + 'static> SerializerTrait<T> for Serializer<T> {
	#[inline]
	default fn serialize<S>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		let vtable = if let metatype::MetaType::TraitObject = metatype::Type::meta_type(t) {
			let trait_object: metatype::TraitObject =
				unsafe { mem::transmute_copy(&metatype::Type::meta(t)) }; // https://github.com/rust-lang/rust/issues/50318
			trait_object.vtable
		} else {
			panic!()
		};
		let mut tup = serializer.serialize_tuple(2)?;
		tup.serialize_element::<Vtable<T>>(&unsafe { Vtable::<T>::from(vtable) })?;
		tup.serialize_element::<SerializeErased<T>>(&SerializeErased(t))?;
		tup.end()
	}
}
struct SerializeErased<'a, T: Serialize + ?Sized + 'a>(&'a T);
impl<'a, T: Serialize + ?Sized> serde::ser::Serialize for SerializeErased<'a, T> {
	#[inline(always)]
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		erased_serde::serialize(self.0, serializer)
	}
}

/// Using a struct + trait to leverage specialisation to respectively handle concrete, slices and traitobjects.
struct Deserializer<T: Deserialize + ?Sized + 'static>(marker::PhantomData<T>);
trait DeserializerTrait<T: Deserialize + ?Sized> {
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<T>, D::Error>
	where
		D: serde::Deserializer<'de>;
}
impl<T: Deserialize> DeserializerTrait<T> for Deserializer<T> {
	#[inline(always)]
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<T>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<T as deserialize::Sealed>::deserialize_box(deserializer)
	}
}
impl DeserializerTrait<str> for Deserializer<str> {
	#[inline(always)]
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<str>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::de::Deserialize::deserialize(deserializer)
	}
}
impl<T: serde::de::DeserializeOwned> DeserializerTrait<[T]> for Deserializer<[T]> {
	#[inline(always)]
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<[T]>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::de::Deserialize::deserialize(deserializer)
	}
}
impl<T: Deserialize + ?Sized + 'static> DeserializerTrait<T> for Deserializer<T> {
	#[inline]
	default fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<T>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		struct Visitor<T: Deserialize + ?Sized>(marker::PhantomData<T>);
		impl<'de, T: Deserialize + ?Sized + 'static> serde::de::Visitor<'de> for Visitor<T> {
			type Value = boxed::Box<T>;
			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				write!(formatter, "a {} trait object", unsafe {
					intrinsics::type_name::<T>()
				})
			}
			#[inline(always)]
			fn visit_seq<A>(self, mut seq: A) -> Result<boxed::Box<T>, A::Error>
			where
				A: serde::de::SeqAccess<'de>,
			{
				let t0: Vtable<T> = match seq.next_element()? {
					Some(value) => value,
					None => return Err(serde::de::Error::invalid_length(0, &self)),
				};
				let object: boxed::Box<T> = unsafe {
					metatype::Type::uninitialized_box(mem::transmute_copy(&metatype::TraitObject {
						vtable: t0.to(),
					})) // https://github.com/rust-lang/rust/issues/50318
				};
				let t1: boxed::Box<T> = match seq.next_element_seed(DeserializeErased(object))? {
					Some(value) => value,
					None => return Err(serde::de::Error::invalid_length(1, &self)),
				};
				Ok(t1)
			}
		}
		deserializer.deserialize_tuple(2, Visitor(marker::PhantomData))
	}
}
struct DeserializeErased<T: Deserialize + ?Sized>(boxed::Box<T>);
impl<'de, T: Deserialize + ?Sized> serde::de::DeserializeSeed<'de> for DeserializeErased<T> {
	type Value = boxed::Box<T>;
	#[inline(always)]
	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: serde::de::Deserializer<'de>,
	{
		let mut x = self.0;
		unsafe {
			(&mut *x).deserialize_erased(&mut erased_serde::Deserializer::erase(deserializer))
		}
		.map(|()| x)
		.map_err(serde::de::Error::custom)
	}
}

/// Serialize a value by reference.
///
/// This is intended to enable:
/// ```
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate serde_traitobject;
///
/// # fn main() {
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
/// 	#[serde(with = "serde_traitobject")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
/// # }
/// ```
///
/// Or, alternatively, if only Serialize is desired:
/// ```
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate serde_traitobject;
///
/// # fn main() {
/// #[derive(Serialize)]
/// struct MyStruct {
/// 	#[serde(serialize_with = "serde_traitobject::serialize")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
/// # }
/// ```
pub fn serialize<T: Serialize + ?Sized + 'static, B: AsRef<T> + ?Sized, S>(
	t: &B, serializer: S,
) -> Result<S::Ok, S::Error>
where
	S: serde::Serializer,
{
	Serializer::<T>::serialize(t.as_ref(), serializer)
}

/// Deserialize a value `T` into `B` where `Box<T>: Into<B>`.
///
/// This is intended to enable:
/// ```
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate serde_traitobject;
///
/// # fn main() {
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
/// 	#[serde(with = "serde_traitobject")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
/// # }
/// ```
///
/// Or, alternatively, if only Deserialize is desired:
/// ```
/// #[macro_use]
/// extern crate serde_derive;
/// extern crate serde_traitobject;
///
/// # fn main() {
/// #[derive(Deserialize)]
/// struct MyStruct {
/// 	#[serde(deserialize_with = "serde_traitobject::deserialize")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
/// # }
/// ```
pub fn deserialize<'de, T: Deserialize + ?Sized + 'static, B, D>(
	deserializer: D,
) -> Result<B, D::Error>
where
	D: serde::Deserializer<'de>,
	boxed::Box<T>: Into<B>,
{
	Deserializer::<T>::deserialize(deserializer).map(<boxed::Box<T> as Into<B>>::into)
}
