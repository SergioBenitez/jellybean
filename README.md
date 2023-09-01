# Jellybean

Syntax highlighting library using tree-sitter.

## Work in Progress

This is almost ready for release. But not quite yet. To build, first sync:

```sh
cargo xtask sync
```

Run some examples:

```sh
cargo run -p example-bat -- file/to/highlight.ext
cargo run -p example-html -- file/to/highlight.ext output/file.html
cargo run -p example-cached
```
