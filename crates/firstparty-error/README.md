# firstparty-error

`firstparty-error` is a tiny error/context crate for applications that want owned,
auditable error plumbing without a derive macro or broad dependency tree.

It provides:

- `Error`: a simple message/source error type.
- `Result<T>`: a crate-local result alias.
- `ResultContext`: `context` and `with_context` helpers for adding stable operator-facing messages.

It intentionally does not know about any application domain. Domain crates should
build typed errors around it or convert their typed errors into it at executable
boundaries.
