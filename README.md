> 当前访问策略：市场目录公开，普通登录用户可安装；上传仅限服务端邮箱白名单。见 [ADR-002](docs/ADR-002-public-read-restricted-upload.md)。

# EOS 统一插件平台

EOS 系列产品共用的插件合同、兼容策略、包与安装运行库。内部市场位于 eos-base，产品注册位于 eos-base-pro，EOS Agent 已接入。当前改动已完成本地链路验收，尚未部署生产或替换日常客户端。

- [架构](ARCHITECTURE.md) / [平台合同](docs/SPEC-plugin-platform-v1.md)
- [宿主接入与执行器矩阵](docs/HOST-INTEGRATION.md)
- [内部运行、密钥、迁移与交付](docs/OPERATIONS.md)
- [实施与验收记录](docs/IMPLEMENTATION-plugin-platform-v1.md) / [本地验证结果](docs/VERIFICATION-20261103.md)

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
python3 scripts/sync-consumers.py --check
cargo run -p eos-plugin-cli -- pack fixtures/agent-integration /tmp/fixture.eos-plugin
```

plugins.json 与以下格式是历史资料，不能通过正式运行入口安装；样例不是已审核市场内容。

## Current static index (legacy baseline)

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
