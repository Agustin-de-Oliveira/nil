# Contributing to nil

This document covers architecture, development workflows, and guidelines for contributing to the nil suite.

## Architecture

nil is structured as a modular Cargo workspace combining Rust backends (Tauri v2) and web frontends (Leptos CSR compiled to WebAssembly with Trunk and Tailwind CSS v4).

```text
nil/
├── Cargo.toml          Workspace manifest
├── Makefile            Development and build orchestration
├── bin/                Local build tools (trunk, tailwindcss)
├── crates/
│   ├── nil-core/       Shared library (clipboard, OCR, system utilities)
│   ├── nil-shot/       Screen capture and annotation backend
│   ├── nil-clip/       Clipboard history manager backend
│   └── nil-notes/      Notes editor backend
└── frontend/
    ├── nil-shot/       Screen capture UI (dev server on :1420)
    ├── nil-clip/       Clipboard manager UI (dev server on :1421)
    └── nil-notes/      Notes editor UI (dev server on :1422)
```

## System Requirements

### Build Dependencies
- Rust (stable toolchain) with target `wasm32-unknown-unknown`:
  ```bash
  rustup target add wasm32-unknown-unknown
  ```
- Build tools available in `bin/` or system path:
  - Trunk
  - Tailwind CSS v4 CLI

### Runtime Dependencies
On Arch Linux:
```bash
sudo pacman -S grim slurp wl-clipboard tesseract tesseract-data-eng tesseract-data-spa libnotify
```

On Debian / Ubuntu:
```bash
sudo apt install grim slurp wl-clipboard tesseract-ocr tesseract-ocr-eng tesseract-ocr-spa libnotify-bin
```

## Development Workflows

The root `Makefile` orchestrates development servers, checks, and release compilation.

### Running in Development Mode
Each application runs with Trunk serving the frontend with hot reload and Tauri attached to the development URL.

- Start nil-shot (port 1420):
  ```bash
  make shot
  ```
- Start nil-clip (port 1421):
  ```bash
  make clip
  ```
- Start nil-notes (port 1422):
  ```bash
  make notes
  ```

Closing the application window or sending `SIGINT` (Ctrl+C) automatically terminates the background Trunk process.

### Code Verification
To run type and borrow checking across all workspace crates and frontends:
```bash
make check
```

### Release Compilation
To compile optimized frontends and release binaries in parallel:
```bash
make build
```

Individual application releases can be compiled via:
```bash
make build-shot
make build-clip
make build-notes
```

To clean build artifacts:
```bash
make clean
```

## Code Guidelines

- Simplicity first: Prefer reducing surface area and consolidating logic over adding layers, flags, or abstractions.
- No comments in code: Intent should be expressed through explicit naming, minimal structure, and tests. Explanations belong in commit messages or pull requests.
- Dependencies: Avoid introducing new dependencies unless strictly necessary and discussed beforehand.
- Consistency: Keep formatting and architecture symmetrical across all crates and frontends.
