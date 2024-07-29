run:
    RUST_LOG=info RUSTFLAGS="-C linker=clang -C link-arg=-fuse-ld=mold" cargo run --release

editor:
    hyprctl dispatch exec [workspace 5] zeditor $PWD
