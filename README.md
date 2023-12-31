<div align="center">

# HotFIX

**An experimental FIX engine written in Rust.**

[![CI](https://github.com/Validus-Risk-Management/hotfix/actions/workflows/test.yml/badge.svg)](https://github.com/Validus-Risk-Management/hotfix/actions/workflows/test.yml)
[![crates-badge]](https://crates.io/crates/hotfix)
[![docs-badge]](https://docs.rs/hotfix)
[![Crates.io](https://img.shields.io/crates/l/hotfix)](LICENSE)

</div>


> **Warning**
>
> HotFIX is an experimental project, currently incomplete and a long way from being complete.
> It is not fit for production.

### Overview

HotFIX is a FIX engine implemented in Rust. While the ambition is to create a robust,
fully compliant, ergonomic and performant engine eventually, this is a large undertaking.

The primary objective of HotFIX is to provide a functional and useful engine for initiators,
supporting FIX 4.4 and 5.0, as soon as possible. This has meant using existing solutions
where possible, prioritising functional components over performance and moving fast with
experimental code rather than good code at this stage.

### What's working already

- [x] TCP transport
- [x] TLS support using `rustls`
- [x] Basic message encoding and decoding (FIX 4.4)
- [x] Persistent message store using [redb](https://www.redb.org/)
- [x] Heartbeats, logon, reconnecting sessions
- [x] Basic logic for sending messages
- [x] Basic logic for receiving messages
- [ ] Resend flows - coming next

Check out the [examples](https://github.com/Validus-Risk-Management/hotfix/tree/main/examples)
to get started.

### Prior Art

The two major influences for HotFIX are QuickFIX and [FerrumFIX](https://ferrumfix.org/).

QuickFIX implementations in various languages (such as [QuickFIX/J](https://quickfixj.org/))
have influenced the designs of the transport and session layers. The FIX message logic
builds on QuickFIX XMLs for the specification.

The FIX message implementation of HotFIX leverages FerrumFIX for things like codegen,
parsing the XML specification, defining fields, etc. This may change in the future,
but re-using FerrumFIX code has proved useful in expediting the process of reaching
a functional engine.

### Contributions

In its current state, the engine has a lot of issues that will be fixed
in due course, so please don't create issues or PRs for individual bugs.

We welcome committed contributors who want to work with us to turn this
into a successful project. There are many components that can be developed
in parallel. If you are interested in participating, don't hesitate to
reach out.

The best way to get in touch is by
[starting a Discussion](https://github.com/Validus-Risk-Management/hotfix/discussions).

[crates-badge]: https://img.shields.io/crates/v/hotfix.svg
[docs-badge]: https://docs.rs/hotfix/badge.svg
