# workspace-node-tools

[![Crates.io](https://img.shields.io/crates/v/workspace-node-tools.svg)](https://crates.io/crates/workspace-node-tools)
[![Docs.rs](https://docs.rs/workspace-node-tools/badge.svg)](https://docs.rs/workspace-node-tools)
[![CI](https://github.com/websublime/workspace-node-tools/workflows/CI/badge.svg)](https://github.com/websublime/workspace-node-tools/actions)

## About

This is a tool to help manage packages in a monorepo style. It can give info about packages existence, package manager defined (node), git helpers to check which package as changes, manage those changes thur a file (.changes.json), give output of conventional commit and changelog generation.

## Installation

`cargo install workspace-node-tools`

### Cargo

- Install the rust toolchain in order to have cargo installed by following
  [this](https://www.rust-lang.org/tools/install) guide.
- run `cargo install workspace-node-tools`

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

See [CONTRIBUTING.md](CONTRIBUTING.md).

## Info

Template from [here](https://rust-github.github.io/)

TODO:

- SemVersion regex

```rust
let regex = Regex::new(r"(?m)(?<major>^(0|[1-9]\d*))\.(?<minor>(0|[1-9]\d*))\.(?<patch>(0|[1-9]\d*))(?<pre>(-[a-zA-Z\d][-a-zA-Z.\d]*))?(?<meta>(\+[a-zA-Z\d][-a-zA-Z.\d]*))?$").unwrap();
```

- PackageInfo url buggy when url is git+ssh
- Bump release_as not being used
- Add Rc bump
- Change snapshot bum without 0
