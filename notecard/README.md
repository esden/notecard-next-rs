# Rust driver for the Blues Notecard

This driver is for the [blues.io](https://blues.io) [notecard](https://blues.io/products/notecard/).

Great thanks to @gauteh for the original driver that inspired a lot of the design decisions. It is not a fork but rather a full rewrite of the driver to allow us to support async and uart.

This driver is based on the APIs defined by embedded-hal, embedded-hal-async, embedded-io and embedded-io-async. Providing the ability to be used on non async systems through the use of futures_lite::block_on.

The goal of this crate is to implement the whole notecard API while allowing the user to implement custom requests in cases where our driver is lagging behind the official specification as well as in cases where the user wants to implement some optimizations.