use std::iter;

use std::ops::{Add, Sub, Neg, Mul};

pub trait Additive where
Self: Sized {
	const ZERO: Self;
	fn add(self, n: Self) -> Self;
	fn sub(self, n: Self) -> Self {
		self.add(n.neg())
	}
	fn neg(self) -> Self {
		Self::ZERO.sub(self)
	}
}

macro_rules! additive_impl_core {
	($id:ident => $body:expr, $($t:ty)*) => ($(
		impl Additive for $t {
			const ZERO: $t = 0 as $t;
			#[inline]
			fn add(self, n: $t) -> $t { self + n }
			#[inline]
			fn sub(self, n: $t) -> $t { self - n }
			#[inline]
			fn neg(self) -> $t { let $id = self; $body }
		}
	)*)
}

macro_rules! additive_impl_numeric {
    ($($t:ty)*) => { additive_impl_core!{ x => -x, $($t)*} }
}

macro_rules! additive_impl_unsigned {
    ($($t:ty)*) => {
        additive_impl_core!{ x => {
            !x.wrapping_add(1)
		}, $($t)*}
	}
}

additive_impl_numeric! { isize i8 i16 i32 i64 f32 f64 }
additive_impl_unsigned! { usize u8 u16 u32 u64 }

pub trait Ring where
Self: Additive + Clone {
	const ONE: Self;
	fn mul(self, n: Self) -> Self;
	fn pow(self, n: u32) -> Self {
		iter::repeat(self).take(n as usize).fold(Ring::ONE, Ring::mul)
	}
}

macro_rules! ring_impl_core {
	($id:ident, $n:ident => $body:expr, $($t:ty)*) => ($(
		impl Ring for $t {
			const ONE: $t = 1 as $t;
			fn mul(self, n: $t) -> $t { self * n }
			fn pow(self, n: u32) -> $t { let $id = self; let $n = n; $body }
		}
	)*)
}

macro_rules! ring_impl_int {
    ($($t:ty)*) => { ring_impl_core!{ x, n => x.pow(n), $($t)*} }
}

macro_rules! ring_impl_float {
    ($($t:ty)*) => { ring_impl_core!{ x, n => x.powi(n as i32), $($t)*} }
}

ring_impl_int! { usize u8 u16 u32 u64 isize i8 i16 i32 i64 }
ring_impl_float! { f32 f64 }

pub trait Module<T> where
Self: Additive, 
T: Ring {
	fn scale(self, n: T) -> Self;
}

macro_rules! module_impl {
	($($t:ty)*) => ($(
		impl Module<$t> for $t {
			fn scale(self, n: $t) -> $t { self * n }
		}
	)*)
}

module_impl! { usize u8 u16 u32 u64 isize i8 i16 i32 i64 f32 f64 }

/* TODO: Implement Field */
pub trait Algebraic where
Self: Ring {
	fn sqrt(self) -> Self;
}

macro_rules! algebraic_impl {
	($($t:ty)*) => ($(
		impl Algebraic for $t {
			fn sqrt(self) -> $t { self.sqrt() }
		}
	)*)
}

algebraic_impl! { f32 f64 }

#[derive(Debug, Clone, Copy, Default)]
pub struct vec2<T>{
	pub x: T,
	pub y: T,
}

impl<T> vec2<T> {
	pub fn new(x: T, y: T) -> vec2<T> {
		vec2 {x, y}
	}
}

impl<T: Additive> Add for vec2<T> {
	type Output = vec2<T>;

	fn add(self, v: vec2<T>) -> vec2<T> {
		vec2 {
			x: self.x.add(v.x),
			y: self.y.add(v.y),
		}
	}
}

impl<T: Additive> Sub for vec2<T> {
	type Output = vec2<T>;

	fn sub(self, v: vec2<T>) -> vec2<T> {
		vec2 {
			x: self.x.sub(v.x),
			y: self.y.sub(v.y),
		}
	}
}

impl<T: Additive> Neg for vec2<T> {
	type Output = vec2<T>;

	fn neg(self) -> vec2<T> {
		vec2 {
			x: self.x.neg(),
			y: self.y.neg(),
		}
	}
}

impl<T: Additive + Copy> Additive for vec2<T> {
	const ZERO: vec2<T> = vec2 {
		x: T::ZERO,
		y: T::ZERO,
	};
	fn add(self, v: Self) -> Self {
		self + v
	}
	fn sub(self, v: Self) -> Self {
		self - v
	}
	fn neg(self) -> Self { -self }
}

impl<T, K> Mul<K> for vec2<T> where
T: Module<K>,
K: Ring + Copy {
	type Output = vec2<T>;

	fn mul(self, n: K) -> vec2<T> {
		vec2 {
			x: self.x.scale(n),
			y: self.y.scale(n),
		}
	}
}

impl<T> vec2<T> where
T: Ring + Copy, {
	pub fn normsq(self) -> T {
		self.x.pow(2).add(self.y.pow(2))
	}
	pub fn norm<A: Algebraic + From<T>>(self) -> A {
		A::from(self.normsq()).sqrt()
	}
}
