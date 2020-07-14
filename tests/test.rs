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
#![allow(clippy::unseparated_literal_suffix, dead_code)]

use serde_closure::Fn;
use serde_derive::{Deserialize, Serialize};
use serde_traitobject as st;
use serde_traitobject::{Deserialize, Serialize};
use std::{any, env, process, rc};
use wasm_bindgen_test::wasm_bindgen_test;

#[derive(Serialize, Deserialize)]
struct Abc {
	#[serde(with = "st")]
	a: rc::Rc<dyn HelloSerialize>,
	b: st::Rc<dyn HelloSerialize>,
	#[serde(with = "st")]
	c: Box<dyn HelloSerialize>,
	d: st::Box<dyn HelloSerialize>,
	#[serde(with = "st")]
	e: Box<dyn st::Any>,
	f: st::Box<dyn st::Any>,
	g: st::Box<dyn st::Fn(usize) -> String>,
	h: st::Box<dyn st::Any>,
	i: st::Box<dyn st::Any>,
	j: st::Box<String>,
	#[serde(with = "st")]
	k: Box<String>,
	l: st::Box<str>,
	#[serde(with = "st")]
	m: Box<str>,
	n: st::Box<[u16]>,
	#[serde(with = "st")]
	o: Box<[u16]>,
}

#[derive(Serialize)]
struct Def<'a> {
	a: &'a (dyn st::FnOnce<(), Output = ()> + 'static),
	c: &'a mut (dyn st::FnOnce<(), Output = ()> + 'static),
}

trait Hello {
	fn hi(&self) -> String;
}
trait HelloSerialize: Hello + Serialize + Deserialize {}
impl<T> HelloSerialize for T where T: Hello + Serialize + Deserialize {}

impl Hello for u32 {
	fn hi(&self) -> String {
		format!("hi u32! {:?}", self)
	}
}
impl Hello for u16 {
	fn hi(&self) -> String {
		format!("hi u16! {:?}", self)
	}
}
impl Hello for u8 {
	fn hi(&self) -> String {
		format!("hi u8! {:?}", self)
	}
}

#[derive(Serialize)]
struct Ghi<'a> {
	#[serde(with = "st")]
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

type Request = st::Box<dyn for<'a> st::FnOnce<(&'a String,), Output = ()> + Send>;

fn _assert() {
	fn assert_serializable<T>()
	where
		T: serde::Serialize + for<'de> serde::Deserialize<'de>,
	{
	}
	assert_serializable::<Request>();
}

#[wasm_bindgen_test]
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
			***Box::<dyn any::Any>::downcast::<st::Box<usize>>(h.into_any()).unwrap(),
			987_654_321
		);
		assert_eq!(
			*Box::<dyn any::Any>::downcast::<usize>(
				Box::<dyn any::Any>::downcast::<st::Box<dyn st::Any>>(i.into_any())
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

		let a: st::Box<dyn st::Any> = st::Box::new(st::Box::new(1usize) as st::Box<dyn st::Any>);
		let a: Box<dyn any::Any> = a.into_any();
		let a: Box<st::Box<dyn st::Any>> = Box::<dyn any::Any>::downcast(a).unwrap();
		let a: st::Box<dyn st::Any> = *a;
		let a: Box<dyn any::Any> = a.into_any();
		let _: Box<usize> = Box::<dyn any::Any>::downcast(a).unwrap();

		let original = Abc {
			a: rc::Rc::new(123u16),
			b: st::Rc::new(456u16),
			c: Box::new(789u32),
			d: st::Box::new(101u8),
			e: Box::new(78u8),
			f: st::Box::new(78u8),
			g: st::Box::new(Fn!(|a: usize| format!("hey {}!", a + 101))),
			h: st::Box::new(st::Box::new(987_654_321usize)),
			i: st::Box::new(st::Box::new(987_654_321usize) as st::Box<dyn st::Any>),
			j: st::Box::new(String::from("abc")),
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
		let a1 = serde_json::to_string(&(st::Box::new(78u8) as st::Box<dyn st::Debug>)).unwrap();
		let a1r: Result<st::Box<dyn st::Debug>, _> = serde_json::from_str(&a1);
		assert!(a1r.is_ok());
		let a1r: Result<st::Box<dyn st::Any>, _> = serde_json::from_str(&a1);
		assert!(a1r.is_err());
		let a1 = bincode::serialize(&(st::Box::new(78u8) as st::Box<dyn st::Debug>)).unwrap();
		let a1: Result<st::Box<dyn st::Any>, _> = bincode::deserialize(&a1);
		assert!(a1.is_err());
	}

	let original = Abc {
		a: rc::Rc::new(123u16),
		b: st::Rc::new(456u16),
		c: Box::new(789u32),
		d: st::Box::new(101u8),
		e: Box::new(78u8),
		f: st::Box::new(78u8),
		g: st::Box::new(Fn!(|a: usize| format!("hey {}!", a + 101))),
		h: st::Box::new(st::Box::new(987_654_321usize)),
		i: st::Box::new(st::Box::new(987_654_321usize) as st::Box<dyn st::Any>),
		j: st::Box::new(String::from("abc")),
		k: Box::new(String::from("def")),
		l: Into::<Box<str>>::into(String::from("ghi")).into(),
		m: String::from("jkl").into(),
		n: Into::<Box<[u16]>>::into(vec![1u16, 2, 3]).into(),
		o: vec![1u16, 2, 3].into(),
	};

	if cfg!(target_arch = "wasm32") {
		return;
	}

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
