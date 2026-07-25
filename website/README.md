# Easy NATS website

This directory contains the minimal Vite shell for the browser demo. The desktop and browser
experiences share the same egui application code; browser commands are handled by the in-memory
demo backend.

## Local development

Requirements:

- Node.js 20 or newer
- pnpm 11
- Rust with the `wasm32-unknown-unknown` target
- `wasm-pack` 0.15

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
pnpm install
pnpm dev
```

`pnpm build` creates the static production output in `website/dist`.

Desktop-sized screens load the interactive WASM demo. Screens up to 720 px wide use the checked-in
preview image and do not initialize the WASM runtime.
