catverters
==========

It's about data con**verters** that do a lot of string con**cat**enation.  Catverters.

What.
-----

It's macros that generate `Display` and `FromStr` impls.

The main macro, `#[derive(catverters::Stringoid)]`, works on enums and structs (including recursively).
It lets you pack those values down nicely into single strings, using delimiters to separate fields and discriminator values.
That, in turn, helps you rapidly develop APIs where keystrokes and linebreaks are precious and worth conserving.

If you want to use these in serialization:
catverters pair up really well with `serde_with`!
Try this: `#[derive(serde_with::SerializeDisplay, serde_with::DeserializeFromStr, catverters::Stringoid)]`!
The combination of catverters with serde lets you have densely packed strings representing complex structures and enums
that can make things one-liners... even when they're in the middle of, say, other complex JSON objects.
It's neat.
