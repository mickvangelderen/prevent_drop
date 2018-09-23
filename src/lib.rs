//! A macro to prevent a type from dropping.
//!
//! At the time of writing the only way to ensure some cleanup code is
//! run, is by implementing Drop. Drop has some limitations however: you
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
//! This macro allows you to detect if a value would be dropped at
//! compile time if and only if the compiler optimization elides the
//! drop function call. This means that you have to enable optimizations
//! for it to work.
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
//! function if it isn't called. If you for some reason need to test
//! your code without optimizations, you can enable the `abort` feature.
//! This will make drop abort the process in debug builds. A debugger
//! can help you find out why.

#![doc(html_root_url = "https://docs.rs/prevent_drop")]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

#[macro_export]
macro_rules! prevent_drop {
    ($T:ty, $label:ident) => {
        #[cfg(opt_level_gt_0)]
        extern "C" {
            fn $label();
        }

        #[cfg(opt_level_gt_0)]
        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                unsafe { $label() };
            }
        }

        #[cfg(all(not(opt_level_gt_0), abort))]
        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                ::std::process::abort();
            }
        }

        #[cfg(all(not(opt_level_gt_0), not(abort)))]
        impl Drop for $T {
            #[inline]
            fn drop(&mut self) {
                compile_error!("`prevent_drop!` requires you to enable optimizations.");
            }
        }
    };
}

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
}
