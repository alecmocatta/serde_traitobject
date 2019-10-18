# serde_traitobject

[![Crates.io](https://img.shields.io/crates/v/serde_traitobject.svg?maxAge=86400)](https://crates.io/crates/serde_traitobject)
[![MIT / Apache 2.0 licensed](https://img.shields.io/crates/l/serde_traitobject.svg?maxAge=2592000)](#License)
[![Build Status](https://dev.azure.com/alecmocatta/serde_traitobject/_apis/build/status/tests?branchName=master)](https://dev.azure.com/alecmocatta/serde_traitobject/_build/latest?branchName=master)

[Docs](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/)

**Serializable and deserializable trait objects.**

This library enables the serialization and deserialization of trait objects so they can be sent between other processes running the same binary.

For example, if you have multiple forks of a process, or the same binary running on each of a cluster of machines, this library lets you send trait objects between them.

Any trait can be made (de)serializable when made into a trait object by adding this crate's [Serialize](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Serialize.html) and [Deserialize](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Deserialize.html) traits as supertraits:

```rust
trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
	fn my_method(&self);
}

#[derive(Serialize, Deserialize)]
struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);

// Woohoo, `Message` is now serializable!
```

And that's it! The two traits are automatically implemented for all `T: serde::Serialize` and all `T: serde::de::DeserializeOwned`, so as long as all implementors of your trait are themselves serializable then you're good to go.

There are two ways to (de)serialize your trait object:
 * Apply the `#[serde(with = "serde_traitobject")]` [field attribute](https://serde.rs/attributes.html), which instructs serde to use this crate's [serialize](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/fn.serialize.html) and [deserialize](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/fn.deserialize.html) functions;
 * The [Box](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/struct.Box.html), [Rc](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/struct.Rc.html) and [Arc](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/struct.Arc.html) structs, which are simple wrappers around their stdlib counterparts that automatically handle (de)serialization without needing the above annotation;

Additionally, there are several convenience traits implemented that extend their stdlib counterparts:

 * [Any](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Any.html), [Debug](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Debug.html), [Display](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Display.html), [Error](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Error.html), [Fn](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.Fn.html), [FnMut](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.FnMut.html), [FnOnce](https://docs.rs/serde_traitobject/0.2.2/serde_traitobject/trait.FnOnce.html)

These are automatically implemented on all implementors of their stdlib counterparts that also implement `serde::Serialize` and `serde::de::DeserializeOwned`.

```rust
use std::any::Any;
use serde_traitobject as s;

#[derive(Serialize, Deserialize, Debug)]
struct MyStruct {
	foo: String,
	bar: usize,
}

let my_struct = MyStruct {
	foo: String::from("abc"),
	bar: 123,
};

let erased: s::Box<dyn s::Any> = s::Box::new(my_struct);

let serialized = serde_json::to_string(&erased).unwrap();
let deserialized: s::Box<dyn s::Any> = serde_json::from_str(&serialized).unwrap();

let downcast: Box<MyStruct> = Box::<dyn Any>::downcast(deserialized.into_any()).unwrap();

println!("{:?}", downcast);
// MyStruct { foo: "abc", bar: 123 }
```

## Security

This crate works by wrapping the vtable pointer with [`relative::Vtable`](https://docs.rs/relative) such that it can safely be sent between processes.

This approach is not yet secure against malicious actors. However, if we assume non-malicious actors and typical (static or dynamic) linking conditions, then it's not unreasonable to consider it sound.

### Validation

Three things are serialized alongside the vtable pointer for the purpose of validation:

 * the [`build_id`](https://github.com/alecmocatta/build_id) (128 bits);
 * the [`type_id`](https://doc.rust-lang.org/std/intrinsics/fn.type_id.html) of the trait object (64 bits);
 * the `type_id` of the concrete type (64 bits).

At some point in Rust's future, I think it would be great if the latter could be used to safely look up and create a trait object. As it is, that functionality doesn't exist yet, so what this crate does instead is serialize the vtable pointer (relative to a static base), and do as much validity checking as it reasonably can before it can be used and potentially invoke UB.

The first two are [checked for validity](https://github.com/alecmocatta/relative/blob/dae206663a09b9c0c4b3012c528b0e9c063df742/src/lib.rs#L457-L474) before usage of the vtable pointer. The `build_id` ensures that the vtable pointer came from an invocation of an identically laid out binary<sup>1</sup>. The `type_id` ensures that the trait object being deserialized is the same type as the trait object that was serialized. They ensure that under non-malicious conditions, attempts to deserialize invalid data return an error rather than UB. The `type_id` of the concrete type is used as a [sanity check](https://github.com/alecmocatta/serde_traitobject/blob/b20d74e183063e7d49aff2eabc9dcd5bc26d7c07/src/lib.rs#L469) that panics if it differs from the `type_id` of the concrete type to be deserialized.

Regarding collisions, the 128 bit `build_id` colliding is sufficiently unlikely that it can be relied upon to never occur. The 64 bit `type_id` colliding is possible, see [rust-lang/rust#10389](https://github.com/rust-lang/rust/issues/10389), though exceedingly unlikely to occur in practise.

The vtable pointer is (de)serialized as a usize relative to the vtable pointer of [this static trait object](https://github.com/alecmocatta/relative/blob/dae206663a09b9c0c4b3012c528b0e9c063df742/src/lib.rs#L90). This enables it to work under typical dynamic linking conditions, where the absolute vtable addresses can differ across invocations of the same binary, but relative addresses remain constant.

All together this leaves, as far as I'm aware, three soundness holes:

 * A malicious user with a copy of the binary could trivially craft a `build_id` and `type_id` that pass validation and gives them control of where to jump to.
 * Data corruption of the serialized vtable pointer but not the `build_id` or `type_id` used for validation, resulting in a jump to an arbitrary address. This could be rectified in a future version of this library by using a cipher to make it vanishingly unlikely for corruptions to affect only the vtable pointer, by mixing the vtable pointer and validation components upon (de)serialization.
 * Dynamic linking conditions where the relative addresses (vtable - static vtable) are different across different invocations of the same binary. I'm sure this is possible, but it's not a scenario I've encountered so I can't speak to its commonness.

<sup>1</sup>I don't think this requirement is strictly necessary, as the `type_id` should include all information that could affect soundness (trait methods, calling conventions, etc), but it's included in case that doesn't hold in practise; to provide a more helpful error message; and to reduce the likelihood of collisions.

## Note

This crate currently requires Rust nightly.

## License
Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE.txt](LICENSE-APACHE.txt) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT.txt](LICENSE-MIT.txt) or http://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
