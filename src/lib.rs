//! At the time of writing the only way to ensure some cleanup code is
//! run is by implementing Drop. Drop has some limitations however: you
//! can't pass extra arguments or report errors from it. Even the
//! standard library faces this problem:
//!
//! ```ignore
//! impl Drop for FileDesc {
//!     fn drop(&mut self) {
//!         // Note that errors are ignored when closing a file descriptor. The
//!         // reason for this is that if an error occurs we don't actually know if
//!         // the file descriptor was closed or not, and if we retried (for
//!         // something like EINTR), we might close another valid file descriptor
//!         // opened after we closed ours.
//!         let _ = unsafe { libc::close(self.fd) };
//!     }
//! }
//! ```
//!
//! Something called linear types would help, see [this
//! thread](https://users.rust-lang.org/t/prevent-drop-at-compile-time/20508)
//! for more information, but that is not available as of September
//! 2018\. For now the `prevent_drop!` macro allows you to detect if a
//! value would be dropped at compile time if and only if the compiler
//! optimization elides the drop function call. This means that you have
//! to enable optimizations for it to work. Unfortunately, if the
//! compiler is unable to prove a drop call can be elided when in theory
//! it could be, `prevent_drop` will report a false positive. Try to
//! restructure your code so you can take ownership of the values that
//! you are dropping. Alternatively, consider falling back to a run-time
//! check.
//!
//! ## Example
//!
//! This does not compile because `r` is dropped.
//!
//! ```compile_error
//! #[macro_use]
//! extern crate prevent_drop;
//!
//! struct Resource;
//!
//! prevent_drop!(Resource, prevent_drop_Resource);
//!
//! fn main() {
//!     let r = Resource;
//!     // `r` is dropped.
//! }
//! ```
//!
//! A more elaborate example. Note that this function __must__ consume
//! the value, otherwise you cannot prevent it from dropping. You might
//! need to put your struct in an `Option` to achieve this.
//!
//! ```
//! #[macro_use]
//! extern crate prevent_drop;
//!
//! struct Resource;
//! struct Context;
//! struct Error;
//!
//! impl Resource {
//!     fn drop(self, context: &Context) -> Error {
//!         let zelf = ::std::mem::ManuallyDrop::new(self);
//!         // Perform cleanup.
//!         // Return error.
//!         Error
//!     }
//! }
//!
//! prevent_drop!(Resource, prevent_drop_Resource);
//!
//! fn main() {
//!     let c = Context;
//!     let r = Resource;
//!     r.drop(&c);
//! }
//! ```
//!
//! ## Configuration
//!
//! By default, `prevent_drop` only works when optimizations are
//! enabled. The macro relies on optimizations to remove the drop
//! function if it isn't called. To enable optimizations for debug
//! builds and tests you can use the following.
//!
//! ```ignore
//! [profile.test]
//! opt-level = 1
//! ```
//!
//! If that is not aggressive enough, you can try enabling maximum
//! optimization as well as thin link time optimization.
//!
//! ```ignore
//! [profile.test]
//! opt-level = 3
//! lto = "thin"
//! incremental = false
//! ```
//!
//! Alternatively, you can enable the either the `abort` or the `panic`
//! feature. Like the names suggest this will make `prevent_drop!` use
//! `prevent_drop_abort!` or `prevent_drop_panic!` respectively. To set
//! the strategy to panic for example, edit your `Cargo.toml` like this:
//!
//! ```ignore
//! [dependencies.prevent_drop]
//! version = "..."
//! features = ["panic"] # or "abort"
//! ```
//!
//! Review the documentation for the different prevent_drop strategies
//! for advice on when to use which one.

#![doc(html_root_url = "https://docs.rs/prevent_drop")]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

/// Implement Drop for a type that will not compile if it
/// gets called.
///
/// This is the default strategy. It declares an `extern` function that
/// should not have an implementation, causing the linker to emit an
/// error. Either `std::mem::ManuallyDrop` or `std::mem::forget` can be
/// used to prevent values from being dropped. Optimization is required
/// to elide the drop calls.
///
/// Since this is a compile-time check you cannot and need not test your
/// code for potential drops as it will not compile.

#[macro_export]
macro_rules! prevent_drop_link {
    ($T:ty, $label:ident) => {
        extern "C" {
            fn $label();
        }

        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                unsafe { $label() };
            }
        }
    };
}

/// Implement Drop for a type that will abort if it gets called.
///
/// The abort strategy simply aborts the process. It is very user
/// unfriendly, because it doesn't report a proper error message and it
/// doesn't unwind like panic, but it is easier to spot in intermediate
/// code or the binary. You can use it on a type if you guarantee that
/// it will never be dropped but the compiler is unable to deduct this.
///
/// Since this is a run-time check you need to have proper tests to
/// discover all potential drops.

#[macro_export]
macro_rules! prevent_drop_abort {
    ($T:ty, $label:ident) => {
        #[inline(never)]
        #[no_mangle]
        #[allow(non_snake_case, private_no_mangle_fns)]
        pub fn $label() {
            ::std::process::abort();
        }

        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                $label();
            }
        }
    };
}

/// Implement Drop for a type that will panic if it gets called.
///
/// The panic strategy panics with a customizable error message only if
/// the thread is not already panicking. The reason for this is that
/// usually the original panic is more informative. If we are already
/// panicking, leaking some resources is not as important.
///
/// Since this is a run-time check you need to have proper tests to
/// discover all potential drops.

#[macro_export]
macro_rules! prevent_drop_panic {
    ($T:ty, $label:ident) => {
        prevent_drop_panic!(
            $T,
            $label,
            concat!(
                "Forgot to explicitly drop an instance of ",
                stringify!($T),
                "."
            )
        );
    };
    ($T:ty, $label:ident, $msg:expr) => {
        #[inline(never)]
        #[no_mangle]
        #[allow(non_snake_case, private_no_mangle_fns)]
        pub fn $label() {
            if ::std::thread::panicking() == false {
                panic!($msg);
            }
        }

        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                $label();
            }
        }
    };
}

#[cfg(all(not(feature = "abort"), not(feature = "panic"), opt_level_gt_0))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        prevent_drop_link!($T, $label);
    };
    ($T:ty, $label:ident, $msg:expr) => {
        prevent_drop!($T, $label);
    };
}

#[cfg(all(not(feature = "abort"), not(feature = "panic"), not(opt_level_gt_0)))]
#[macro_export]

/// Implement Drop for a type so that instances of it cannot
/// be dropped.
///
/// By default, this macro redirects to `prevent_drop_link`. If the
/// `abort` feature is enabled it will redirect to `prevent_drop_abort.
/// If the `panic` feature is enabled it will redirect to
/// `prevent_drop_panic`.

macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        compile_error!("The `prevent_drop!` macro requires you to enable optimizations or to enable either the `abort` or the `panic` feature.");
    };
    ($T:ty, $label:ident, $msg:expr) => {
        prevent_drop!($T, $label);
    };
}

#[cfg(all(feature = "abort", not(feature = "panic")))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        prevent_drop_abort!($T, $label);
    };
    ($T:ty, $label:ident, $msg:expr) => {
        prevent_drop!($T, $label);
    };
}

#[cfg(all(not(feature = "abort"), feature = "panic"))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        prevent_drop_panic!($T, $label);
    };
    ($T:ty, $label:ident, $msg:expr) => {
        prevent_drop_panic!($T, $label, $msg);
    };
}

#[cfg(all(feature = "abort", feature = "panic"))]
compile_error!("You cannot use both the abort and the panic strategies at the same time. Choose one or the other.");

#[cfg(test)]
mod tests {
    struct Resource;
    struct Context;
    struct Error;

    impl Resource {
        fn drop(self, _context: &Context) -> Error {
            let _self = ::std::mem::ManuallyDrop::new(self);
            Error
        }
    }

    prevent_drop!(Resource, prevent_drop_Resource);

    #[test]
    fn test() {
        let c = Context;
        let r = Resource;
        r.drop(&c);
    }

    #[derive(Debug)]
    struct PanicStrategy;

    prevent_drop_panic!(PanicStrategy, forget_to_explicitly_drop_an_instance_of_PanicStrategy);

    #[test]
    #[should_panic(expected = "Forgot to explicitly drop an instance of PanicStrategy.")]
    fn prevent_drop_panic_panics() {
        let x = PanicStrategy;
        ::std::mem::drop(x);
    }

    #[test]
    #[should_panic(expected = "Something else happened that I need to know about!")]
    #[allow(unreachable_code, unused_variables)]
    fn prevent_drop_panic_does_not_panic_while_panicking() {
        let x = PanicStrategy;
        panic!("Something else happened that I need to know about!");
        ::std::mem::drop(x);
    }

    #[test]
    fn prevent_drop_panic_does_not_panic_if_value_is_dropped() {
        let _ = ::std::mem::ManuallyDrop::new(PanicStrategy);
    }
}
