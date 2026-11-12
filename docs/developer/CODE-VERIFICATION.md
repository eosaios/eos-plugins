# 开发文档源码核实与验证记录

日期：2026-11-12。范围为本地源码、文档和可重复执行的教学示例。未连接生产数据库、未上传或发布插件、未修改市场配置，也未重新验证正式客户端/原生 WebView。历史部署结果不能由本记录推断。

## 核实基线

| 仓库 | 核实起点 SHA | 用途 |
| --- | --- | --- |
| eos-plugins | `60392a3` | 清单 Schema、严格解析、包、策略、CLI、页面 SDK、多语言 |
| eos-core-rs | `e7296118c7837ddba8b88bbc58d76d1de172f041` | Agent HostDescriptor、管理 RPC、模块挂载、能力代理、WASM/MCP |
| eos-app-src | `95f58301ed09149eb0f80dc4e7ae37868763a596` | 安装 UI、iframe/CSP、账号桥、日志、渠道状态与心跳 |
| eos-base | `f692f9ecbb286c16b46479d0097b93dbe98cdabd` | 实际 API、上传资格、审核/签名状态、目标、多语言 |
| eos-base-pro | `46455bdaa7d0efbf932c8e5112438b140ca3a018` | 独立市场上传/展示、EOS 注册与组合 |

SHA 表示读取的本地检出基线，不表示远端最新提交或服务器运行 SHA。文档和新样例位于 eos-plugins；没有修改其他仓库。

## 关键事实与拥有者

相对路径以各自仓库根目录为准，方便维护者复核；作者完成教程不需要打开这些实现。

| 核实点 | 代码拥有者 | 文档结论 |
| --- | --- | --- |
| CLI 命令/退出行为 | eos-plugins `crates/eos-plugin-cli/src/main.rs` | version/pack/validate/compatibility/inventory/init-dev/sign；无安装/发布/热重载；兼容性失败需读 JSON |
| 清单字段 | eos-plugins `crates/eos-plugin-protocol/schema/contracts.json`、`src/lib.rs` | 严格 TOML/Schema；data 只有 version/migrations；HTTP MCP endpoint 而非 entry |
| 包规则 | eos-plugins `crates/eos-plugin-runtime/src/package.rs` | pack 全目录不读 gitignore；README/LICENSE/入口检查；执行位；大小与路径限制 |
| 兼容性/目标选择 | eos-plugins `crates/eos-plugin-policy/src/lib.rs` | 必需/可选、能力版本、profile、ABI；兼容命令不选择市场制品 |
| 依赖/配置/迁移 | eos-plugins `crates/eos-plugin-runtime/src/store.rs`、`store_transaction.rs` | 精确计划、普通配置重检、事务、数据迁移与降级拒绝；无 data.recovery |
| 页面 SDK | eos-plugins `sdk/typescript/package.json`、`src/client.ts` | private 源码 SDK；端口握手、请求/响应、超时与并发 |
| 页面加载/CSP | eos-app-src `frontend/src/plugin-market.tsx` | 单 HTML srcdoc；无相对资源服务；外联阻断；安装配置与模块选择 |
| 页面能力实现 | eos-core-rs `crates/eos-core-app-server/src/plugin_market/ui.rs`、`host.rs` | 安装级 storage、configuration.get；log 仅确认；技能 invoke 非模型完成接口 |
| 真实宿主描述 | eos-core-rs `crates/eos-core-app-server/src/plugin_market/host.rs` | 用户 scope、当前三个能力；UI 依赖 opaque-frame；OS/架构取 Rust 常量 |
| 模块加载 | eos-core-rs `crates/eos-core-runtime/src/engine_impl/plugin_platform.rs` | 安装时 Skills 注册、MCP probe、WASM mount；无通用事件调度 |
| 技能解析/工具限制 | eos-core-rs `crates/eos-core-runtime/src/services/skill_loader.rs`、`skill_tool_policy.rs` | 简单单行 frontmatter；命名空间、allowed-tools 三态 |
| WASM ABI/限制 | eos-plugins `crates/eos-plugin-protocol/wit/plugin.wit`；eos-core-rs `crates/eos-core-runtime/src/services/plugin_component.rs` | 无 WASI；describe 工具合同、每次新实例、fuel/内存/超时、配置与技能代理 |
| MCP 环境/网络 | eos-core-rs `crates/eos-core-mcp/src/lib.rs`、`plugin_http.rs` | env_clear、入口父目录 cwd、stderr 丢弃；HTTPS/SSRF 限制 |
| 当前市场路由 | eos-base `crates/eos-api/src/plugin_market.rs` | uploads/trust-bundle/按 id+version 状态路由；无目标规范中的其他管理接口 |
| 发布与发布者 | eos-base `crates/eos-service/src/plugin_market/mod.rs`、`catalog.rs`、`config.rs` | 白名单与 active 账号、approved publisher；validated→approved→published；revision；当前无逐发布者用户成员校验 |
| 多语言校验 | eos-plugins `crates/eos-plugin-runtime/src/localization.rs`；eos-base upload 路径 | 上传时投影检查；CLI validate 只检查资源存在 |
| 网页与后台上传差别 | eos-base-pro `web/portal/src/PluginMarketUpload.jsx`；eos-base `web/admin/src/pages/system/PluginMarket.tsx` | 网页可填 ABI，后台上传默认 abis=[] |
| 日志/渠道/心跳 | eos-app-src `internal/adapter/stdio_client.go`、`internal/app/channel_data.go`、`frontend/src/plugin-market-heartbeat.ts` | Core 日志子路径、Dev 状态隔离、应用 4 分钟/页面 5 分钟策略刷新 |
| 旧安装退役 | eos-core-rs `crates/eos-core-app-server/src/lib.rs` | 旧 plugin/install/list/remove/search 返回 PLUGIN_PROTOCOL_RETIRED，旧文件存在不代表入口仍可用 |

## 本次可复现验证

从 eos-plugins 根目录运行：

```sh
python3 scripts/verify-developer-examples.py
cargo test --workspace --locked
python3 scripts/generate.py --check
```

已执行结果：

- 从 QUICKSTART 原文提取的完整 TOML/SKILL.md 代码块已独立打包、校验并判定 compatible；所有 JSON 示例可解析。
- 四个样例均通过 pack、validate；使用合成 HostDescriptor 验证支持路径，页面/MCP/WASM 在仅 Skills 描述下验证 incompatible；MCP 为 authorization-required，非自动授权。
- 页面样例 `npm ci`、`npm run build` 成功；Node 合成 DOM 中执行实际打包 SDK，验证拒绝错误 nonce、读取配置、恢复/写入草稿、显示宿主拒绝错误。不是浏览器或原生安全验证。
- 原生 MCP 使用锁文件编译、暂存，空环境和实际入口 cwd 下验证 initialize、tools/list、正常 tools/call 与非法输入；结果 `echo:hello`。
- 平台工作区 20 项测试通过，含解析、归档、兼容、事务/恢复、配置/迁移、公开发现、TUF 与多语言。
- 生成协议检查通过；21 份 Markdown 的相对链接和差异空白检查通过；原生 MCP 样例通过 Clippy `-D warnings`。工具环境：Rust 1.99.1、Node 22.23.4、Python 3.12.13；依赖版本由各锁文件固定，不宣称已测所有 OS/最低版本。

完整消费者同步检查 `python3 scripts/sync-consumers.py --check` **未通过**：base 快照通过，Core 快照的 runtime/src/lib.rs 缺少新 localization 模块的 re-export，与根工作区不同。该漂移存在于本次未改动的源/消费快照中；未覆盖当前 Core 分支或自动重生成其他仓库。当前 Core 不解释市场展示 JSON，文档分别核对了它的实际代码。跨仓集成人员应在相应开发分支安排快照同步/回归；本文不宣称跨仓完全同源检查全绿。

## 仍需真实环境验收的部分

- 四个新教学包的实际 Agent 安装、模型输出、WASM describe/invoke 与原生页面隔离。
- 新包的服务器 HTTP 上传、审核签名、下载与实际客户端组合；本次未运行需要数据库/Core 的跨仓服务 E2E。
- 生产配置、运行版本、市场已上架插件、签名根仪式、密钥轮换及发布渠道。
- Windows/Linux 原生二进制构建与页面支持；不能把本机样例编译结果标成跨平台验收。

这些步骤的可操作流程在[调试文档](DEBUGGING.md)和[发布指南](PUBLISHING.md)，历史 2026-11-03 记录只保留为历史证据。当前平台没有任意本地包安装/热重载，所以本手册明确把离线模块开发与测试市场签名安装分开；不虚构一个开发开关。
