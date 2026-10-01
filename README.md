# zixcel-version-control

`zixcel-version-control` owns bounded, read-only observations of a local
version-control workspace. It does not define project compilation, decide that
work is complete, authenticate to a remote service, push, fetch, run hooks, or
copy source content.

The provider-neutral `SourceRevisionObservation` remains useful without Git.
The included Git adapter maps immutable commit/tree identities and the current
worktree state onto that contract. Branch names are mutable display hints only.
Remote GitHub/GitLab APIs remain separate provider packages.

An absent repository is `Ok(None)`: consumers must preserve their non-versioned
workflow. An explicitly selected but broken repository returns a typed error.

```bash
cargo test
```

## Package integration

The package is an independently consumable unit. Callers reference its documented
interface through a versioned dependency and own application-specific composition
and integration.
