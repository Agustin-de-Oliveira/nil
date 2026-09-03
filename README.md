# nil

A suite of focused, lightweight desktop utilities for Linux. Built with Rust, Tauri 2, Leptos (WASM), and Tailwind CSS.

---

## Overview

nil is an ecosystem of single-purpose, unobtrusive desktop applications designed for speed, visual coherence, and minimal resource usage. Each tool operates independently while sharing a unified architecture and design language.

Compatible natively with Wayland compositors (Hyprland, Sway, GNOME) and X11 sessions.

---

## Applications

### nil-shot
Fast and interactive screen capture and annotation tool.
- Instant focused-output capture or interactive area selection.
- Precision annotation tools: freehand pen, highlighter, arrows, rectangles, and ellipses.
- Sensitive information redaction with dynamic blur.
- On-screen OCR text and entity extraction (URLs, colors) with one-click copy.
- Full undo/redo history, canvas panning, and zoom magnifier.

### nil-clip
Minimalist clipboard history manager.
- Low-latency clipboard monitoring and persistent history.
- Real-time fuzzy filtering and search across copied snippets.
- Syntax-aware previews for code, links, and formatted text.
- Keyboard-first interaction model for rapid paste and retrieval.

### nil-notes
Distraction-free rich-text scratchpad and note editor.
- Fluid document hierarchy with automatic heading-to-body transitions.
- Markdown-compatible formatting: bold, italic, lists, and custom checklists.
- Reactive title synchronization with multi-tab persistence.
- Clean typography and adaptive dark/light interface.

---

## Design Principles

- **Speed and Efficiency**: Native Rust core ensuring near-instantaneous startup times and negligible idle resource usage.
- **Unobtrusive UX**: Keyboard-centric workflows designed to appear on demand and dismiss without friction.
- **Visual Coherence**: Shared aesthetic standards, typography, and layout proportions across all tools in the suite.

---

## Installation

Pre-built packages (AppImage, deb, rpm) and standalone binaries are available on the [Releases](https://github.com/Agustin-de-Oliveira/glint/releases) page.

---

## Development

For instructions on building from source, setting up the local environment, and contributing, see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## License

This project is licensed under the MIT License.
