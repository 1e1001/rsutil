use alloc::boxed::Box;
use core::any::Any;

use crate::Miny;

#[test]
fn sized_values() {
	let small = Miny::new(1u8);
	let large = Miny::new([1usize; 32]);
	assert!(small.on_stack());
	assert!(!large.on_stack());
}

#[test]
fn dyn_values() {
	let small1 = Miny::new(1u8).unsize::<dyn Any>();
	let small2 = Miny::new(2u8).unsize::<dyn Any + Sync + Send>();
	let large1 = Miny::new([1usize; 32]).unsize::<dyn Any>();
	let large2 = Miny::new([1usize; 32]).unsize::<dyn Any + Sync + Send>();
	assert!(small1.on_stack());
	assert!(small2.on_stack());
	assert!(!large1.on_stack());
	assert!(!large2.on_stack());
	assert!(small1.is::<u8>());
	assert!(small2.is::<u8>());
	assert!(large1.is::<[usize; 32]>());
	assert!(large2.is::<[usize; 32]>());
}

#[test]
fn slices_unsize() {
	let small = Miny::new([1, 2]).unsize::<[u8]>();
	let large = Miny::new([0; 128]).unsize::<[u8]>();
	assert!(small.on_stack());
	assert!(!large.on_stack());
	assert_eq!(small.into_box().len(), 2);
	assert_eq!(large.into_box().len(), 128);
}

#[test]
fn slices_from() {
	let small = Miny::from(Box::new([1u8, 2]) as Box<[u8]>);
	let large = Miny::from(Box::new([0u8; 128]) as Box<[u8]>);
	assert!(small.on_stack());
	assert!(!large.on_stack());
	assert_eq!(small.len(), 2);
	assert_eq!(large.len(), 128);
}
