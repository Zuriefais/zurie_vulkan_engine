run:
    cargo fmt
    cargo zigbuild --release
    mangohud RUST_LOG=info RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=mold" ./target/release/vulcan_engine

editor:
    hyprctl dispatch exec [workspace 5] zeditor $PWD

fix:
    cargo clippy --fix --allow-dirty
