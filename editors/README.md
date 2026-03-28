# Txtcode — VS Code Extension

Provides language support for `.tc` / `.txtcode` files in Visual Studio Code.

## Features

- Syntax highlighting (TextMate grammar)
- Code snippets
- Go-to-definition, hover, rename — via the `txtcode lsp` language server
- Inline diagnostics (parse errors, type warnings) as you type

## Requirements

- VS Code 1.85+
- The `txtcode` binary must be installed and on PATH (used as the language server)

## Build

This is a Node.js project. The Rust build (`cargo build`) does not touch this directory.

```sh
cd editors/
npm install
npx vsce package        # produces txtcode-0.1.0.vsix
```

Install locally:
```sh
code --install-extension txtcode-0.1.0.vsix
```

## How it works

The extension starts `txtcode lsp` as a child process and communicates with it over
stdin/stdout using the Language Server Protocol (LSP). The Rust-side LSP server
(`src/cli/lsp.rs`) handles:

- `textDocument/completion`
- `textDocument/definition`
- `textDocument/hover`
- `textDocument/rename`
- `textDocument/publishDiagnostics`

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `txtcode.serverPath` | `"txtcode"` | Path to the txtcode binary |
| `txtcode.trace.server` | `"off"` | LSP trace level (`off` / `messages` / `verbose`) |

## Publish (deferred)

Publishing to the VS Code Marketplace requires a personal access token and publisher
account. Deferred until the language reaches a stable public release.

```sh
npx vsce publish        # requires VSCE_PAT environment variable
```
