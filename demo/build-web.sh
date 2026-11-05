#!/bin/sh
# Builds the demo for the browser into demo/dist: the wasm module, its
# JavaScript glue and index.html, ready for any static file server.
#
# Needs the wasm32-unknown-unknown target (`rustup target add
# wasm32-unknown-unknown`) and the wasm-bindgen CLI at the version
# demo/Cargo.toml pins (`cargo install wasm-bindgen-cli --version <it>`).
# wasm-opt (binaryen), when installed, makes the module smaller.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
dist="$root/demo/dist"
target_dir=${CARGO_TARGET_DIR:-$root/target}

cd "$root"
cargo build -p egui_stack_demo --release --target wasm32-unknown-unknown
rm -rf "$dist"
wasm-bindgen --target web --no-typescript --out-dir "$dist" \
    "$target_dir/wasm32-unknown-unknown/release/egui_stack_demo.wasm"
if command -v wasm-opt > /dev/null; then
    wasm-opt -O2 --enable-bulk-memory --enable-nontrapping-float-to-int \
        "$dist/egui_stack_demo_bg.wasm" -o "$dist/egui_stack_demo_bg.wasm"
fi
cp demo/index.html "$dist/"
echo "Built $dist; serve it with, for example: python3 -m http.server -d demo/dist"
