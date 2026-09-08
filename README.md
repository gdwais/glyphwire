# Octomark

A fast, local Markdown editor and live preview for macOS, written in Rust.

## Features

- Open a directory or a Markdown file from Terminal
- Browse all files and expandable subdirectories
- Edit raw Markdown beside a live preview
- Auto-save after you stop typing
- CommonMark plus tables, task lists, strikethrough, footnotes, and highlighted code blocks
- Local and remote images
- Clickable links

## Install

Install Rust with [rustup](https://rustup.rs/) if needed, then run:

```sh
cd ~/Development/octomark
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
octomark .
```

Or open a Markdown file directly:

```sh
octomark README.md
```

Files with `.md` and `.markdown` extensions are editable. Other files remain visible but disabled in the file browser.

## Development

```sh
cargo run -- .
cargo test
```
