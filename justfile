[unix]
run:
    cargo fmt
    cargo zigbuild --release
    mangohud RUST_LOG=info ./target/release/vulcan_engine

[windows]
run:
    cargo fmt
    cargo zigbuild --release
    RUST_LOG=info ./target/release/vulcan_engine.exe

build_windows:
    cargo xwin build --release --target x86_64-pc-windows-msvc

[unix]
editor:
    hyprctl dispatch exec [workspace 5] zeditor $PWD

fix:
    cargo clippy --fix --allow-dirty
