# prevent_drop

This crate provides a macro to generate a `Drop` implementation that will not
compile if it ever gets called. Such a thing is useful if you want to force a
custom drop-like function to be called. This custom cleanup function can, unlike
drop, 1) take additional parameters and 2) return a value.

## Practical advice

The way the compile time check is implemented is by declaring a extern function
and calling that in `drop`. The linker will try to find a definition for this
function but it cannot, giving an error. If `drop` is never called, the external
function is never called and thus does not need to be linked. 

For simple examples, the compile time assertion works. It does require that the
`drop` call is elided. That means we need fairly aggressive optimization which
in turn, increases compilation time and makes it harder to debug the
application. That is something I could personally live with to be sure my values
are always properly cleaned up.

What we can not live with is based on these premises:

1. realistic code usualy does something interesting in between creating a
   resource and cleaning it up,
2. interesting code is very likely to have circumstances under which it will
   panic,
3. during a panic, all values are dropped.

Conclusion: even if we call our custom clean up function, drop may still be
called because of a panic (using `ManuallyDrop` defeats the purpose of
`prevent_drop`) and thus the compiler cannot elide the call and therefore the
linker will rightfully complain.

As unfortunate as it is, this means we usually will have to resort to run-time
checks. There are two major strategies: panicking and aborting. Aborting leads
to less code and guarantees the program does not recover, but it requires a
debugger to figure out where the problem comes from. You can choose a specific
strategy by using the appropriate macro or select the default strategy through
one of `prevent_drop`'s features.

## Reading material

 * https://users.rust-lang.org/t/prevent-drop-at-compile-time/20508
 * https://gankro.github.io/blah/linear-rust/

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
