# HotFIX, an experimental FIX engine

A FIX engine implemented in Rust with `rustls` support.

> **Warning**
>
> HotFIX is an experimental project, currently incomplete and it may never be completed.
> It is not fit for production.

For the time being, HotFIX uses [ferrumfix](https://github.com/ferrumfix/ferrumfix)'s
amazing message encoding and decoding, and focuses on implementing the
transport and session layer.
