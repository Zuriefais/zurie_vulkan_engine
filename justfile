[unix]
run:
    cargo fmt
    cargo zigbuild --release --package zurie_core
    just build_mods
    mangohud RUST_LOG=info ./target/release/zurie_engine

[windows]
run:
    cargo fmt
    cargo zigbuild --release --package zurie_core
    just build_mods
    RUST_LOG=info ./target/release/zurie_engine.exe

build_mods:
    cargo build --package example_mod --target  wasm32-unknown-unknown --release

build_windows:
    cargo xwin build --release --target x86_64-pc-windows-msvc

[unix]
editor:
    hyprctl dispatch exec [workspace 5] zeditor $PWD

fix:
    cargo clippy --fix --allow-dirty

dev:
    mangohud RUST_LOG=info CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift cargo +nightly run -Zcodegen-backend
