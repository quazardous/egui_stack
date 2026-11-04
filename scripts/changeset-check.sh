#!/bin/sh
# Checks the changesets of a pull request (RELEASING.md).
#
#   scripts/changeset-check.sh <base> [--internal]
#
# Fails when a pending changeset is malformed (knope ignores a misspelt
# package or bump without a word), or when the pull request changes
# `src/` and adds no changeset, unless it is labelled `internal`
# (`--internal`).
set -eu

if [ $# -lt 1 ]; then
    echo "usage: $0 <base> [--internal]" >&2
    exit 2
fi
base=$1
internal=${2:-}
failed=0

fail() {
    echo "changeset-check: $*" >&2
    failed=1
}

# Every pending changeset: a front matter of exactly one line,
# `egui_stack: patch|minor|major`, then some text.
for file in .changeset/*.md; do
    [ -e "$file" ] || continue
    if [ "$(sed -n 1p "$file")" != "---" ]; then
        fail "$file: no front matter (it starts with a \`---\` line)"
        continue
    fi
    front=$(awk 'NR > 1 && $0 == "---" { exit } NR > 1 { print }' "$file")
    case $front in
        "egui_stack: patch" | "egui_stack: minor" | "egui_stack: major") ;;
        *) fail "$file: the front matter must be one line, \`egui_stack: patch\`, \`minor\` or \`major\`; it is: $front" ;;
    esac
    text=$(awk 'n >= 2 && NF { print } $0 == "---" { n++ }' "$file")
    if [ -z "$text" ]; then
        fail "$file: no text below the front matter"
    fi
done

# A change to the crate's code comes with a changeset.
changed=$(git diff --name-only "$base"...HEAD -- src)
added=$(git diff --name-only --diff-filter=A "$base"...HEAD -- '.changeset/*.md')
if [ -n "$changed" ] && [ -z "$added" ]; then
    if [ "$internal" = "--internal" ]; then
        echo "changeset-check: src/ changes without a changeset, labelled internal"
    else
        fail "this pull request changes src/ and adds no changeset in .changeset/; add one (see .changeset/README), or label the pull request \`internal\` if users notice nothing"
    fi
fi

if [ "$failed" -ne 0 ]; then
    exit 1
fi
echo "changeset-check: ok"
