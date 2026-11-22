# For coding agents

Agents are welcome here, under the same rules as anyone (see CONTRIBUTING.md).

- Build: `cargo build`; examples: `cargo build -p egui_stack_demo --examples`.
- Checks before a pull request: `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
  `cargo test --all-features`, `cargo doc --no-deps --all-features`.
- Library code in `src/`, examples in `demo/examples/`, the demo in `demo/` (a separate crate; see its README).
- A change users notice adds a changeset in `.changeset/` (format in `.changeset/README`); do not edit
  CHANGELOG.md, which is written at release time (RELEASING.md). Check: `sh scripts/changeset-check.sh origin/main`.
