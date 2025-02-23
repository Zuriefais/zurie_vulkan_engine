[unix]
run:
    cargo fmt
    cargo zigbuild --release --package zurie_bin
    just build_mods
    mangohud RUST_LOG=info ./target/release/zurie_engine

[windows]
run:
    cargo fmt
    cargo zigbuild --release --package zurie_bin
    just build_mods
    RUST_LOG=info ./target/release/zurie_engine.exe

build_mods:
    cargo build --package vampire_like_demo --target wasm32-wasip2 --release


build_windows:
    cargo xwin build --release --target x86_64-pc-windows-msvc --package zurie_bin

[unix]
editor:
    hyprctl dispatch exec [workspace 5] zeditor $PWD

fix:
    cargo clippy --fix --allow-dirty

dev:
    mangohud RUST_LOG=info CARGO_PROFILE_DEV_CODEGEN_BACKEND=cranelift cargo +nightly run -Zcodegen-backend

test_ecs:
    cargo test --package zurie_ecs -- --nocapture

docs:
    #!/usr/bin/env fish
    cargo doc --no-deps --package zurie_mod_api
    open http://0.0.0.0:8000/zurie_mod_api/index.html
    simple-http-server target/doc/

run_android DEVICE:
    x run --package zurie_android --device {{DEVICE}} --release

zurie_render2:
    RUST_LOG=info mangohud cargo run --release --package zurie_render2
