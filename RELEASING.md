# Releasing

Versions and changelog entries come from **changesets**: each pull
request changing what users get adds one in `.changeset/` (the format is
in [.changeset/README](.changeset/README)), and
[knope](https://knope.tech) turns the pending ones into a version and
its CHANGELOG.md entries at release time. Entries released before
changesets (up to 0.22.1) stay as written.

knope is a development tool, not a dependency:
`cargo install knope --version 0.23.2 --locked`, the version CI pins
(`KNOPE_VERSION` in `.github/workflows/`).

## On pull requests

CI's **Changesets** job (`scripts/changeset-check.sh`) fails when a
pending changeset is malformed (knope would silently skip a misspelt
package or kind), or when the pull request changes `src/` and adds no
changeset, unless it is labelled `internal`; adding the label runs CI
again. It then prints what `knope release --dry-run` would do. Locally:

```sh
sh scripts/changeset-check.sh origin/main
knope release --dry-run
```

## Making a release

Publishing is the owner's call, and stays a manual step.

1. **Prepare**, on a branch from an up-to-date `main`:

   ```sh
   git switch -c release-X.Y.Z origin/main
   knope release --dry-run   # the version and the entries
   knope release
   git commit -s -m "chore: release X.Y.Z"
   ```

   `knope release` raises the version in `Cargo.toml` from the pending
   changesets and deletes them; `scripts/release-notes.sh` then writes
   the release at the top of CHANGELOG.md, as
   `## egui_stack X.Y.Z - DATE` with its "Breaking changes", "Added" and
   "Fixed" sections, and stages it. Nothing is committed: read the
   entry, touch it up if needed (a sentence above the sections, as 0.22.0
   has, is written by hand), then commit.

2. **Review**: push the branch and open a pull request titled
   `chore: release X.Y.Z`. CI's semver job checks the new version against
   crates.io. Merge it.

3. **Publish**, from the merged commit on `main`:

   ```sh
   cargo publish --dry-run
   cargo publish
   ```

4. **Tag** the published commit and push the tag:

   ```sh
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

## Semver checks

CI's **Semver checks** job first runs `knope pending-versions`, which
writes in its checkout (and commits nothing) the version the pending
changesets lead to, then `cargo semver-checks check-release`, which
compares the crate with its latest release on crates.io. So the
changeset's kind is what is checked: a breaking change passes when a
`major` changeset makes the version a new minor (0.22.1 to 0.23.0) and
fails when it is declared `minor` or `patch`, naming what broke. A
release pull request, whose version is already raised, is checked the
same way.

`cargo semver-checks` sees the Rust API only. A break it cannot see (a
changed default, a different behaviour, a new egui version) is still a
`major`: declare what the change is.
