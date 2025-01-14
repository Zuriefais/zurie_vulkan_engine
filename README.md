# Zurie Vulkan Engine

A modern game engine powered by WebAssembly for code execution and Vulkan for rendering.

## Features

- WASI-based modding system
- Vulkan rendering backend
- Entity Component System (ECS)
- Cross-platform support
- Sprite rendering system

## Project Status

### Completed ✅
- [x] Sprite rendering system
- [x] Entity Component System (ECS)
- [x] Basic code execution
- [x] WASI component model migration
- [x] Migration to standard logging crate

### In Progress 🚧
- [ ] Demo game (Vampire Survivors-like)
- [ ] Snake game implementation
- [ ] Sprite animation system
- [ ] Full Android support

## Getting Started

### Prerequisites

- Nightly rust toolchain and wasm32-wasip2 target
- [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild)
- [mangohud](https://github.com/flightlessmango/MangoHud) (Linux debug only)
- Vulkan SDK
- [xbuild](https://github.com/rust-mobile/xbuild)
- CMake
- Ninja

### Creating a Mod

1. Create a new Rust project
2. Add `zurie_mod_api` to your dependencies:
   ```toml
   [dependencies]
   zurie_mod_api = "0.1.0"
   ```
3. Implement the `Mod` trait
4. Compile your mod
5. Launch the engine
6. Load your mod into the engine

## Platform Support

| Platform | Status |
|----------|---------|
| Windows  | ✅ Supported |
| Linux    | ✅ Supported |
| macOS    | ⚠️ Experimental |
| Android  | 🚧 In Progress |

## Running the Engine

### Desktop
```bash
just
```

### Android
```bash
x run --device <device-id> --package zurie_android
```

## Documentation

- [WASM Component Model](https://component-model.bytecodealliance.org/)
- [Rust Documentation](https://www.rust-lang.org/learn)
- [Wasmtime Documentation](https://docs.wasmtime.dev/)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
