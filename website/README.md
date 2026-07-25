# Easy NATS website

## Local development

Requirements:

- Node.js 22.13 or newer
- pnpm 11
- Rust with the `wasm32-unknown-unknown` target
- `wasm-pack` 0.15 available on `PATH`

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
pnpm install --frozen-lockfile
pnpm dev
```

```sh
pnpm typecheck
pnpm build
pnpm preview
```

`pnpm build` rebuilds the Rust WASM package and writes the static site to
`dist`. Run `pnpm preview` after a build to preview that output locally.
