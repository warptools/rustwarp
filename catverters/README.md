catverters
==========

It's about data con**verters** that do a lot of string con**cat**enation.  Catverters.


What.
-----

The idea is you might have some Rust structures like:

```rust
enum VariousThings {
    A(ThingA),
    B(ThingB),
    C(ThingC),
}

struct ThingA {
    val: String
}

struct ThingB {
    other_val: String
}

struct ThingC {
    complex: String
    tuples: String
}
```

... and maybe some strings like:

- `"a:value"`
- `"b:other-value"`
- `"c:this-triggers:recursive-parsing"`

... are valid ways to display values that inhabit the `VariousThings` type, and should also be parsable into those types.

In other words, those values pack down nicely into single strings, using delimiters to separate fields and discriminator values.
And should be parsable from the same.
This might be desirable if you're making some kind of API that really values terseness,
or you have some values like this that you really need to display to a user in a concise way.

Catverters do that.

This crate doesn't do much itself.
But check out the sibling "catverters-derive" crate,
which has macros that generate `Display` and `FromStr` impls that do the above.

Pairs well with Serde.
The `serde_with` crate has bridges to `Display` and `FromStr` traits,
which lets you use a combination of catverters together with Serde,
thus letting you have densely packed strings representing complex structures and enums
that can make things one-liners... even when they're in the middle of, say, other complex JSON objects.
It's neat.


What's in `catverters` vs `catverters-derive`?
----------------------------------------------

Rust has this rule that you can't have macros in the same crate as anything _else_.
So this crate, `catverters`, contains some library code that you end up depending on,
while `catverters-derive` contains all the macros.

The majority of the lifting is in the `catverters-derive` crate.
This one really only contains some error types.
(We really like well-typed errors around here, so this was important to us.)
