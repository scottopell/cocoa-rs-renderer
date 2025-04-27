# `jp2view`

A simple macOS application for viewing JP2 image files (work in progress).

## Current Features
- File selection dialog for JP2 files
- Placeholder image generation (text, gradient, checkerboard)
- Image navigation with pan and zoom
- Gesture support for pinch-to-zoom

## Status
This application currently shows placeholder images instead of actual JP2 content. Full JP2 decoding support is coming soon.

## Requirements
- macOS
- Rust toolchain

## Building
```
cargo run --release
```