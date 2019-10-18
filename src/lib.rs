//! Serializable and deserializable trait objects.
//!
//! **[Crates.io](https://crates.io/crates/serde_traitobject) │ [Repo](https://github.com/alecmocatta/serde_traitobject)**
//!
//! This library enables the serialization and deserialization of trait objects so they can be sent between other processes running the same binary.
//!
//! For example, if you have multiple forks of a process, or the same binary running on each of a cluster of machines, this library lets you send trait objects between them.
//!
//! Any trait can be made (de)serializable when made into a trait object by adding this crate's [Serialize] and [Deserialize] traits as supertraits:
//!
//! ```
//! # use serde_derive::{Serialize, Deserialize};
//! trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
//! 	fn my_method(&self);
//! }
//!
//! #[derive(Serialize, Deserialize)]
//! struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
//!
//! // Woohoo, `Message` is now serializable!
//! ```
//!
//! And that's it! The two traits are automatically implemented for all `T: serde::Serialize` and all `T: serde::de::DeserializeOwned`, so as long as all implementors of your trait are themselves serializable then you're good to go.
//!
//! There are two ways to (de)serialize your trait object:
//!  * Apply the `#[serde(with = "serde_traitobject")]` [field attribute](https://serde.rs/attributes.html), which instructs serde to use this crate's [serialize](serialize()) and [deserialize](deserialize()) functions;
//!  * The [Box], [Rc] and [Arc] structs, which are simple wrappers around their stdlib counterparts that automatically handle (de)serialization without needing the above annotation;
//!
//! Additionally, there are several convenience traits implemented that extend their stdlib counterparts:
//!
//!  * [Any], [Debug], [Display], [Error], [Fn], [FnMut], [FnOnce]
//!
//! These are automatically implemented on all implementors of their stdlib counterparts that also implement `serde::Serialize` and `serde::de::DeserializeOwned`.
//!
//! ```
//! # use serde_derive::{Serialize, Deserialize};
//! use std::any::Any;
//! use serde_traitobject as s;
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
//! ```
//!
//! # Security
//!
//! This crate works by wrapping the vtable pointer with [`relative::Vtable`](https://docs.rs/relative) such that it can safely be sent between processes.
//!
//! This approach is not yet secure against malicious actors. However, if we assume non-malicious actors and typical (static or dynamic) linking conditions, then it's not unreasonable to consider it sound.
//!
//! ## Validation
//!
//! Three things are serialized alongside the vtable pointer for the purpose of validation:
//!
//!  * the [`build_id`](https://github.com/alecmocatta/build_id) (128 bits);
//!  * the [`type_id`](https://doc.rust-lang.org/std/intrinsics/fn.type_id.html) of the trait object (64 bits);
//!  * the `type_id` of the concrete type (64 bits).
//!
//! At some point in Rust's future, I think it would be great if the latter could be used to safely look up and create a trait object. As it is, that functionality doesn't exist yet, so what this crate does instead is serialize the vtable pointer (relative to a static base), and do as much validity checking as it reasonably can before it can be used and potentially invoke UB.
//!
//! The first two are [checked for validity](https://github.com/alecmocatta/relative/blob/dae206663a09b9c0c4b3012c528b0e9c063df742/src/lib.rs#L457-L474) before usage of the vtable pointer. The `build_id` ensures that the vtable pointer came from an invocation of an identically laid out binary<sup>1</sup>. The `type_id` ensures that the trait object being deserialized is the same type as the trait object that was serialized. They ensure that under non-malicious conditions, attempts to deserialize invalid data return an error rather than UB. The `type_id` of the concrete type is used as a [sanity check](https://github.com/alecmocatta/serde_traitobject/blob/b20d74e183063e7d49aff2eabc9dcd5bc26d7c07/src/lib.rs#L469) that panics if it differs from the `type_id` of the concrete type to be deserialized.
//!
//! Regarding collisions, the 128 bit `build_id` colliding is sufficiently unlikely that it can be relied upon to never occur. The 64 bit `type_id` colliding is possible, see [rust-lang/rust#10389](https://github.com/rust-lang/rust/issues/10389), though exceedingly unlikely to occur in practise.
//!
//! The vtable pointer is (de)serialized as a usize relative to the vtable pointer of [this static trait object](https://github.com/alecmocatta/relative/blob/dae206663a09b9c0c4b3012c528b0e9c063df742/src/lib.rs#L90). This enables it to work under typical dynamic linking conditions, where the absolute vtable addresses can differ across invocations of the same binary, but relative addresses remain constant.
//!
//! All together this leaves, as far as I'm aware, three soundness holes:
//!
//!  * A malicious user with a copy of the binary could trivially craft a `build_id` and `type_id` that pass validation and gives them control of where to jump to.
//!  * Data corruption of the serialized vtable pointer but not the `build_id` or `type_id` used for validation, resulting in a jump to an arbitrary address. This could be rectified in a future version of this library by using a cipher to make it vanishingly unlikely for corruptions to affect only the vtable pointer, by mixing the vtable pointer and validation components upon (de)serialization.
//!  * Dynamic linking conditions where the relative addresses (vtable - static vtable) are different across different invocations of the same binary. I'm sure this is possible, but it's not a scenario I've encountered so I can't speak to its commonness.
//!
//! <sup>1</sup>I don't think this requirement is strictly necessary, as the `type_id` should include all information that could affect soundness (trait methods, calling conventions, etc), but it's included in case that doesn't hold in practise; to provide a more helpful error message; and to reduce the likelihood of collisions.
//!
//! # Note
//!
//! This crate currently requires Rust nightly.

#![doc(html_root_url = "https://docs.rs/serde_traitobject/0.2.2")]
#![feature(
	arbitrary_self_types,
	coerce_unsized,
	fn_traits,
	specialization,
	unboxed_closures,
	unsize
)]
#![warn(
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	trivial_casts,
	trivial_numeric_casts,
	unused_import_braces,
	unused_qualifications,
	unused_results,
	clippy::pedantic
)] // from https://github.com/rust-unofficial/patterns/blob/master/anti_patterns/deny-warnings.md
#![allow(where_clauses_object_safety, clippy::must_use_candidate)]

mod convenience;

use metatype::{type_coerce, type_id};
use relative::Vtable;
use serde::ser::SerializeTuple;
use std::{any::type_name, boxed, fmt, marker, ptr::NonNull};

pub use convenience::*;

/// Any trait with this as a supertrait can be serialized as a trait object.
///
/// It is automatically implemented for all `T: serde::Serialize`, i.e. you should not implement it manually.
///
/// To use, simply add it as a supertrait to your trait:
/// ```
/// use serde_derive::{Serialize, Deserialize};
///
/// trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// 	fn my_method(&self);
/// }
/// ```
///
/// Now your trait object is serializable!
/// ```
/// # use serde_derive::{Serialize, Deserialize};
/// #
/// # trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// # 	fn my_method(&self);
/// # }
/// #[derive(Serialize, Deserialize)]
/// struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
///
/// // Woohoo, `Message` is now serializable!
/// ```
///
/// Any implementers of `MyTrait` would now have to themselves implement `serde::Serialize` and `serde::de::DeserializeOwned`. This would typically be through `serde_derive`, like:
/// ```
/// # use serde_derive::{Serialize, Deserialize};
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
/// ```
pub trait Serialize: serialize::Sealed {}
impl<T: serde::ser::Serialize + ?Sized> Serialize for T {}

/// Any trait with this as a supertrait can be deserialized as a boxed trait object.
///
/// It is automatically implemented for all `T: serde::de::DeserializeOwned`, i.e. you should not implement it manually.
///
/// To use, simply add it as a supertrait to your trait:
/// ```
/// use serde_derive::{Serialize, Deserialize};
///
/// trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// 	fn my_method(&self);
/// }
/// ```
///
/// Now your trait object is serializable!
/// ```
/// # use serde_derive::{Serialize, Deserialize};
/// #
/// # trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
/// # 	fn my_method(&self);
/// # }
/// #[derive(Serialize, Deserialize)]
/// struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);
///
/// // Woohoo, `Message` is now serializable!
/// ```
///
/// Any implementers of `MyTrait` would now have to themselves implement `serde::Serialize` and `serde::de::DeserializeOwned`. This would typically be through `serde_derive`, like:
/// ```
/// # use serde_derive::{Serialize, Deserialize};
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
/// ```
pub trait Deserialize: deserialize::Sealed {}
impl<T: serde::de::DeserializeOwned> Deserialize for T {}
impl Deserialize for str {}
impl<T: serde::de::DeserializeOwned> Deserialize for [T] {}

mod serialize {
	use super::*;

	pub trait Sealed: erased_serde::Serialize {
		fn serialize_sized<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized;

		#[inline]
		fn type_id(&self) -> u64
		where
			Self: 'static,
		{
			type_id::<Self>()
		}
	}

	impl<T: serde::ser::Serialize + ?Sized> Sealed for T {
		#[inline]
		default fn serialize_sized<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized,
		{
			let _ = serializer;
			unreachable!()
		}
	}

	impl<T: serde::ser::Serialize> Sealed for T {
		#[inline]
		fn serialize_sized<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: serde::Serializer,
			Self: Sized,
		{
			serde::ser::Serialize::serialize(self, serializer)
		}
	}
}

mod deserialize {
	use super::*;

	pub trait Sealed {
		fn deserialize_erased(
			self: *const Self, deserializer: &mut dyn erased_serde::Deserializer,
		) -> Result<NonNull<()>, erased_serde::Error> {
			let _ = deserializer;
			unreachable!()
		}

		fn deserialize_box<'de, D>(deserializer: D) -> Result<boxed::Box<Self>, D::Error>
		where
			D: serde::Deserializer<'de>,
			Self: Sized,
		{
			let _ = deserializer;
			unreachable!()
		}

		#[inline]
		fn type_id(self: *const Self) -> u64
		where
			Self: 'static,
		{
			type_id::<Self>()
		}
	}

	impl<T: serde::de::DeserializeOwned> Sealed for T {
		#[inline]
		fn deserialize_erased(
			self: *const Self, deserializer: &mut dyn erased_serde::Deserializer,
		) -> Result<NonNull<()>, erased_serde::Error> {
			erased_serde::deserialize::<Self>(deserializer)
				.map(|x| NonNull::new(boxed::Box::into_raw(boxed::Box::new(x)).cast()).unwrap())
		}

		#[inline]
		fn deserialize_box<'de, D>(deserializer: D) -> Result<boxed::Box<Self>, D::Error>
		where
			D: serde::Deserializer<'de>,
			Self: Sized,
		{
			serde::de::Deserialize::deserialize(deserializer).map(boxed::Box::new)
		}
	}

	impl Sealed for str {}
	impl<T: serde::de::DeserializeOwned> Sealed for [T] {}

	/// Rust currently doesn't support returning Self traitobjects from
	/// traitobject methods. Work around that by returning a thin pointer and
	/// fattening it.
	#[allow(clippy::module_name_repetitions)]
	#[inline]
	pub fn deserialize_erased<T: ?Sized>(
		self_: *const T, deserializer: &mut dyn erased_serde::Deserializer,
	) -> Result<boxed::Box<T>, erased_serde::Error>
	where
		T: Sealed,
	{
		self_.deserialize_erased(deserializer).map(|raw| {
			let object: *mut T = metatype::Type::fatten(raw.as_ptr(), metatype::Type::meta(self_));
			unsafe { boxed::Box::from_raw(object) }
		})
	}
}

/// Using a struct + trait to leverage specialisation to respectively handle
/// concrete, slices and traitobjects.
struct Serializer<T: Serialize + ?Sized + 'static>(marker::PhantomData<fn(T)>);
trait SerializerTrait<T: Serialize + ?Sized> {
	fn serialize<S>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer;
}
impl<T: Serialize> SerializerTrait<T> for Serializer<T> {
	#[inline]
	fn serialize<S>(t: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		t.serialize_sized(serializer)
	}
}
impl SerializerTrait<str> for Serializer<str> {
	#[inline]
	fn serialize<S>(t: &str, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		serde::ser::Serialize::serialize(t, serializer)
	}
}
impl<T: serde::ser::Serialize> SerializerTrait<[T]> for Serializer<[T]> {
	#[inline]
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
			let trait_object: metatype::TraitObject = type_coerce(metatype::Type::meta(t));
			trait_object.vtable
		} else {
			panic!()
		};
		let mut tup = serializer.serialize_tuple(3)?;
		// We're making the assumption that the vtable is positioned the same
		// relative to the base vtable in every invocation, through e.g. being
		// in the same segment, or the binary being statically linked.
		//
		// See the [`relative`](https://github.com/alecmocatta/relative) crate
		// for more information.
		tup.serialize_element::<Vtable<T>>(&unsafe { Vtable::<T>::from(vtable) })?;
		tup.serialize_element::<u64>(&<T as serialize::Sealed>::type_id(&t))?;
		tup.serialize_element::<SerializeErased<T>>(&SerializeErased(t))?;
		tup.end()
	}
}
struct SerializeErased<'a, T: Serialize + ?Sized + 'a>(&'a T);
impl<'a, T: Serialize + ?Sized> serde::ser::Serialize for SerializeErased<'a, T> {
	#[inline]
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		erased_serde::serialize(self.0, serializer)
	}
}

/// Using a struct + trait to leverage specialisation to respectively handle
/// concrete, slices and traitobjects.
struct Deserializer<T: Deserialize + ?Sized + 'static>(marker::PhantomData<T>);
trait DeserializerTrait<T: Deserialize + ?Sized> {
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<T>, D::Error>
	where
		D: serde::Deserializer<'de>;
}
impl<T: Deserialize> DeserializerTrait<T> for Deserializer<T> {
	#[inline]
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<T>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		<T as deserialize::Sealed>::deserialize_box(deserializer)
	}
}
impl DeserializerTrait<str> for Deserializer<str> {
	#[inline]
	fn deserialize<'de, D>(deserializer: D) -> Result<boxed::Box<str>, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		serde::de::Deserialize::deserialize(deserializer)
	}
}
impl<T: serde::de::DeserializeOwned> DeserializerTrait<[T]> for Deserializer<[T]> {
	#[inline]
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
				write!(formatter, "a \"{}\" trait object", type_name::<T>())
			}
			#[inline]
			fn visit_seq<A>(self, mut seq: A) -> Result<boxed::Box<T>, A::Error>
			where
				A: serde::de::SeqAccess<'de>,
			{
				let t0: Vtable<T> = match seq.next_element()? {
					Some(value) => value,
					None => return Err(serde::de::Error::invalid_length(0, &self)),
				};
				let t1: u64 = match seq.next_element()? {
					Some(value) => value,
					None => return Err(serde::de::Error::invalid_length(1, &self)),
				};
				let meta = metatype::TraitObject { vtable: t0.to() };
				let object: *const T = metatype::Type::dangling(type_coerce(meta)).as_ptr();
				assert_eq!(t1, object.type_id(), "Deserializing the trait object \"{}\" failed in a way that should never happen. Please file an issue! https://github.com/alecmocatta/serde_traitobject/issues/new", type_name::<T>());
				let t2: boxed::Box<T> = match seq.next_element_seed(DeserializeErased(object))? {
					Some(value) => value,
					None => return Err(serde::de::Error::invalid_length(2, &self)),
				};
				Ok(t2)
			}
		}
		deserializer.deserialize_tuple(3, Visitor(marker::PhantomData))
	}
}
struct DeserializeErased<T: Deserialize + ?Sized>(*const T);
impl<'de, T: Deserialize + ?Sized> serde::de::DeserializeSeed<'de> for DeserializeErased<T> {
	type Value = boxed::Box<T>;

	#[inline]
	fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
	where
		D: serde::de::Deserializer<'de>,
	{
		let deserializer = &mut erased_serde::Deserializer::erase(deserializer);
		deserialize::deserialize_erased(self.0, deserializer).map_err(serde::de::Error::custom)
	}
}

/// Serialize a value by reference.
///
/// This is intended to enable:
/// ```
/// # use serde_derive::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
/// 	#[serde(with = "serde_traitobject")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
/// ```
///
/// Or, alternatively, if only Serialize is desired:
/// ```
/// # use serde_derive::{Serialize, Deserialize};
///
/// #[derive(Serialize)]
/// struct MyStruct {
/// 	#[serde(serialize_with = "serde_traitobject::serialize")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
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
/// # use serde_derive::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct MyStruct {
/// 	#[serde(with = "serde_traitobject")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
/// ```
///
/// Or, alternatively, if only Deserialize is desired:
/// ```
/// # use serde_derive::{Serialize, Deserialize};
///
/// #[derive(Deserialize)]
/// struct MyStruct {
/// 	#[serde(deserialize_with = "serde_traitobject::deserialize")]
/// 	field: Box<dyn serde_traitobject::Any>,
/// }
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
