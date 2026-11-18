#!/bin/sh
# Writes the crate's version (major.minor) into README.md's quick start
# (`egui_stack = "X.Y"`) and stages it. Run by `knope release` after the
# version is raised; `tests/readme_version.rs` fails when they disagree.
set -eu

version=$(sed -n 's/^version = "\([0-9]*\.[0-9]*\)\..*"/\1/p' Cargo.toml | head -n 1)
if [ -z "$version" ]; then
    echo "readme-version: no version in Cargo.toml" >&2
    exit 1
fi
sed -i "s/^egui_stack = \"[0-9.]*\"/egui_stack = \"$version\"/" README.md
git add README.md
