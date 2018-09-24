//! A macro to prevent value of a type from being implicitly dropped.
//!
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
//! to enable optimizations for it to work. Unfortunately, sometimes it
//! is simply impossible for the compiler to remove the drop calls which
//! means `prevent_drop` will report a false positive. Try to
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
//! builds and tests you can use the following. Perhaps you need a
//! `opt-level = 2` before the compiler elides drop calls.
//!
//! ```ignore
//! [profile.dev]
//! opt-level = 1
//!
//! [profile.test]
//! opt-level = 1
//! ```
//!
//! Alternatively, you can enable the either the `abort` or the `panic`
//! feature. Like the names suggest this will make `prevent_drop!` use
//! `prevent_drop_abort!` or `prevent_drop_panic!` respectively. To set the strategy to panic for example, edit your `Cargo.toml` like this:
//!
//! ```ignore
//! [dependencies.prevent_drop]
//! version = "..."
//! features = ["panic"]
//! ```
//!
//! This can be useful to figure out where drop is being called through
//! debugging. It does mean you need to figure out how to get your
//! application to actually perform the drop.

#![doc(html_root_url = "https://docs.rs/prevent_drop")]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

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

#[macro_export]
macro_rules! prevent_drop_abort {
    ($T:ty) => {
        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                ::std::process::abort();
            }
        }
    };
}

#[macro_export]
macro_rules! prevent_drop_panic {
    ($T:ty) => {
        prevent_drop_panic!(
            $T,
            concat!(
                "Failed to explicitly drop an instance of ",
                stringify!($T),
                "."
            )
        );
    };
    ($T:ty, $msg:expr) => {
        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                if ::std::thread::panicking() == false {
                    panic!($msg);
                }
            }
        }
    };
}

#[cfg(all(not(abort), not(panic), opt_level_gt_0))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        prevent_drop_link!($T, $label);
    };
}

#[cfg(all(not(abort), not(panic), not(opt_level_gt_0)))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        compile_error!("The `prevent_drop!` macro requires you to enable optimizations or to enable either the `abort` or the `panic` feature.")
    };
}

#[cfg(all(abort, not(panic)))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        prevent_drop_abort!($T);
    };
}

#[cfg(all(not(abort), panic))]
#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        prevent_drop_panic!($T, $label);
    };
}

#[cfg(all(abort, panic))]
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

    prevent_drop_panic!(PanicStrategy);

    #[test]
    #[should_panic(expected = "Failed to explicitly drop an instance of PanicStrategy.")]
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
