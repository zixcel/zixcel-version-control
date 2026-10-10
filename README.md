# zixcel-version-control

Read the state of a local version-control workspace for a caller that needs change evidence.

## What you can do

- Inspect local repository metadata.
- Return a bounded observation for review.

## Current scope

This observer does not fetch, push, run hooks or change the working tree.

This is an independently packaged Rust library. Cargo dependencies are resolved from crates.io; runtime authority, network and storage configuration remain caller-owned.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. No private dependency registry or sibling source checkout is required. Run from this repository:

```sh
cargo test --locked
```

## Documentation and source

[Usage guide](docs/getting-started.md)

[Implementation and public interfaces](src) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
