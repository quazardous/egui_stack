#!/bin/sh
# Moves the release `knope release` just wrote to target/release-notes.md
# to the top of CHANGELOG.md, above the newest release, and stages it.
#
# knope's heading `## 0.23.0 (2026-11-05)` becomes
# `## egui_stack 0.23.0 - 2026-11-05`, and the lines continuing an entry
# are indented under its bullet, as in the hand-written entries.
set -eu

notes=target/release-notes.md
changelog=CHANGELOG.md

if [ ! -s "$notes" ]; then
    echo "release-notes: $notes is missing or empty; run \`knope release\`" >&2
    exit 1
fi

release=target/release-notes.section
# knope lists one-line entries first, then wrapped ones after a blank
# line; the awk pass drops that line so that a section is one tight
# list, and the trailing blank lines.
sed -E \
    -e 's/^## ([^ ]+) \(([^)]+)\)$/## egui_stack \1 - \2/' \
    -e '/^[^-#[:space:]]/s/^/  /' \
    "$notes" |
    awk '
        $0 == "" { blank = 1; next }
        blank {
            if (!($0 ~ /^- / && prev !~ /^#/)) print ""
            blank = 0
        }
        { print; prev = $0 }
    ' > "$release"

# Insert before the first release heading (the first `## ` line).
awk -v release="$release" '
    !done && /^## / {
        while ((getline line < release) > 0) print line
        print ""
        done = 1
    }
    { print }
    END {
        if (!done) {
            print ""
            while ((getline line < release) > 0) print line
        }
    }
' "$changelog" > "$changelog.new"
mv "$changelog.new" "$changelog"
rm -f "$notes" "$release"

git add "$changelog"
echo "release-notes: $(grep -m1 '^## ' "$changelog" | sed 's/^## //') written to $changelog"
