# Glyphwire

A fast, cyberpunk-styled local Markdown editor and live preview for macOS, written in Rust.

## Features

- Open a directory or a Markdown file from Terminal
- Keep multiple files open in tabs, or use **New Window** for side-by-side windows
- Browse all files and expandable subdirectories; right-click files to delete with confirmation
- Edit raw Markdown beside a live preview, or **Hide raw** / **Show raw** to focus on reading
- **Copy Markdown** copies the entire current source, even with the raw pane hidden
- Click preview task checkboxes to toggle `[ ]` / `[x]` in the Markdown
- Linked proportional scrolling between source and preview panes
- Auto-save edits and checkbox changes, including in background tabs
- Larger Source Sans 3 reading font, JetBrains Mono source font, and higher-contrast labels
- CommonMark plus tables, task lists, strikethrough, footnotes, and highlighted code blocks
- Local and remote images
- Clickable links
- Cyberpunk-inspired neon interface
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
| Hide/reopen raw editor | **Hide raw** / **Show raw**, Cmd/Ctrl+Shift+E |
| Copy all raw Markdown | **Copy Markdown** in the toolbar |
| Save all open tabs now | Cmd/Ctrl+S |
| Toggle a task | Click its checkbox in the preview |
| Delete a file | Right-click its name → **Delete file…**, then confirm |

Deletion is permanent (not moved to Trash). Deleting an open file closes its tab and discards its unsaved changes after confirmation. Failed saves keep the tab/window open and display an error with a retry button.

### Fonts

Bundled [Source Sans 3](https://github.com/adobe-fonts/source-sans) and [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono) are licensed under the SIL Open Font License; license files are in `assets/fonts/`.

## Development

```sh
cargo run -- .
cargo test
cargo clippy --all-targets -- -D warnings
```
