# EOS Plugin Index

This repository hosts the official plugin index for EOS CLI and EOS App.

## Format

`plugins.json` is a JSON array of plugin entries:

```json
[
  {
    "name": "my-plugin",
    "description": "What it does",
    "version": "1.0.0",
    "author": "you",
    "source": "https://github.com/you/your-plugin-repo",
    "permissions": ["network"],
    "tags": ["search", "category"]
  }
]
```

## Submitting a plugin

1. Create a repo with a `plugin.toml` manifest
2. Open a PR adding your entry to `plugins.json`
3. The `source` field should be your repo's git URL

## Plugin manifest format (plugin.toml)

```toml
[plugin]
name = "my-plugin"
version = "1.0.0"
description = "What it does"
permissions = ["network"]

[tools]
type = "mcp"
command = "npx"
args = ["-y", "@example/mcp-server"]

skills = ["my-skill"]
```
