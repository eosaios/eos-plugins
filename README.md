> 当前访问策略：市场目录公开，普通登录用户可安装；上传仅限服务端邮箱白名单。见 [ADR-002](docs/ADR-002-public-read-restricted-upload.md)。

# EOS 统一插件平台

EOS 系列产品共用的插件合同、兼容策略、包与安装运行库。市场位于 eos-base，产品注册位于 eos-base-pro，EOS Agent 已接入。[独立插件市场](https://plugins.eosaios.com/)提供公开浏览入口，登录后可下载和安装；上传由服务端配置的邮箱白名单限制，名单数量和内容以目标环境配置为准。

## 开发插件

从[插件开发手册](docs/developer/README.md)开始：

- [从零开发第一个插件](docs/developer/QUICKSTART.md)
- [清单、权限与兼容性参考](docs/developer/MANIFEST.md)
- [Skills / 页面 SDK / MCP / WASM 模块开发](docs/developer/MODULES.md)
- [调试、日志与排障](docs/developer/DEBUGGING.md)
- [上传、审核与发布](docs/developer/PUBLISHING.md)

`examples/` 提供四个可构建样例；`python3 scripts/verify-developer-examples.py` 复现离线打包、正反兼容性、页面 SDK 与原生 MCP 检查。源码核实与验证边界见[核实记录](docs/developer/CODE-VERIFICATION.md)。当前 CLI 不提供本地安装/热重载或上传发布命令，真实安装走同环境市场；不要使用旧 plugin.json 模板。

## 官方插件

- [EOS 会议纪要](official/meeting-notes/README.md)：`eos.meeting-notes`，将用户提供的会议记录整理为摘要、已确认决策、行动项和待确认事项。源文件位于 `official/`；目录存在不表示已上架，市场审核和签名发布状态才是上架依据。

已安装用户可参考[会议纪要调用指南](docs/USE-meeting-notes.md)，使用完整技能名称明确调用。市场包版本保持不可覆盖，指南的更新不修改已发布制品。

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

## 提交与上架

按[开发手册](docs/developer/README.md)编写 `plugin.toml`、README、LICENSE 和实际模块入口，使用 `eos-plugin pack` 与 `validate` 打包校验。获准上传的账号提交到统一市场，管理员审核后签名发布，详见[作者发布指南](docs/developer/PUBLISHING.md)；服务器维护者另见[运维流程](docs/OPERATIONS.md)。

`plugins.json` 是历史静态索引，不用于正式市场、安装或上架；`fixtures/` 仅用于隔离验证，不自动发布。
