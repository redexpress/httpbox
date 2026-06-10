# HTTPBox

[![build](https://github.com/redexpress/httpbox/actions/workflows/build.yml/badge.svg)](https://github.com/redexpress/httpbox/actions/workflows/build.yml)

A lightweight HTTP API debugging tool — a minimal Postman alternative. Built with Rust + egui.

## Features

- GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS
- Query params, Headers, JSON body with format
- Bearer Token / Basic Auth
- Multi-request tabs (in-memory)
- Response: status, headers, cookies, body (Pretty/Raw)
- Configurable timeout, keyboard shortcuts

## Install

Download from [Releases](https://github.com/redexpress/httpbox/releases) or build from source:

```bash
cargo build --release
```

## Usage

| Shortcut | Action |
|----------|--------|
| `Ctrl+Enter` | Send |
| `Ctrl+N` | New request |
| `Ctrl+Q` | Quit |

Set `HTTPBOX_LOG=info,httpbox=debug` for verbose logging.

## License

[Apache 2.0](LICENSE)
