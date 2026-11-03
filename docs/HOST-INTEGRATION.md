# 接入统一插件系统

平台合同版本 1.0.0。EOS Agent 是当前完成接入的产品；新产品实现自己的宿主适配器，不复制市场、包解析器、策略或安装事务。

## 依赖和生成

- `eos-plugin-protocol`：严格清单解析、生成的 DTO、Schema、摘要和标准错误。
- `eos-plugin-policy`：不依赖 IO 的兼容判定、制品选择、权限和资源范围校验。
- `eos-plugin-runtime`：安全包处理、TUF 校验、安装计划、事务和数据迁移；MarketClient 统一处理鉴权请求、签名策略、离线缓存和受控下载。
- `sdk/typescript/src/client.ts`：独立页面的消息端口客户端。
- `sdk/go/protocol.go`：生成的传输字段，Go 壳不承担兼容性或授权裁决。

`schema/contracts.json` 是插件 DTO 的唯一来源。运行 `python3 scripts/sync-consumers.py` 更新独立仓库的源码快照和字段；CI 使用 `--check` 检查漂移。快照的 `source-lock.json` 固定每一个文件的摘要，消费方可独立构建。敏感的账户传输 DTO 也由此生成，但不输出到前端 TypeScript 接口，也不派生 Rust Debug。

## 一个新宿主要做什么

1. 在产品组合层注册 applicationId；服务端内部成员授权明确包含该应用。应用 ID 来自经过认证的应用会话，不能只信请求体。
2. 构造 `HostDescriptor`：应用版本、平台 API、环境、稳定实例 ID、OS/架构/界面类型，以及实际实现的执行器、能力版本和 UI 插槽。
3. 实现 `HostAdapter` 的 prepare、activate、commit、deactivate、dispose；数据升级通过 migrate 执行。activate 只暂存贡献，持久化完成后 commit 才开放权限。资源注册按安装 ID 和 generation 管理，失败必须可撤销。
4. 从可信账户会话构造 MarketAccess，将宿主固定的公开根字节传给共享 MarketClient；阻塞网络客户端放在工作线程调用，异步服务器使用 spawn_blocking。产品适配器只负责自身应用/环境/来源/根的绑定。使用 `PluginStore` 的 prepare/install/enable/remove/recover；服务端解析结果仍要在本地重检。确认必须带同一 planDigest，下载结果须通过长度、摘要、清单及 TUF 校验。
5. 能力代理每次检查主体、安装作用域、模块、generation、租约、能力、操作和资源范围。宿主私有业务 API 只能由已注册的能力适配器提供。
6. 在公开新的账户身份前，先向运行时传送 host-only 身份信号并确认旧上下文关闭；离线资料同步失败不伪造账户变化。将签名策略续期绑定到应用生命周期；退出账户、切换环境、撤销和租约过期均卸载贡献。离线身份只用于选择本地安装，运行权仍来自有效签名租约。

共享缓存可以复用制品字节；安装记录、普通配置、数据和授权分别绑定环境、应用、实例、账户和 scope。Agent 当前支持用户作用域。其他作用域需要产品显式实现和服务端授权。

## Agent 当前执行器与能力

| 项目 | 行为 |
| --- | --- |
| skills/v1 | 解析 SKILL.md，按插件/模块隔离名称，接入现有技能目录和调用 |
| mcp-stdio/v1 | trusted-process，固定包内入口和参数，清空继承环境，复用原有工具审批 |
| mcp-http/v1 | 固定 HTTPS 端点，禁私网、重定向和隐式代理，DNS 校验并固定解析结果 |
| wasm-component/v1 | Component Model world `eos:plugin/component@1.0.0`，无 WASI，燃料/内存/时间/取消限制 |
| webview/v1 | macOS 原生隔离验证通过后声明；独立 srcdoc iframe，sandbox 仅 allow-scripts，CSP 禁外联与表单，原生桥只接收主页面 |
| eos.ai.skills 1.0.0 | invoke，仅当前插件命名空间；非空资源选择器使用精确 skills 列表 |
| eos.ui.panel 1.0.0 | panel/page/settings 插槽，实际页面通过隔离面板呈现 |

Agent 不声明 sandboxed-process、远程服务端执行、OAuth/秘密代理或跨插件导出调用能力。要求这些能力的插件必须显示不兼容，不能自动降级为本机受信进程，也不能将秘密放入普通配置。新宿主可以按同一版本协商协议实现并声明对应能力。

macOS 的 Wails beta.5 原生适配在创建窗口前封闭子 frame 消息与子页面自主导航，并在主 frame 的 document-start 脚本闭包中保存随机凭证；HTTP runtime 仅接受携带该凭证的 POST。WKURLSchemeTask 会丢弃部分来源请求头，因此不能单凭 Origin 判定调用者。只有原生保护安装成功且宿主 POST 实际通过后，Go 才传送 `uiIsolation=opaque-frame`；Core 据此声明页面能力。其他平台或 ABI 不匹配时不声明 UI 执行器，必需页面模块不兼容，可选页面模块不可选择。扩展平台需要各自的原生逃逸验证。

## 普通配置和数据

configuration.schema 必须是包内、自包含的 JSON Schema。解析计划固定它的内容，安装时重新比对并校验配置。普通配置随安装主体保存；独立页面通过 configuration.get 读取，组件通过内建 eos.plugin.configuration/get 读取。它不是秘密存储。

私有页面数据为至多 1 MiB 的 JSON 对象。data.migrations 声明 fromVersion、toVersion 和 WASM 模块；组件的 invoke("migrate", input) 接收 `{fromVersion,toVersion,data}`，返回升级后的数据对象。迁移时清空授权，不提供外部业务副作用。事务日志保留旧数据，激活失败和进程中断均恢复；数据降级明确拒绝。

## 页面 SDK

独立页面导入 `@eos/plugin-sdk` 的 connectPlugin()。握手校验父窗口、随机 nonce 和唯一 MessagePort；可调用 storage.get/set/delete、configuration.get、capability.invoke 和 log。SDK 包含并发、消息大小和超时限制。

宿主创建 sandbox iframe，并在 onload 时将 MessagePort 转移给该 iframe。每次转发必须重新绑定当前 installationId、moduleId、generation；关闭页面销毁端口。账号 token、宿主 HTTP 客户端、Wails/Tauri 桥和宿主 UI 框架均不能传入。

## 最小验证

`fixtures/portable-skills`、`fixtures/targeted-tasks` 提供合同夹具；`fixtures/agent-integration` 提供真实 Skills、WASM、MCP 和页面测试包。它们不会自动进入市场。

新宿主至少验证：相同通用包的兼容结果、专属应用拒绝、可选模块协商、固定计划确认、跨账户隔离、实际执行、重启恢复、撤销/离线过期、迁移失败回滚、旧 generation 拒绝，以及卸载后无贡献残留。仅通过类型检查不能标为已验证宿主。
