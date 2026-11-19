# 本地验收记录：2026-11-03

> 历史记录，保留当日结果和“未执行”事项；不表示当前部署或发版状态。当前开发入口见[开发手册](developer/README.md)，2026-11-12 源码核实与本次验证见[核实记录](developer/CODE-VERIFICATION.md)。

范围：`feat/unified-plugin-market` 上的共享协议/运行库、内部市场服务、管理后台和 EOS Agent 接入。全部账号、数据库、Redis、密钥、包、Core 配置和浏览器页面使用隔离夹具。

| 验证 | 结果 |
| --- | --- |
| eos-plugins：cargo test --workspace | 14 项通过，包含恶意归档、TUF 篡改、依赖事务、崩溃恢复、配置与迁移回滚 |
| eos-plugins：cargo clippy --workspace --all-targets -- -D warnings | 通过 |
| generate / vendor / 跨仓 DTO 检查 | 通过，敏感账户传输字段不生成前端接口，不派生 Rust Debug |
| Core：cargo build --workspace | 通过 |
| Core protocol | 172 项通过 |
| Core MCP | 34 项通过 |
| Core WASM Component | 3 项通过，包含真实执行、无限循环限制、提交前关闭权限和 generation 身份 |
| Core 旧插件协议拒绝 | 通过 |
| base：cargo build --workspace | 通过 |
| 真实数据库服务测试 | 通过，覆盖审查发布、私有下载、成员控制、安全撤销和临期续签 |
| 真实 HTTP → 独立 Core stdio | 通过，覆盖安装、实际 Skills/WASM/MCP、审批、版本升级、数据保留、离线重启、旧 generation 拒绝、账户上下文交接、成员撤销/重新授权、插件撤销、卸载 |
| Go 账户、生成协议与桥接 | 通过；市场会话不落盘，离线选择没有 bearer，退出删除缓存身份 |
| Agent 前端生产构建及相关回归 | 构建通过，256 项回归通过 |
| 管理后台生产构建 | 通过 |
| pro 单向组合构建 | 通过，包含共享源码快照和 EOS 应用注册 |
| 真实 Chrome 独立页面 | 360/1080、浅色/深色四组通过；宿主 DOM、宿主存储隔离，私有消息端口可用 |
| macOS 原生 Wails/WebKit 独立夹具 | 通过：主页面 HTTP/原生调用成功；子 frame 原生事件、无凭证 POST、runtime 导航均未触达探针；自主导航被拒绝；结果 ok=true |
| TypeScript SDK 类型检查 / Go SDK 编译 | 通过 |

Core 的 Clippy 仍报告旧工具与事件队列中三处 Atomic::fetch_update 弃用警告；base 仍有原有未使用项等警告。没有将这些报告为全仓零警告。前端构建仍有既有大包提示。

只读盘点了当前机器默认 `.eos/plugins` 与 `.eos-dev/plugins`，两处目录不存在；这不是服务器和所有客户端的历史数据盘点结论。未改动或删除历史插件数据。

未执行：服务器部署、生产根密钥仪式与公开根注入、正式安装包更新/日常客户端验收、市场外部开放。当前支持矩阵和未来宿主接入边界见 [HOST-INTEGRATION](HOST-INTEGRATION.md)，部署要求见 [OPERATIONS](OPERATIONS.md)。

原生复现（仅 macOS，独立隐藏窗口，不启动日常客户端）：在 eos-app-src 执行 `go build -tags plugincheck -o /tmp/eos-plugin-native-check ./cmd/plugin-webview-check`，然后 `/tmp/eos-plugin-native-check /tmp/eos-plugin-native-result.json`。失败返回非零；结果只记录探针调用、阻断次数和能力，不记录随机凭证。
