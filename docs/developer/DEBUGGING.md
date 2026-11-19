# 调试与排障

按包检查 → 模块本地检查 → 测试市场真实安装 → 升级/生命周期验收的顺序定位。每一步记录输入、实际输出与宿主版本；上一层成功不能代替下一层。

## 当前可用的开发循环

| 阶段 | 操作 | 能证明什么 |
| --- | --- | --- |
| 离线打包 | CLI pack + validate | 结构、路径、清单、配置 Schema 可读 |
| 离线兼容 | CLI compatibility + HostDescriptor | 所选模块与声明能力匹配；不包含制品选择/签名 |
| 模块调试 | 页面模拟宿主、原生 MCP smoke | 模块在受控本地输入下响应 |
| 测试市场 | 上传、审核签名、同环境 Agent 安装 | 下载、校验、授权、真实运行和注册 |
| 发布验收 | 实际用户调用、升级、重启、停用/卸载 | 对用户可用的完整流程 |

没有 `eos-plugin install ./dir`、Git URL 安装、拖入包导入、watch 热重载或“跳过签名”开关。旧 `plugin/install` RPC 已退役；旧代码/模板仍可能存在，不能据此作为当前入口。`init-dev` 生成隔离测试签名资料，不会启动市场、批准发布者、安装包或配置桌面端。

不能联络测试市场维护者时，仍可完成离线包/模块开发，但不能将其标成“已通过 Agent 安装”。下一步是提供包、目标、清单和验收步骤给维护者，按[发布指南](PUBLISHING.md)完成测试环境闭环。

## CLI 全部命令

在仓库根目录可用 `cargo run --locked -p eos-plugin-cli -- <命令>`；编译后二进制命令为：

```text
eos-plugin version
eos-plugin pack <directory> <output.eos-plugin>
eos-plugin validate <artifact.eos-plugin>
eos-plugin compatibility <artifact.eos-plugin> <host.json> [optional-module-id ...]
eos-plugin inventory <explicit-legacy-root>
eos-plugin init-dev <new-isolated-directory>
eos-plugin sign <root.json> <targets-dir> <metadata-dir> <revision> <key-path> <key-path> ...
```

当前 `--help` 只返回简短 usage，不是完整帮助。version 返回工具版本（默认 1.0.0），不表示宿主或清单版本。inventory 仅做旧清单盘点；不执行、不迁移、不删除。init-dev/sign 是平台维护工具，普通作者无需签名私钥。

compatibility 的可选模块参数是模块 ID 的位置参数，例如：

```sh
cargo run --locked -p eos-plugin-cli -- compatibility ../plugin-artifacts/fixture.eos-plugin examples/hosts/skills.json panel
```

必需模块自动选中。可选模块未选时，不兼容可能只出现在 unavailableModules，不使整体 incompatible；选中后则会失败。退出码 0 仍需检查 JSON。authorization-required 也不表示已经得到用户同意。

## 获取真实宿主描述与诊断

普通作者可以把兼容性错误和应用“关于”中的版本发给维护者。内部受信壳层提供：

```text
PluginMarketCall("host", "{}")
PluginMarketCall("diagnose", "{}")
```

host 返回 HostDescriptor；diagnose 返回 `{host,installations}`。这是应用集成接口，**不是 shell 命令、插件页面 SDK 方法或浏览器控制台全局函数**。目前插件页面不能自行获取这些数据，GUI 也没有一键导出它们。维护者应通过自身测试壳层导出 host.json，删除安装中的用户标识、配置和本地资料后再共享；不要传出内部 MarketAccess 的账号 token。

HostDescriptor 字段示例见 `examples/hosts/skills.json`；实际 applicationId、environmentId、hostVersion、executors、capabilities、uiSlots 以当前应用为准。合成描述的 compatible 不能证明真实宿主可用。

## 模块本地检查

- **Skills**：检查 SKILL.md 第一行/结尾 frontmatter、非空 name、单行字段和工具策略；明确完整技能名。pack 不解析技能正文。真实宿主安装后确认列表中出现技能，再显式调用。
- **页面**：按[模块手册](MODULES.md)运行 notes-panel 的 dev-host.html；浏览器控制台看 JS 异常与 CSP 错误；保存后关闭/打开页面核对内容。模拟宿主只覆盖 SDK 消息和 UI，真实 Agent 再验证隔离与持久化。
- **MCP stdio**：运行 examples/mcp-echo/smoke.py；检查三步协议回复及退出行为。确认 stdout 没有日志、二进制架构正确、执行位保留、无环境继承依赖。真实 Agent 安装时 MCP probe 失败会使安装失败。
- **MCP HTTP**：从目标设备验证 HTTPS MCP 协议、DNS 与证书；不能只测 HTTP 200。重定向/私网/需要 OAuth 的服务不符合当前模板。
- **WASM**：pack 不编译组件。先使用 wasm-ack 对照样例，然后在测试 Agent 查工具目录、实际执行。Core WASM/WASI 模块与 Component Model 不通用。

## 日志与安装状态

Agent 的设置 → 关于中有日志目录与“打开日志文件夹”。实际目录可能按渠道/用户设置变化；Core 日志在所选目录的 `core/eos-core.log`。临时本地启动可设置 `EOS_LOG_LEVEL=debug`；不要把 token、用户原文或私密配置写入日志。MCP 子进程 stderr 当前不被转储到 Core 日志；SDK log 也仅确认接收。

Core 插件状态默认在 `$EOS_HOME/plugin-platform/<environment>`；未设置 EOS_HOME 时为 `~/.eos/plugin-platform/<environment>`。Dev 桌面有自己的渠道状态目录，且会设置 EOS_HOME。不要修改 HOME、把生产状态复制到 Dev，或手工改安装 JSON/缓存来“修复”。停用、卸载和重新准备安装走应用管理入口。当前没有通用作者日志聚合面板。

## 常见问题

| 现象/错误 | 先核对 | 处理 |
| --- | --- | --- |
| SCHEMA_INVALID | 拼写、未知字段、publisher、必填数组、entry | 按当前清单示例修正；TOML 的后续字段属于最近的表 |
| missing module entry | Skills 是否有子目录/SKILL.md；页面/二进制是否已生成 | 构建后只打包 package/ |
| ARCHIVE_UNSAFE | 符号链接、路径、同名碰撞、特殊文件 | 用普通文件暂存目录，重新 pack |
| CONFIGURATION_INVALID | 缺 required、类型/额外字段、Schema | 安装确认页填写实际 JSON；default 不自动填 |
| compatible 但安装不兼容 | ABI、真实 OS/架构、制品目标 | 单独兼容命令不选择市场制品；核对目标声明 |
| CAPABILITY/EXECUTOR_UNSUPPORTED | host.json 与实际构建 | 缩小功能或使用真正实现能力的宿主 |
| ISOLATION_UNAVAILABLE | sandboxed-process 或页面隔离未启用 | 当前 stdio 只支持 trusted-process；页面按真实声明 |
| ACTIVATION_FAILED | MCP 启动、组件 ABI、有效 Skills | 先模块独立检查，再看 Core 失败详情；勿把 pack 成功当执行成功 |
| PLUGIN_HOST_REQUIRED | 直接打开独立 HTML | 用演示 dev-host 或真实 Agent 页面 |
| PLUGIN_HANDSHAKE_TIMEOUT | SDK 是否先注册监听；页面是否内联 | 检查构建和 CSP；不要在加载阶段 await 不存在的网络资源 |
| PLUGIN_REQUEST_TIMEOUT | 超频、消息过大、端口过期 | 限流、防抖、缩小消息；关闭旧端口再重开页面 |
| 页面只有空白或缺样式 | 相对 script/src、外链 CSS、fetch、动态 import | 内联单文件；禁用远程依赖 |
| METHOD_NOT_FOUND | SDK 方法名或调用了旧 RPC | 使用 MODULES 列出的 request 方法 |
| GRANT_REVOKED | 权限是否由该模块引用；技能范围；generation | 重新确认合法计划、重新打开页面；不复用旧代次端口 |
| MARKET_UNCONFIGURED | 环境/测试公开根/市场启用 | 请环境维护者配置；不是包格式问题 |
| UPLOAD_FORBIDDEN | 活跃账号与上传邮箱白名单 | 请维护者授权，管理员身份也不自动允许上传 |
| NOT_FOUND_OR_FORBIDDEN | 发布者是否获批；包是否已发布；登录 | 告知维护者具体阶段；不靠改官方 publisher 绕过 |
| RESOURCE_CONFLICT | ID/version 已存在或 revision 过期 | 新版本重新打包；状态操作先刷新记录 |
| PLAN_CHANGED/PLAN_EXPIRED | 计划摘要、目录策略、时间 | 重新准备并确认，不重用旧 planDigest |
| SIGNATURE_INVALID/METADATA_EXPIRED | 环境、公钥根、签名目录时效 | 由维护者修复信任链/续签；不跳过验签 |
| POLICY_LEASE_EXPIRED | 离线时长、账号会话 | 联网刷新授权；最长租约 24 小时 |
| CLOCK_ROLLBACK | 系统时钟回拨 | 校准系统时间并联网 |
| MIGRATION_FAILED/MIGRATION_REQUIRED | 迁移链与旧数据版本 | 测试数据上重现，修复后发新版本；不直接覆盖数据 |

## 发布前真实验收

1. 记录源码提交、插件版本、包 SHA-256、清单摘要、制品目标、Agent 版本及实际 HostDescriptor。
2. 新账号/新安装确认权限和配置；可选模块分别测试不选和选择。
3. Skills 显式调用并验证内容边界；MCP/WASM 实际调用并核对结果，页面验证配置读取、保存与重新打开。
4. 输入错误、拒绝权限、远程不可用时应显示错误；不能偷偷扩大授权或切换执行模式。
5. 停用后入口不可调用；启用与重启恢复正确；切换账号不见旧账号授权/数据；卸载后不残留技能/工具/页面。
6. 先装旧版本并写入合成数据，再升级；扩大权限需重新确认，失败不破坏旧安装；数据迁移验证成功与失败恢复，降级拒绝。
7. 下架与安全撤销在**隔离测试市场**验证：下架停止新安装，已有租约仍可运行；撤销在策略同步或租约到期后阻止运行。应用有 4 分钟心跳，插件页另有 5 分钟刷新；离线不能保证即时撤销。

本仓自动验收脚本验证静态示例、CLI 判定、MCP 三步协议和页面构建；真实 Agent、模型结果、原生 WebView 与服务端链路需按上述步骤另记证据。不要将模拟宿主显示成功写成“生产验收通过”。
