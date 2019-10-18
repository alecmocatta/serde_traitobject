#![feature(unboxed_closures)]
#![warn(
	missing_copy_implementations,
	missing_debug_implementations,
	// missing_docs,
	trivial_numeric_casts,
	unused_extern_crates,
	unused_import_braces,
	unused_qualifications,
	unused_results,
	clippy::pedantic
)] // from https://github.com/rust-unofficial/patterns/blob/master/anti_patterns/deny-warnings.md
#![allow(where_clauses_object_safety, clippy::unseparated_literal_suffix)]

use serde_closure::Fn;
use serde_derive::{Deserialize, Serialize};
use serde_traitobject::{Deserialize, Serialize};
use std::{any, env, process, rc};

#[derive(Serialize, Deserialize)]
struct Abc {
	#[serde(with = "serde_traitobject")]
	a: rc::Rc<dyn HelloSerialize>,
	b: serde_traitobject::Rc<dyn HelloSerialize>,
	#[serde(with = "serde_traitobject")]
	c: Box<dyn HelloSerialize>,
	d: serde_traitobject::Box<dyn HelloSerialize>,
	#[serde(with = "serde_traitobject")]
	e: Box<dyn serde_traitobject::Any>,
	f: serde_traitobject::Box<dyn serde_traitobject::Any>,
	g: serde_traitobject::Box<dyn serde_traitobject::Fn(usize) -> String>,
	h: serde_traitobject::Box<dyn serde_traitobject::Any>,
	i: serde_traitobject::Box<dyn serde_traitobject::Any>,
	j: serde_traitobject::Box<String>,
	#[serde(with = "serde_traitobject")]
	k: Box<String>,
	l: serde_traitobject::Box<str>,
	#[serde(with = "serde_traitobject")]
	m: Box<str>,
	n: serde_traitobject::Box<[u16]>,
	#[serde(with = "serde_traitobject")]
	o: Box<[u16]>,
}

#[derive(Serialize)]
struct Def<'a> {
	a: &'a (dyn serde_traitobject::FnOnce<(), Output = ()> + 'static),
	c: &'a mut (dyn serde_traitobject::FnOnce<(), Output = ()> + 'static),
}

trait Hello {
	fn hi(&self) -> String;
}
trait HelloSerialize: Hello + Serialize + Deserialize {}
impl<T> HelloSerialize for T where T: Hello + Serialize + Deserialize {}

#[allow(clippy::use_self)]
impl Hello for u32 {
	fn hi(&self) -> String {
		format!("hi u32! {:?}", self)
	}
}
#[allow(clippy::use_self)]
impl Hello for u16 {
	fn hi(&self) -> String {
		format!("hi u16! {:?}", self)
	}
}
#[allow(clippy::use_self)]
impl Hello for u8 {
	fn hi(&self) -> String {
		format!("hi u8! {:?}", self)
	}
}

#[derive(Serialize)]
struct Ghi<'a> {
	#[serde(with = "serde_traitobject")]
	e: &'a (dyn Hello2Serialize + 'static),
}
trait Hello2 {}
trait Hello2Serialize: Hello2 + Serialize + Deserialize {}
impl<T> Hello2Serialize for T where T: Hello2 + Serialize + Deserialize {}
impl<'a> AsRef<dyn Hello2Serialize + 'a> for dyn Hello2Serialize {
	fn as_ref(&self) -> &(dyn Hello2Serialize + 'a) {
		self
	}
}

#[allow(clippy::too_many_lines)]
fn main() {
	let test = |Abc {
	                a,
	                b,
	                c,
	                d,
	                e,
	                f,
	                g,
	                h,
	                i,
	                j,
	                k,
	                l,
	                m,
	                n,
	                o,
	            }| {
		assert_eq!(a.hi(), "hi u16! 123");
		assert_eq!(b.hi(), "hi u16! 456");
		assert_eq!(c.hi(), "hi u32! 789");
		assert_eq!(d.hi(), "hi u8! 101");
		assert_eq!(
			*Box::<dyn any::Any>::downcast::<u8>(e.into_any()).unwrap(),
			78
		);
		assert_eq!(
			*Box::<dyn any::Any>::downcast::<u8>(f.into_any()).unwrap(),
			78
		);
		assert_eq!(g(22), "hey 123!");
		assert_eq!(
			***Box::<dyn any::Any>::downcast::<serde_traitobject::Box<usize>>(h.into_any())
				.unwrap(),
			987_654_321
		);
		assert_eq!(
			*Box::<dyn any::Any>::downcast::<usize>(
				Box::<dyn any::Any>::downcast::<serde_traitobject::Box<dyn serde_traitobject::Any>>(
					i.into_any()
				)
				.unwrap()
				.into_any()
			)
			.unwrap(),
			987_654_321
		);
		assert_eq!(**j, "abc");
		assert_eq!(*k, "def");
		assert_eq!(&**l, "ghi");
		assert_eq!(&*m, "jkl");
		assert_eq!(&**n, &[1u16, 2, 3]);
		assert_eq!(&*o, &[1u16, 2, 3]);
	};

	for _ in 0..1_000 {
		let a: Box<dyn any::Any> = Box::new(Box::new(1usize) as Box<dyn any::Any>);
		let a: Box<Box<dyn any::Any>> = Box::<dyn any::Any>::downcast(a).unwrap();
		let a: Box<dyn any::Any> = *a;
		let _: Box<usize> = Box::<dyn any::Any>::downcast(a).unwrap();

		let a: serde_traitobject::Box<dyn serde_traitobject::Any> =
			serde_traitobject::Box::new(serde_traitobject::Box::new(1usize)
				as serde_traitobject::Box<dyn serde_traitobject::Any>);
		let a: Box<dyn any::Any> = a.into_any();
		let a: Box<serde_traitobject::Box<dyn serde_traitobject::Any>> =
			Box::<dyn any::Any>::downcast(a).unwrap();
		let a: serde_traitobject::Box<dyn serde_traitobject::Any> = *a;
		let a: Box<dyn any::Any> = a.into_any();
		let _: Box<usize> = Box::<dyn any::Any>::downcast(a).unwrap();

		let original = Abc {
			a: rc::Rc::new(123u16),
			b: serde_traitobject::Rc::new(456u16),
			c: Box::new(789u32),
			d: serde_traitobject::Box::new(101u8),
			e: Box::new(78u8),
			f: serde_traitobject::Box::new(78u8),
			g: serde_traitobject::Box::new(Fn!(|a: usize| format!("hey {}!", a + 101))),
			h: serde_traitobject::Box::new(serde_traitobject::Box::new(987_654_321usize)),
			i: serde_traitobject::Box::new(serde_traitobject::Box::new(987_654_321usize)
				as serde_traitobject::Box<dyn serde_traitobject::Any>),
			j: serde_traitobject::Box::new(String::from("abc")),
			k: Box::new(String::from("def")),
			l: Into::<Box<str>>::into(String::from("ghi")).into(),
			m: String::from("jkl").into(),
			n: Into::<Box<[u16]>>::into(vec![1u16, 2, 3]).into(),
			o: vec![1u16, 2, 3].into(),
		};
		let a1 = serde_json::to_string(&original).unwrap();
		let a2 = bincode::serialize(&original).unwrap();
		let a1 = serde_json::from_str(&a1).unwrap();
		let a2 = bincode::deserialize(&a2).unwrap();
		test(a1);
		test(a2);
		let a1 = serde_json::to_string(
			&(serde_traitobject::Box::new(78u8)
				as serde_traitobject::Box<dyn serde_traitobject::Debug>),
		)
		.unwrap();
		let a1r: Result<serde_traitobject::Box<dyn serde_traitobject::Debug>, _> =
			serde_json::from_str(&a1);
		assert!(a1r.is_ok());
		let a1r: Result<serde_traitobject::Box<dyn serde_traitobject::Any>, _> =
			serde_json::from_str(&a1);
		assert!(a1r.is_err());
		let a1 = bincode::serialize(
			&(serde_traitobject::Box::new(78u8)
				as serde_traitobject::Box<dyn serde_traitobject::Debug>),
		)
		.unwrap();
		let a1: Result<serde_traitobject::Box<dyn serde_traitobject::Any>, _> =
			bincode::deserialize(&a1);
		assert!(a1.is_err());
	}

	let original = Abc {
		a: rc::Rc::new(123u16),
		b: serde_traitobject::Rc::new(456u16),
		c: Box::new(789u32),
		d: serde_traitobject::Box::new(101u8),
		e: Box::new(78u8),
		f: serde_traitobject::Box::new(78u8),
		g: serde_traitobject::Box::new(Fn!(|a: usize| format!("hey {}!", a + 101))),
		h: serde_traitobject::Box::new(serde_traitobject::Box::new(987_654_321usize)),
		i: serde_traitobject::Box::new(serde_traitobject::Box::new(987_654_321usize)
			as serde_traitobject::Box<dyn serde_traitobject::Any>),
		j: serde_traitobject::Box::new(String::from("abc")),
		k: Box::new(String::from("def")),
		l: Into::<Box<str>>::into(String::from("ghi")).into(),
		m: String::from("jkl").into(),
		n: Into::<Box<[u16]>>::into(vec![1u16, 2, 3]).into(),
		o: vec![1u16, 2, 3].into(),
	};

	if let Ok(x) = env::var("SERDE_TRAITOBJECT_SPAWNED") {
		let (a, bc): (_, Vec<u8>) = serde_json::from_str(&x).unwrap();
		eq(&original, &a);
		let b = bincode::deserialize(&bc).unwrap();
		eq(&original, &b);
		test(a);
		test(b);
		process::exit(0);
	}

	let exe = env::current_exe().unwrap();
	for i in 0..100 {
		println!("{}", i);
		let output = process::Command::new(&exe)
			.stdin(process::Stdio::null())
			.stdout(process::Stdio::inherit())
			.stderr(process::Stdio::inherit())
			.env(
				"SERDE_TRAITOBJECT_SPAWNED",
				serde_json::to_string(&(&original, bincode::serialize(&original).unwrap()))
					.unwrap(),
			)
			.output()
			.unwrap();
		if !output.status.success() {
			panic!("{}: {:?}", i, output);
		}
	}
}

fn eq<T: ?Sized>(_: &T, _: &T) {}
