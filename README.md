# serde_traitobject

[![Crates.io](https://img.shields.io/crates/v/serde_traitobject.svg?style=flat-square&maxAge=86400)](https://crates.io/crates/serde_traitobject)
[![Apache-2.0 licensed](https://img.shields.io/crates/l/serde_traitobject.svg?style=flat-square&maxAge=2592000)](LICENSE.txt)
[![Build Status](https://ci.appveyor.com/api/projects/status/github/alecmocatta/serde_traitobject?branch=master&svg=true)](https://ci.appveyor.com/project/alecmocatta/serde-traitobject)
[![Build Status](https://circleci.com/gh/alecmocatta/serde_traitobject/tree/master.svg?style=shield)](https://circleci.com/gh/alecmocatta/serde_traitobject)
[![Build Status](https://travis-ci.com/alecmocatta/serde_traitobject.svg?branch=master)](https://travis-ci.com/alecmocatta/serde_traitobject)

[Docs](https://docs.rs/crate/serde_traitobject)

Serializable trait objects.

This library enables the serialization of trait objects such that they can be sent between other processes running the same binary.

For example, if you have multiple forks of a process, or the same binary running on each of a cluster of machines, this library would help you to send trait objects between them.

The heart of this crate is the [Serialize](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Serialize.html) and [Deserialize](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Deserialize.html) traits. They are automatically implemented for all `T: serde::Serialize` and all `T: serde::de::DeserializeOwned` respectively.

Any trait can be made (de)serializable when made into a trait object by simply adding them as supertraits:

```rust
#[macro_use]
extern crate serde_derive;
extern crate serde_traitobject;

trait MyTrait: serde_traitobject::Serialize + serde_traitobject::Deserialize {
	fn my_method(&self);
}

#[derive(Serialize, Deserialize)]
struct Message(#[serde(with = "serde_traitobject")] Box<dyn MyTrait>);

// Woohoo, `Message` is now serializable!
```

There are two ways to use serde_traitobject to handle the (de)serialization:
 * `#[serde(with = "serde_traitobject")]` [field attribute](https://serde.rs/attributes.html) on a boxed trait object, which instructs serde to use the [serialize](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/fn.serialize.html) and [deserialize](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/fn.deserialize.html) functions;
 * The [Box](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/struct.Box.html), [Rc](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/struct.Rc.html) and [Arc](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/struct.Arc.html) structs, which are simple wrappers around their stdlib counterparts that automatically handle (de)serialization without needing the above annotation;

Additionally, there are several convenience traits implemented that extend their stdlib counterparts:

 * [Any](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Any.html), [Debug](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Debug.html), [Display](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Display.html), [Error](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Error.html), [Fn](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.Fn.html), [FnBox](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.FnBox.html), [FnMut](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.FnMut.html), [FnOnce](https://docs.rs/serde_traitobject/0.1.1/serde_traitobject/trait.FnOnce.html)

These are automatically implemented on all (de)serializable implementors of their stdlib counterparts:

```rust
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_traitobject as s;

use std::any::Any;

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

## Note

This crate works by wrapping the vtable pointer with [relative::Vtable](https://docs.rs/relative) such that it can safely be sent between processes.

This currently requires Rust nightly.

## License
Licensed under Apache License, Version 2.0, ([LICENSE.txt](LICENSE.txt) or http://www.apache.org/licenses/LICENSE-2.0).

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be licensed as above, without any additional terms or conditions.
