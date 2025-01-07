// SPDX-License-Identifier: MIT OR Apache-2.0
use alloc::boxed::Box;
use core::any::Any;

use crate::Miny;

#[test]
fn sized_values() {
	let small = Miny::new(1_u8);
	let large = Miny::new([1_usize; 32]);
	assert!(Miny::on_stack(&small));
	assert!(!Miny::on_stack(&large));
	assert_eq!(Miny::into_inner(small), 1);
	assert_eq!(Miny::into_inner(large), [1; 32]);
}

#[test]
fn dyn_values() {
	let small1: Miny<dyn Any> = Miny::new_unsized(1_u8);
	let small2: Miny<dyn Any + Sync + Send> = Miny::new_unsized(2_u8);
	let large1: Miny<dyn Any> = Miny::new_unsized([1_usize; 32]);
	let large2: Miny<dyn Any + Sync + Send> = Miny::new_unsized([1_usize; 32]);
	assert!(Miny::on_stack(&small1));
	assert!(Miny::on_stack(&small2));
	assert!(!Miny::on_stack(&large1));
	assert!(!Miny::on_stack(&large2));
	assert!(small1.is::<u8>());
	assert!(small2.is::<u8>());
	assert!(large1.is::<[usize; 32]>());
	assert!(large2.is::<[usize; 32]>());
}

#[test]
fn slices_unsize() {
	let small = Miny::unsize::<[u8]>(Miny::new([1, 2]));
	let large = Miny::unsize::<[u8]>(Miny::new([0; 128]));
	assert!(Miny::on_stack(&small));
	assert!(!Miny::on_stack(&large));
	assert_eq!(Miny::into_box(small).len(), 2);
	assert_eq!(Miny::into_box(large).len(), 128);
}

#[test]
fn slices_from() {
	let small = Miny::from(Box::new([1_u8, 2]) as Box<[u8]>);
	let large = Miny::from(Box::new([0_u8; 128]) as Box<[u8]>);
	assert!(Miny::on_stack(&small));
	assert!(!Miny::on_stack(&large));
	assert_eq!(small.len(), 2);
	assert_eq!(large.len(), 128);
}

#[test]
fn store_reference() {
	let mut value = 1_u8;
	let mut reference = Miny::new(&mut value);
	assert_eq!(**reference, 1);
	**reference += 1;
	drop(reference);
	assert_eq!(value, 2);
}

#[test]
fn with_zst() {
	let real = Miny::new(());
	let fake = Miny::unsize::<dyn Any>(real.clone());
	assert!(fake.is::<()>());
	assert!(Miny::into_box(fake).is::<()>());
	assert_eq!(Miny::into_box(real), Box::new(()));
}

#[test]
fn zst_from_box() {
	let boxed = Box::new(());
	let min = core::hint::black_box(Miny::from(boxed));
	// Should not dealloc
	drop(min);
}
