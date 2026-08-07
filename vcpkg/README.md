# PipeQL vcpkg Port

This directory contains the **vcpkg port** for the PipeQL C SDK (`libpipeql`).

- `ports/pipeql/vcpkg.json` — port metadata (name, version, license)
- `ports/pipeql/portfile.cmake` — build recipe (downloads source via GitHub,
  builds the Rust `pipeql-cffi` crate with cargo, installs `libpipeql.h` + the
  shared library)

## Publish to the official vcpkg registry (curated PR)

vcpkg uses **GitHub accounts**, not Microsoft/vcpkg.io logins. The Microsoft
sign-in page on vcpkg.io is unrelated to publishing.

1. Fork [`microsoft/vcpkg`](https://github.com/microsoft/vcpkg) on GitHub
2. Clone your fork and branch:

   ```bash
   git clone https://github.com/<you>/vcpkg.git && cd vcpkg
   git checkout -b add-pipeql
   ```

3. Copy the port folder in:

   ```bash
   cp -R <this-dir>/ports/pipeql ports/pipeql
   ```

4. Register the version and update the baseline:

   ```bash
   vcpkg x-add-version pipeql
   ```

5. Commit, push, and open a Pull Request against `microsoft/vcpkg`.
   Microsoft's CI builds the port on Windows/macOS/Linux triplets before merge.

## Use a custom Git registry (instant, no PR review)

Host the registry layout (`ports/` + `versions/` + `baseline.json`) in your own
repo and let consumers add it via `vcpkg-configuration.json`:

```json
{
  "registries": [
    {
      "kind": "git",
      "repository": "https://github.com/Flaxmbot/PipeQL-vcpkg-registry",
      "baseline": "<commit>",
      "packages": ["pipeql"]
    }
  ]
}
```

> **Note:** the port builds Rust on the consumer machine, so it requires a
> network connection for the first `cargo` fetch. `vcpkg_find_acquire_program`
> handles the toolchain; cross-compiling to non-host triplets additionally
> needs `rustup target add <triple>`.
