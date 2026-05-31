# Npm Release Preparation

The first npm package is `jscpd-rs`. It exposes these bin commands:

- `jscpd-rs`: primary `npx jscpd-rs` entrypoint, runs the native `jscpd` CLI.
- `jscpd`: installed alias for the native `jscpd` CLI.
- `jscpd-server`: installed alias for the native server binary.

The package is intentionally source-build for the first release candidate:
`postinstall` runs `cargo build --release --locked --bin jscpd --bin
jscpd-server` inside the installed npm package. This keeps the npm path simple
and verifiable before publication. Users installing from npm need Node, npm, and
a Rust/Cargo toolchain. Prebuilt platform packages can be added later without
changing the CLI behavior.

Local verification:

```bash
scripts/npm-package-check.sh
```

That script verifies:

- `package.json` version matches `Cargo.toml`;
- `npm pack` includes the expected Rust source and npm shim files;
- forbidden paths such as `jscpd/`, `target/`, `report/`, `scripts/`, and
  `node_modules/` are not packed;
- `npm publish --dry-run --json` succeeds without publishing;
- a local npm install exposes working `jscpd-rs`, `jscpd`, and `jscpd-server`
  bin commands;
- `npx --package <local-tarball> jscpd-rs --version` works.

Before actual publication, run:

```bash
scripts/prepublish-check.sh
npm view jscpd-rs version
```

`npm view` should return `E404` for the first publication, or the package must
already be owned by this project. Do not run `npm publish` until explicit
release approval.
