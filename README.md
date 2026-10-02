# zixcel-version-control

Read the state of a local version-control workspace for a caller that needs change evidence.

## What you can do

- Inspect local repository metadata.
- Return a bounded observation for review.

## Current scope

This observer does not fetch, push, run hooks or change the working tree.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. Use the configured private registry when a dependency is not distributed publicly. Run from this repository:

```sh
cargo test --locked
```

## Documentation and source

[Usage guide](docs/getting-started.md)

[Implementation and public interfaces](src) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
