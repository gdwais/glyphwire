# Glyphwire

A fast, cyberpunk-styled local Markdown editor and live preview for macOS, written in Rust.

## Features

- Open a directory or a Markdown file from Terminal
- Keep multiple files open in tabs, or use **New Window** for side-by-side windows
- Browse all files and expandable subdirectories; right-click files to delete with confirmation
- Edit raw Markdown beside a live preview, or **Hide raw** / **Show raw** to focus on reading
- **Copy Markdown** copies the entire current source, even with the raw pane hidden
- **Find** (Cmd/Ctrl+F) searches the current source with case-insensitive matching, result counts, and highlighted results
- Dark-grey selection backgrounds keep selected text readable
- Click preview task checkboxes to toggle `[ ]` / `[x]` in the Markdown
- Linked proportional scrolling between source and preview panes
- Auto-save edits and checkbox changes, including in background tabs
- Larger Source Sans 3 reading font, JetBrains Mono source font, and higher-contrast labels
- CommonMark plus tables, task lists, strikethrough, footnotes, and highlighted code blocks
- Local and remote images
- Clickable links
- Cyberpunk-inspired neon interface
- Native Metal rendering via wgpu (no deprecated macOS OpenGL backend)
- Non-blocking CLI—the terminal is released as soon as the window launches

## Install

Install Rust with [rustup](https://rustup.rs/) if needed, then run:

```sh
cd ~/Development/glyphwire
cargo install --path .
```

Make sure Cargo's binary directory is on your `PATH`:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

Add that line to `~/.zshrc` if it is not already present.

## Use

From any directory:

```sh
glyphwire .
```

The command launches Glyphwire in a separate process and immediately returns control to your terminal.

Or use the short `gw` alias:

```sh
gw .
```

Open a Markdown file directly with either command:

```sh
glyphwire README.md
gw README.md
```

Files with `.md` and `.markdown` extensions open in their own tabs. Selecting an already-open file returns to its tab without losing edits or scroll position. Other files remain visible and can also be deleted from the right-click menu, but cannot be edited.

Use **New Window** to open another browser for the same folder, or run `gw <PATH>` again to open a different folder or file alongside the existing window.

### Controls

| Action | Control |
| --- | --- |
| New window | **New Window**, Cmd/Ctrl+N |
| Close current tab | Tab **×**, Cmd/Ctrl+W |
| Collapse/expand file picker | **Hide files** / **Show files** in the toolbar |
| Hide/reopen raw editor | **Hide raw** / **Show raw**, Cmd/Ctrl+Shift+E |
| Copy all raw Markdown | **Copy Markdown** in the toolbar |
| Find in current document | **Find**, Cmd/Ctrl+F (opens raw editor) |
| Next / previous match | Enter / Shift+Enter in search, or Cmd/Ctrl+G / Cmd/Ctrl+Shift+G |
| Close search | Esc or search **×** |
| Save all open tabs now | Cmd/Ctrl+S |
| Toggle a task | Click its checkbox in the preview |
| Delete a file | Right-click its name → **Delete file…**, then confirm |

Deletion is permanent (not moved to Trash). Deleting an open file closes its tab and discards its unsaved changes after confirmation. Failed saves keep the tab/window open and display an error with a retry button.

### Fonts

Bundled [Source Sans 3](https://github.com/adobe-fonts/source-sans) and [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) are licensed under the SIL Open Font License; license files are in `assets/fonts/`.

### Graphics crashes on display changes

Version 0.2.1 replaces OpenGL with Metal to avoid the AppKit `NSOpenGLContext` / `CGLFlushDrawable` crash path observed during display reconfiguration. Lost or outdated rendering surfaces are reconfigured on subsequent frames. This is a backend mitigation; it does not guarantee that every graphics-driver crash is resolved.

After updating with `cargo install --path . --locked`, save and close all old Glyphwire windows, then launch `gw` again. Existing processes continue using the old renderer until restarted. Check the installed version with `gw --version`.

For foreground diagnostics (without the normal detached launcher), run:

```sh
RUST_BACKTRACE=1 gw --glyphwire-gui-process README.md
```

## Development

```sh
cargo run -- .
cargo test
cargo clippy --all-targets -- -D warnings
```

Renderer smoke test on macOS: launch both `gw` and `glyphwire`; resize/minimize/restore their windows, move them between displays with different scaling, disconnect/reconnect an external display, and sleep/wake. Verify rendering resumes and editing/autosave still work. Unit tests cover renderer selection and surface-error handling, not these OS-level transitions.
