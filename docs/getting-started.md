# Using zixcel-version-control

Read the state of a local version-control workspace for a caller that needs change evidence.

## Before you start

This observer does not fetch, push, run hooks or change the working tree.

## First steps

Run from the repository root:

```sh
cargo test --locked
```

## How to assess the result

- Inspect local repository metadata.
- Return a bounded observation for review.

A passing source-level check establishes only what that check observes. Keep missing configuration, unavailable services and unverified deployment paths visible.

## Continue reading

[Repository overview](../README.md)
