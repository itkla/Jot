# Jot

A minimal, cross-platform notepad app built with Tauri v2 and Rust. Inspired by the simplicity of classic MS Notepad.

## Features

- **Ultra-minimal UI**: Full-window text editor with no visual clutter
- **Fast file operations**: Native Rust backend for instant performance
- **Essential shortcuts**: Cmd+N (new window), Cmd+O (open), Cmd+S (save), Cmd+Shift+S (save as), Cmd+Shift+N (clear)
- **Smart confirmations**: Native dialogs for unsaved changes
- **Cross-platform**: Works on macOS, Windows, and Linux

## Development

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

## Tech Stack

- **Backend**: Rust with Tauri v2
- **Frontend**: Vanilla HTML/CSS/JavaScript
- **File System**: Native OS dialogs and file operations
