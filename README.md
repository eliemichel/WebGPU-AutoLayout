WebGPU Auto-layout
==================

Running like: https://eliemichel.github.io/WebGPU-AutoLayout

Building
--------

Install [Rust](https://www.rust-lang.org/tools/install), [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/) and [nodejs](https://nodejs.org/en/download).

```
cargo build --release
cd frontend
npm install
npm run build
```

Developing
----------

```
cd frontend
npm run start
```

The dev server automatically calls `cargo build` whenever the rust files change.

