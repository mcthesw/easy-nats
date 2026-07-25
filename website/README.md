# Easy NATS website

This directory contains the static Vue shell for the Easy NATS live demo and
installation page. Vue owns only the public page; the desktop and browser demos
share the same egui application code.

## Local development

Requirements:

- Node.js 20 or newer
- pnpm 11
- Rust with the `wasm32-unknown-unknown` target
- `wasm-pack` 0.15 available on `PATH`

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-pack --version 0.15.0 --locked
pnpm install --frozen-lockfile
pnpm dev
```

Use `pnpm typecheck` for the Vue/TypeScript check. `pnpm build` rebuilds the
Rust WASM package and creates the complete static production site in
`website/dist`. `pnpm preview` serves that output locally after a build.

Screens at least 900 px wide load the interactive WASM demo. Smaller screens
use the checked-in desktop preview and do not request the WASM runtime. Language
and theme choices are intentionally in-memory and reset from browser/system
preferences on refresh.

## Cloudflare Pages

The build assumes deployment at the root of a domain and needs no Pages
Functions, runtime variables, redirects, or custom headers.

For a Dashboard Direct Upload:

1. Run `pnpm build`.
2. In Cloudflare, open **Workers & Pages** and create a Direct Upload project.
3. Upload the `website/dist` directory.
4. In the Pages project, open **Custom domains**, select **Set up a domain**,
   and bind the desired domain or subdomain.

For Wrangler Direct Upload from this directory:

```sh
pnpm build
pnpm dlx wrangler@latest login
pnpm dlx wrangler@latest pages project create
pnpm dlx wrangler@latest pages deploy dist --project-name <PROJECT_NAME>
```

Cloudflare documents both deployment paths in its
[Direct Upload guide](https://developers.cloudflare.com/pages/get-started/direct-upload/)
and describes domain binding in
[Custom domains](https://developers.cloudflare.com/pages/configuration/custom-domains/).
Choose the project mode deliberately: Cloudflare currently does not allow a
Direct Upload project to be converted to Git integration later.
