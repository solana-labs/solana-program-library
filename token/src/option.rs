//! A C representation of Rust's `std::option::Option` used accross the FFI
//! boundary for Solana program interfaces
//!
//! This implementation mostly matches `std::option` except iterators since the iteration
//! trait requires returning `std::option::Option`

use std::pin::Pin;
use std::{
    convert, hint, mem,
    ops::{Deref, DerefMut},
};

/// A C representation of Rust's `std::option::Option`
#[repr(C)]
#[derive(Copy, PartialEq, PartialOrd, Eq, Ord, Debug, Hash)]
pub enum OptionReprC<T> {
    /// No value
    None,
    /// Some value `T`
    Some(T),
}

/////////////////////////////////////////////////////////////////////////////
// Type implementation
/////////////////////////////////////////////////////////////////////////////

impl<T> OptionReprC<T> {
    /////////////////////////////////////////////////////////////////////////
    // Querying the contained values
    /////////////////////////////////////////////////////////////////////////

    /// Returns `true` if the option is a [`OptionReprC::Some`] value.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x: OptionReprC<u32> = OptionReprC::Some(2);
    /// assert_eq!(x.is_some(), true);
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// assert_eq!(x.is_some(), false);
    /// ```
    ///
    /// [`OptionReprC::Some`]: #variant.OptionReprC::Some
    #[must_use = "if you intended to assert that this has a value, consider `.unwrap()` instead"]
    #[inline]
    pub fn is_some(&self) -> bool {
        match *self {
            OptionReprC::Some(_) => true,
            OptionReprC::None => false,
        }
    }

    /// Returns `true` if the option is a [`OptionReprC::None`] value.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x: OptionReprC<u32> = OptionReprC::Some(2);
    /// assert_eq!(x.is_none(), false);
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// assert_eq!(x.is_none(), true);
    /// ```
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    #[must_use = "if you intended to assert that this doesn't have a value, consider \
                  `.and_then(|| panic!(\"`OptionReprC` had a value when expected `OptionReprC::None`\"))` instead"]
    #[inline]
    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    /// Returns `true` if the option is a [`OptionReprC::Some`] value containing the given value.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// #![feature(option_result_contains)]
    ///
    /// let x: OptionReprC<u32> = OptionReprC::Some(2);
    /// assert_eq!(x.contains(&2), true);
    ///
    /// let x: OptionReprC<u32> = OptionReprC::Some(3);
    /// assert_eq!(x.contains(&2), false);
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// assert_eq!(x.contains(&2), false);
    /// ```
    #[must_use]
    #[inline]
    pub fn contains<U>(&self, x: &U) -> bool
    where
        U: PartialEq<T>,
    {
        match self {
            OptionReprC::Some(y) => x == y,
            OptionReprC::None => false,
        }
    }

    /////////////////////////////////////////////////////////////////////////
    // Adapter for working with references
    /////////////////////////////////////////////////////////////////////////

    /// Converts from `&OptionReprC<T>` to `OptionReprC<&T>`.
    ///
    /// # Examples
    ///
    /// Converts an `OptionReprC<`[`String`]`>` into an `OptionReprC<`[`usize`]`>`, preserving the original.
    /// The [`map`] method takes the `self` argument by value, consuming the original,
    /// so this technique uses `as_ref` to first take an `OptionReprC` to a reference
    /// to the value inside the original.
    ///
    /// [`map`]: enum.OptionReprC.html#method.map
    /// [`String`]: ../../std/string/struct.String.html
    /// [`usize`]: ../../std/primitive.usize.html
    ///
    /// ```ignore
    /// let text: OptionReprC<String> = OptionReprC::Some("Hello, world!".to_string());
    /// // First, cast `OptionReprC<String>` to `OptionReprC<&String>` with `as_ref`,
    /// // then consume *that* with `map`, leaving `text` on the stack.
    /// let text_length: OptionReprC<usize> = text.as_ref().map(|s| s.len());
    /// println!("still can print text: {:?}", text);
    /// ```
    #[inline]
    pub fn as_ref(&self) -> OptionReprC<&T> {
        match *self {
            OptionReprC::Some(ref x) => OptionReprC::Some(x),
            OptionReprC::None => OptionReprC::None,
        }
    }

    /// Converts from `&mut OptionReprC<T>` to `OptionReprC<&mut T>`.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut x = OptionReprC::Some(2);
    /// match x.as_mut() {
    ///     OptionReprC::Some(v) => *v = 42,
    ///     OptionReprC::None => {},
    /// }
    /// assert_eq!(x, OptionReprC::Some(42));
    /// ```
    #[inline]
    pub fn as_mut(&mut self) -> OptionReprC<&mut T> {
        match *self {
            OptionReprC::Some(ref mut x) => OptionReprC::Some(x),
            OptionReprC::None => OptionReprC::None,
        }
    }

    /// Converts from [`Pin`]`<&OptionReprC<T>>` to `OptionReprC<`[`Pin`]`<&T>>`.
    ///
    /// [`Pin`]: ../pin/struct.Pin.html
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_pin_ref(self: Pin<&Self>) -> OptionReprC<Pin<&T>> {
        unsafe { Pin::get_ref(self).as_ref().map(|x| Pin::new_unchecked(x)) }
    }

    /// Converts from [`Pin`]`<&mut OptionReprC<T>>` to `OptionReprC<`[`Pin`]`<&mut T>>`.
    ///
    /// [`Pin`]: ../pin/struct.Pin.html
    #[inline]
    #[allow(clippy::wrong_self_convention)]
    pub fn as_pin_mut(self: Pin<&mut Self>) -> OptionReprC<Pin<&mut T>> {
        unsafe {
            Pin::get_unchecked_mut(self)
                .as_mut()
                .map(|x| Pin::new_unchecked(x))
        }
    }

    /////////////////////////////////////////////////////////////////////////
    // Getting to contained values
    /////////////////////////////////////////////////////////////////////////

    /// Unwraps an option, yielding the content of a [`OptionReprC::Some`].
    ///
    /// # Panics
    ///
    /// Panics if the value is a [`OptionReprC::None`] with a custom panic message provided by
    /// `msg`.
    ///
    /// [`OptionReprC::Some`]: #variant.OptionReprC::Some
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some("value");
    /// assert_eq!(x.expect("the world is ending"), "value");
    /// ```
    ///
    /// ```ignore{.should_panic}
    /// let x: OptionReprC<&str> = OptionReprC::None;
    /// x.expect("the world is ending"); // panics with `the world is ending`
    /// ```
    #[inline]
    pub fn expect(self, msg: &str) -> T {
        match self {
            OptionReprC::Some(val) => val,
            OptionReprC::None => expect_failed(msg),
        }
    }

    /// Moves the value `v` out of the `OptionReprC<T>` if it is [`OptionReprC::Some(v)`].
    ///
    /// In general, because this function may panic, its use is discouraged.
    /// Instead, prefer to use pattern matching and handle the [`OptionReprC::None`]
    /// case explicitly.
    ///
    /// # Panics
    ///
    /// Panics if the self value equals [`OptionReprC::None`].
    ///
    /// [`OptionReprC::Some(v)`]: #variant.OptionReprC::Some
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some("air");
    /// assert_eq!(x.unwrap(), "air");
    /// ```
    ///
    /// ```ignore{.should_panic}
    /// let x: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.unwrap(), "air"); // fails
    /// ```
    #[inline]
    pub fn unwrap(self) -> T {
        match self {
            OptionReprC::Some(val) => val,
            OptionReprC::None => {
                panic!("called `OptionReprC::unwrap()` on a `OptionReprC::None` value")
            }
        }
    }

    /// Returns the contained value or a default.
    ///
    /// Arguments passed to `unwrap_or` are eagerly evaluated; if you are passing
    /// the result of a function call, it is recommended to use [`unwrap_or_else`],
    /// which is lazily evaluated.
    ///
    /// [`unwrap_or_else`]: #method.unwrap_or_else
    ///
    /// # Examples
    ///
    /// ```ignore
    /// assert_eq!(OptionReprC::Some("car").unwrap_or("bike"), "car");
    /// assert_eq!(OptionReprC::None.unwrap_or("bike"), "bike");
    /// ```
    #[inline]
    pub fn unwrap_or(self, def: T) -> T {
        match self {
            OptionReprC::Some(x) => x,
            OptionReprC::None => def,
        }
    }

    /// Returns the contained value or computes it from a closure.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let k = 10;
    /// assert_eq!(OptionReprC::Some(4).unwrap_or_else(|| 2 * k), 4);
    /// assert_eq!(OptionReprC::None.unwrap_or_else(|| 2 * k), 20);
    /// ```
    #[inline]
    pub fn unwrap_or_else<F: FnOnce() -> T>(self, f: F) -> T {
        match self {
            OptionReprC::Some(x) => x,
            OptionReprC::None => f(),
        }
    }

    /////////////////////////////////////////////////////////////////////////
    // Transforming contained values
    /////////////////////////////////////////////////////////////////////////

    /// Maps an `OptionReprC<T>` to `OptionReprC<U>` by applying a function to a contained value.
    ///
    /// # Examples
    ///
    /// Converts an `OptionReprC<`[`String`]`>` into an `OptionReprC<`[`usize`]`>`, consuming the original:
    ///
    /// [`String`]: ../../std/string/struct.String.html
    /// [`usize`]: ../../std/primitive.usize.html
    ///
    /// ```ignore
    /// let maybe_some_string = OptionReprC::Some(String::from("Hello, World!"));
    /// // `OptionReprC::map` takes self *by value*, consuming `maybe_some_string`
    /// let maybe_some_len = maybe_some_string.map(|s| s.len());
    ///
    /// assert_eq!(maybe_some_len, OptionReprC::Some(13));
    /// ```
    #[inline]
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> OptionReprC<U> {
        match self {
            OptionReprC::Some(x) => OptionReprC::Some(f(x)),
            OptionReprC::None => OptionReprC::None,
        }
    }

    /// Applies a function to the contained value (if any),
    /// or returns the provided default (if not).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some("foo");
    /// assert_eq!(x.map_or(42, |v| v.len()), 3);
    ///
    /// let x: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.map_or(42, |v| v.len()), 42);
    /// ```
    #[inline]
    pub fn map_or<U, F: FnOnce(T) -> U>(self, default: U, f: F) -> U {
        match self {
            OptionReprC::Some(t) => f(t),
            OptionReprC::None => default,
        }
    }

    /// Applies a function to the contained value (if any),
    /// or computes a default (if not).
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let k = 21;
    ///
    /// let x = OptionReprC::Some("foo");
    /// assert_eq!(x.map_or_else(|| 2 * k, |v| v.len()), 3);
    ///
    /// let x: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.map_or_else(|| 2 * k, |v| v.len()), 42);
    /// ```
    #[inline]
    pub fn map_or_else<U, D: FnOnce() -> U, F: FnOnce(T) -> U>(self, default: D, f: F) -> U {
        match self {
            OptionReprC::Some(t) => f(t),
            OptionReprC::None => default(),
        }
    }

    /// Transforms the `OptionReprC<T>` into a [`Result<T, E>`], mapping [`OptionReprC::Some(v)`] to
    /// [`Ok(v)`] and [`OptionReprC::None`] to [`Err(err)`].
    ///
    /// Arguments passed to `ok_or` are eagerly evaluated; if you are passing the
    /// result of a function call, it is recommended to use [`ok_or_else`], which is
    /// lazily evaluated.
    ///
    /// [`Result<T, E>`]: ../../std/result/enum.Result.html
    /// [`Ok(v)`]: ../../std/result/enum.Result.html#variant.Ok
    /// [`Err(err)`]: ../../std/result/enum.Result.html#variant.Err
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    /// [`OptionReprC::Some(v)`]: #variant.OptionReprC::Some
    /// [`ok_or_else`]: #method.ok_or_else
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some("foo");
    /// assert_eq!(x.ok_or(0), Ok("foo"));
    ///
    /// let x: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.ok_or(0), Err(0));
    /// ```
    #[inline]
    pub fn ok_or<E>(self, err: E) -> Result<T, E> {
        match self {
            OptionReprC::Some(v) => Ok(v),
            OptionReprC::None => Err(err),
        }
    }

    /// Transforms the `OptionReprC<T>` into a [`Result<T, E>`], mapping [`OptionReprC::Some(v)`] to
    /// [`Ok(v)`] and [`OptionReprC::None`] to [`Err(err())`].
    ///
    /// [`Result<T, E>`]: ../../std/result/enum.Result.html
    /// [`Ok(v)`]: ../../std/result/enum.Result.html#variant.Ok
    /// [`Err(err())`]: ../../std/result/enum.Result.html#variant.Err
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    /// [`OptionReprC::Some(v)`]: #variant.OptionReprC::Some
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some("foo");
    /// assert_eq!(x.ok_or_else(|| 0), Ok("foo"));
    ///
    /// let x: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.ok_or_else(|| 0), Err(0));
    /// ```
    #[inline]
    pub fn ok_or_else<E, F: FnOnce() -> E>(self, err: F) -> Result<T, E> {
        match self {
            OptionReprC::Some(v) => Ok(v),
            OptionReprC::None => Err(err()),
        }
    }

    /////////////////////////////////////////////////////////////////////////
    // Boolean operations on the values, eager and lazy
    /////////////////////////////////////////////////////////////////////////

    /// Returns [`OptionReprC::None`] if the option is [`OptionReprC::None`], otherwise returns `optb`.
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some(2);
    /// let y: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.and(y), OptionReprC::None);
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// let y = OptionReprC::Some("foo");
    /// assert_eq!(x.and(y), OptionReprC::None);
    ///
    /// let x = OptionReprC::Some(2);
    /// let y = OptionReprC::Some("foo");
    /// assert_eq!(x.and(y), OptionReprC::Some("foo"));
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// let y: OptionReprC<&str> = OptionReprC::None;
    /// assert_eq!(x.and(y), OptionReprC::None);
    /// ```
    #[inline]
    pub fn and<U>(self, optb: OptionReprC<U>) -> OptionReprC<U> {
        match self {
            OptionReprC::Some(_) => optb,
            OptionReprC::None => OptionReprC::None,
        }
    }

    /// Returns [`OptionReprC::None`] if the option is [`OptionReprC::None`], otherwise calls `f` with the
    /// wrapped value and returns the result.
    ///
    /// OptionReprC::Some languages call this operation flatmap.
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// fn sq(x: u32) -> OptionReprC<u32> { OptionReprC::Some(x * x) }
    /// fn nope(_: u32) -> OptionReprC<u32> { OptionReprC::None }
    ///
    /// assert_eq!(OptionReprC::Some(2).and_then(sq).and_then(sq), OptionReprC::Some(16));
    /// assert_eq!(OptionReprC::Some(2).and_then(sq).and_then(nope), OptionReprC::None);
    /// assert_eq!(OptionReprC::Some(2).and_then(nope).and_then(sq), OptionReprC::None);
    /// assert_eq!(OptionReprC::None.and_then(sq).and_then(sq), OptionReprC::None);
    /// ```
    #[inline]
    pub fn and_then<U, F: FnOnce(T) -> OptionReprC<U>>(self, f: F) -> OptionReprC<U> {
        match self {
            OptionReprC::Some(x) => f(x),
            OptionReprC::None => OptionReprC::None,
        }
    }

    /// Returns [`OptionReprC::None`] if the option is [`OptionReprC::None`], otherwise calls `predicate`
    /// with the wrapped value and returns:
    ///
    /// - [`OptionReprC::Some(t)`] if `predicate` returns `true` (where `t` is the wrapped
    ///   value), and
    /// - [`OptionReprC::None`] if `predicate` returns `false`.
    ///
    /// This function works similar to [`Iterator::filter()`]. You can imagine
    /// the `OptionReprC<T>` being an iterator over one or zero elements. `filter()`
    /// lets you decide which elements to keep.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// fn is_even(n: &i32) -> bool {
    ///     n % 2 == 0
    /// }
    ///
    /// assert_eq!(OptionReprC::None.filter(is_even), OptionReprC::None);
    /// assert_eq!(OptionReprC::Some(3).filter(is_even), OptionReprC::None);
    /// assert_eq!(OptionReprC::Some(4).filter(is_even), OptionReprC::Some(4));
    /// ```
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    /// [`OptionReprC::Some(t)`]: #variant.OptionReprC::Some
    /// [`Iterator::filter()`]: ../../std/iter/trait.Iterator.html#method.filter
    #[inline]
    pub fn filter<P: FnOnce(&T) -> bool>(self, predicate: P) -> Self {
        if let OptionReprC::Some(x) = self {
            if predicate(&x) {
                return OptionReprC::Some(x);
            }
        }
        OptionReprC::None
    }

    /// Returns the option if it contains a value, otherwise returns `optb`.
    ///
    /// Arguments passed to `or` are eagerly evaluated; if you are passing the
    /// result of a function call, it is recommended to use [`or_else`], which is
    /// lazily evaluated.
    ///
    /// [`or_else`]: #method.or_else
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some(2);
    /// let y = OptionReprC::None;
    /// assert_eq!(x.or(y), OptionReprC::Some(2));
    ///
    /// let x = OptionReprC::None;
    /// let y = OptionReprC::Some(100);
    /// assert_eq!(x.or(y), OptionReprC::Some(100));
    ///
    /// let x = OptionReprC::Some(2);
    /// let y = OptionReprC::Some(100);
    /// assert_eq!(x.or(y), OptionReprC::Some(2));
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// let y = OptionReprC::None;
    /// assert_eq!(x.or(y), OptionReprC::None);
    /// ```ignore
    #[inline]
    pub fn or(self, optb: OptionReprC<T>) -> OptionReprC<T> {
        match self {
            OptionReprC::Some(_) => self,
            OptionReprC::None => optb,
        }
    }

    /// Returns the option if it contains a value, otherwise calls `f` and
    /// returns the result.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// fn nobody() -> OptionReprC<&'static str> { OptionReprC::None }
    /// fn vikings() -> OptionReprC<&'static str> { OptionReprC::Some("vikings") }
    ///
    /// assert_eq!(OptionReprC::Some("barbarians").or_else(vikings), OptionReprC::Some("barbarians"));
    /// assert_eq!(OptionReprC::None.or_else(vikings), OptionReprC::Some("vikings"));
    /// assert_eq!(OptionReprC::None.or_else(nobody), OptionReprC::None);
    /// ```
    #[inline]
    pub fn or_else<F: FnOnce() -> OptionReprC<T>>(self, f: F) -> OptionReprC<T> {
        match self {
            OptionReprC::Some(_) => self,
            OptionReprC::None => f(),
        }
    }

    /// Returns [`OptionReprC::Some`] if exactly one of `self`, `optb` is [`OptionReprC::Some`], otherwise returns [`OptionReprC::None`].
    ///
    /// [`OptionReprC::Some`]: #variant.OptionReprC::Some
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = OptionReprC::Some(2);
    /// let y: OptionReprC<u32> = OptionReprC::None;
    /// assert_eq!(x.xor(y), OptionReprC::Some(2));
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// let y = OptionReprC::Some(2);
    /// assert_eq!(x.xor(y), OptionReprC::Some(2));
    ///
    /// let x = OptionReprC::Some(2);
    /// let y = OptionReprC::Some(2);
    /// assert_eq!(x.xor(y), OptionReprC::None);
    ///
    /// let x: OptionReprC<u32> = OptionReprC::None;
    /// let y: OptionReprC<u32> = OptionReprC::None;
    /// assert_eq!(x.xor(y), OptionReprC::None);
    /// ```
    #[inline]
    pub fn xor(self, optb: OptionReprC<T>) -> OptionReprC<T> {
        match (self, optb) {
            (OptionReprC::Some(a), OptionReprC::None) => OptionReprC::Some(a),
            (OptionReprC::None, OptionReprC::Some(b)) => OptionReprC::Some(b),
            _ => OptionReprC::None,
        }
    }

    /////////////////////////////////////////////////////////////////////////
    // Entry-like operations to insert if OptionReprC::None and return a reference
    /////////////////////////////////////////////////////////////////////////

    /// Inserts `v` into the option if it is [`OptionReprC::None`], then
    /// returns a mutable reference to the contained value.
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut x = OptionReprC::None;
    ///
    /// {
    ///     let y: &mut u32 = x.get_or_insert(5);
    ///     assert_eq!(y, &5);
    ///
    ///     *y = 7;
    /// }
    ///
    /// assert_eq!(x, OptionReprC::Some(7));
    /// ```
    #[inline]
    pub fn get_or_insert(&mut self, v: T) -> &mut T {
        self.get_or_insert_with(|| v)
    }

    /// Inserts a value computed from `f` into the option if it is [`OptionReprC::None`], then
    /// returns a mutable reference to the contained value.
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut x = OptionReprC::None;
    ///
    /// {
    ///     let y: &mut u32 = x.get_or_insert_with(|| 5);
    ///     assert_eq!(y, &5);
    ///
    ///     *y = 7;
    /// }
    ///
    /// assert_eq!(x, OptionReprC::Some(7));
    /// ```
    #[inline]
    pub fn get_or_insert_with<F: FnOnce() -> T>(&mut self, f: F) -> &mut T {
        if let OptionReprC::None = *self {
            *self = OptionReprC::Some(f())
        }

        match *self {
            OptionReprC::Some(ref mut v) => v,
            OptionReprC::None => unsafe { hint::unreachable_unchecked() },
        }
    }

    /////////////////////////////////////////////////////////////////////////
    // Misc
    /////////////////////////////////////////////////////////////////////////

    /// Replaces the actual value in the option by the value given in parameter,
    /// returning the old value if present,
    /// leaving a [`OptionReprC::Some`] in its place without deinitializing either one.
    ///
    /// [`OptionReprC::Some`]: #variant.OptionReprC::Some
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut x = OptionReprC::Some(2);
    /// let old = x.replace(5);
    /// assert_eq!(x, OptionReprC::Some(5));
    /// assert_eq!(old, OptionReprC::Some(2));
    ///
    /// let mut x = OptionReprC::None;
    /// let old = x.replace(3);
    /// assert_eq!(x, OptionReprC::Some(3));
    /// assert_eq!(old, OptionReprC::None);
    /// ```
    #[inline]
    pub fn replace(&mut self, value: T) -> OptionReprC<T> {
        mem::replace(self, OptionReprC::Some(value))
    }
}

impl<T: Copy> OptionReprC<&T> {
    /// Maps an `OptionReprC<&T>` to an `OptionReprC<T>` by copying the contents of the
    /// option.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = 12;
    /// let opt_x = OptionReprC::Some(&x);
    /// assert_eq!(opt_x, OptionReprC::Some(&12));
    /// let copied = opt_x.copied();
    /// assert_eq!(copied, OptionReprC::Some(12));
    /// ```
    pub fn copied(self) -> OptionReprC<T> {
        self.map(|&t| t)
    }
}

impl<T: Copy> OptionReprC<&mut T> {
    /// Maps an `OptionReprC<&mut T>` to an `OptionReprC<T>` by copying the contents of the
    /// option.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut x = 12;
    /// let opt_x = OptionReprC::Some(&mut x);
    /// assert_eq!(opt_x, OptionReprC::Some(&mut 12));
    /// let copied = opt_x.copied();
    /// assert_eq!(copied, OptionReprC::Some(12));
    /// ```
    pub fn copied(self) -> OptionReprC<T> {
        self.map(|&mut t| t)
    }
}

impl<T: Clone> OptionReprC<&T> {
    /// Maps an `OptionReprC<&T>` to an `OptionReprC<T>` by cloning the contents of the
    /// option.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let x = 12;
    /// let opt_x = OptionReprC::Some(&x);
    /// assert_eq!(opt_x, OptionReprC::Some(&12));
    /// let cloned = opt_x.cloned();
    /// assert_eq!(cloned, OptionReprC::Some(12));
    /// ```
    pub fn cloned(self) -> OptionReprC<T> {
        self.map(|t| t.clone())
    }
}

impl<T: Clone> OptionReprC<&mut T> {
    /// Maps an `OptionReprC<&mut T>` to an `OptionReprC<T>` by cloning the contents of the
    /// option.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mut x = 12;
    /// let opt_x = OptionReprC::Some(&mut x);
    /// assert_eq!(opt_x, OptionReprC::Some(&mut 12));
    /// let cloned = opt_x.cloned();
    /// assert_eq!(cloned, OptionReprC::Some(12));
    /// ```
    pub fn cloned(self) -> OptionReprC<T> {
        self.map(|t| t.clone())
    }
}

impl<T: Default> OptionReprC<T> {
    /// Returns the contained value or a default
    ///
    /// Consumes the `self` argument then, if [`OptionReprC::Some`], returns the contained
    /// value, otherwise if [`OptionReprC::None`], returns the [default value] for that
    /// type.
    ///
    /// # Examples
    ///
    /// Converts a string to an integer, turning poorly-formed strings
    /// into 0 (the default value for integers). [`parse`] converts
    /// a string to any other type that implements [`FromStr`], returning
    /// [`OptionReprC::None`] on error.
    ///
    /// ```ignore
    /// let good_year_from_input = "1909";
    /// let bad_year_from_input = "190blarg";
    /// let good_year = good_year_from_input.parse().ok().unwrap_or_default();
    /// let bad_year = bad_year_from_input.parse().ok().unwrap_or_default();
    ///
    /// assert_eq!(1909, good_year);
    /// assert_eq!(0, bad_year);
    /// ```
    ///
    /// [`OptionReprC::Some`]: #variant.OptionReprC::Some
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    /// [default value]: ../default/trait.Default.html#tymethod.default
    /// [`parse`]: ../../std/primitive.str.html#method.parse
    /// [`FromStr`]: ../../std/str/trait.FromStr.html
    #[inline]
    pub fn unwrap_or_default(self) -> T {
        match self {
            OptionReprC::Some(x) => x,
            OptionReprC::None => Default::default(),
        }
    }
}

impl<T: Deref> OptionReprC<T> {
    /// Converts from `OptionReprC<T>` (or `&OptionReprC<T>`) to `OptionReprC<&T::Target>`.
    ///
    /// Leaves the original OptionReprC in-place, creating a new one with a reference
    /// to the original one, additionally coercing the contents via [`Deref`].
    ///
    /// [`Deref`]: ../../std/ops/trait.Deref.html
    ///
    /// # Examples
    ///
    /// ```ignore
    /// #![feature(inner_deref)]
    ///
    /// let x: OptionReprC<String> = OptionReprC::Some("hey".to_owned());
    /// assert_eq!(x.as_deref(), OptionReprC::Some("hey"));
    ///
    /// let x: OptionReprC<String> = OptionReprC::None;
    /// assert_eq!(x.as_deref(), OptionReprC::None);
    /// ```
    pub fn as_deref(&self) -> OptionReprC<&T::Target> {
        self.as_ref().map(|t| t.deref())
    }
}

impl<T: DerefMut> OptionReprC<T> {
    /// Converts from `OptionReprC<T>` (or `&mut OptionReprC<T>`) to `OptionReprC<&mut T::Target>`.
    ///
    /// Leaves the original `OptionReprC` in-place, creating a new one containing a mutable reference to
    /// the inner type's `Deref::Target` type.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// #![feature(inner_deref)]
    ///
    /// let mut x: OptionReprC<String> = OptionReprC::Some("hey".to_owned());
    /// assert_eq!(x.as_deref_mut().map(|x| {
    ///     x.make_ascii_uppercase();
    ///     x
    /// }), OptionReprC::Some("HEY".to_owned().as_mut_str()));
    /// ```
    pub fn as_deref_mut(&mut self) -> OptionReprC<&mut T::Target> {
        self.as_mut().map(|t| t.deref_mut())
    }
}

impl<T, E> OptionReprC<Result<T, E>> {
    /// Transposes an `OptionReprC` of a [`Result`] into a [`Result`] of an `OptionReprC`.
    ///
    /// [`OptionReprC::None`] will be mapped to [`Ok`]`(`[`OptionReprC::None`]`)`.
    /// [`OptionReprC::Some`]`(`[`Ok`]`(_))` and [`OptionReprC::Some`]`(`[`Err`]`(_))` will be mapped to
    /// [`Ok`]`(`[`OptionReprC::Some`]`(_))` and [`Err`]`(_)`.
    ///
    /// [`OptionReprC::None`]: #variant.OptionReprC::None
    /// [`Ok`]: ../../std/result/enum.Result.html#variant.Ok
    /// [`OptionReprC::Some`]: #variant.OptionReprC::Some
    /// [`Err`]: ../../std/result/enum.Result.html#variant.Err
    ///
    /// # Examples
    ///
    /// ```ignore
    /// #[derive(Debug, Eq, PartialEq)]
    /// struct OptionReprC::SomeErr;
    ///
    /// let x: Result<OptionReprC<i32>, OptionReprC::SomeErr> = Ok(OptionReprC::Some(5));
    /// let y: OptionReprC<Result<i32, OptionReprC::SomeErr>> = OptionReprC::Some(Ok(5));
    /// assert_eq!(x, y.transpose());
    /// ```
    #[inline]
    pub fn transpose(self) -> Result<OptionReprC<T>, E> {
        match self {
            OptionReprC::Some(Ok(x)) => Ok(OptionReprC::Some(x)),
            OptionReprC::Some(Err(e)) => Err(e),
            OptionReprC::None => Ok(OptionReprC::None),
        }
    }
}

// This is a separate function to reduce the code size of .expect() itself.
#[inline(never)]
#[cold]
fn expect_failed(msg: &str) -> ! {
    panic!("{}", msg)
}

// // This is a separate function to reduce the code size of .expect_none() itself.
// #[inline(never)]
// #[cold]
// fn expect_none_failed(msg: &str, value: &dyn fmt::Debug) -> ! {
//     panic!("{}: {:?}", msg, value)
// }

/////////////////////////////////////////////////////////////////////////////
// Trait implementations
/////////////////////////////////////////////////////////////////////////////

impl<T: Clone> Clone for OptionReprC<T> {
    #[inline]
    fn clone(&self) -> Self {
        match self {
            OptionReprC::Some(x) => OptionReprC::Some(x.clone()),
            OptionReprC::None => OptionReprC::None,
        }
    }

    #[inline]
    fn clone_from(&mut self, source: &Self) {
        match (self, source) {
            (OptionReprC::Some(to), OptionReprC::Some(from)) => to.clone_from(from),
            (to, from) => *to = from.clone(),
        }
    }
}

impl<T> Default for OptionReprC<T> {
    /// Returns [`OptionReprC::None`][OptionReprC::OptionReprC::None].
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let opt: OptionReprC<u32> = OptionReprC::default();
    /// assert!(opt.is_none());
    /// ```
    #[inline]
    fn default() -> OptionReprC<T> {
        OptionReprC::None
    }
}

impl<T> From<T> for OptionReprC<T> {
    fn from(val: T) -> OptionReprC<T> {
        OptionReprC::Some(val)
    }
}

impl<'a, T> From<&'a OptionReprC<T>> for OptionReprC<&'a T> {
    fn from(o: &'a OptionReprC<T>) -> OptionReprC<&'a T> {
        o.as_ref()
    }
}

impl<'a, T> From<&'a mut OptionReprC<T>> for OptionReprC<&'a mut T> {
    fn from(o: &'a mut OptionReprC<T>) -> OptionReprC<&'a mut T> {
        o.as_mut()
    }
}

impl<T> OptionReprC<OptionReprC<T>> {
    /// Converts from `OptionReprC<OptionReprC<T>>` to `OptionReprC<T>`
    ///
    /// # Examples
    /// Basic usage:
    /// ```ignore
    /// #![feature(option_flattening)]
    /// let x: OptionReprC<OptionReprC<u32>> = OptionReprC::Some(OptionReprC::Some(6));
    /// assert_eq!(OptionReprC::Some(6), x.flatten());
    ///
    /// let x: OptionReprC<OptionReprC<u32>> = OptionReprC::Some(OptionReprC::None);
    /// assert_eq!(OptionReprC::None, x.flatten());
    ///
    /// let x: OptionReprC<OptionReprC<u32>> = OptionReprC::None;
    /// assert_eq!(OptionReprC::None, x.flatten());
    /// ```
    /// Flattening once only removes one level of nesting:
    /// ```ignore
    /// #![feature(option_flattening)]
    /// let x: OptionReprC<OptionReprC<OptionReprC<u32>>> = OptionReprC::Some(OptionReprC::Some(OptionReprC::Some(6)));
    /// assert_eq!(OptionReprC::Some(OptionReprC::Some(6)), x.flatten());
    /// assert_eq!(OptionReprC::Some(6), x.flatten().flatten());
    /// ```
    #[inline]
    pub fn flatten(self) -> OptionReprC<T> {
        self.and_then(convert::identity)
    }
}
