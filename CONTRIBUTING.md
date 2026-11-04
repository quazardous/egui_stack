# Contributing

Issues and pull requests are welcome.

## AI-assisted contributions

There is no particular policy on them. A contribution written with an AI tool is reviewed like any other: it must
build, pass the checks below, follow the project's style, and its author answers for it. Saying in the pull request
that a tool helped is appreciated, not required.

## Pull requests

1. Fork the repository and create a branch for your change.
2. Open a pull request against `main`. A draft is fine for early feedback.

Before it is merged:

- `cargo fmt` (checked by CI);
- it compiles and passes `cargo clippy --all-targets --all-features -- -D warnings` and `cargo test --all-features`
  (checked by CI);
- a change users notice adds a changeset in `.changeset/` (see [.changeset/README](.changeset/README)); one users do
  not notice is labelled `internal`. CHANGELOG.md is written from the changesets at release time
  ([RELEASING.md](RELEASING.md)), not by hand;
- public API has documentation comments; a major feature has an example in `examples/`.

## Issues

Bug reports should say how to reproduce the bug, if known.
