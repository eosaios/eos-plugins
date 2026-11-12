# 清单、权限与兼容性参考

清单使用 UTF-8 TOML，结构版本为 `schemaVersion = 1`。大小上限 1 MiB；未知字段、重复键、类型错误会拒绝。不要使用旧后台的 `plugin.json` 或历史 Agent `[tools]` 清单。完整起步样例见[快速开始](QUICKSTART.md)。

## 顶层字段

| 字段 | 必填 | 当前含义 |
| --- | --- | --- |
| `schemaVersion` | 是 | 固定为整数 1 |
| `[plugin]` | 是 | 身份、名称、版本、说明、许可证及资源 |
| `[compatibility]` | 是 | 平台 API 版本范围、portable/targeted、应用限制 |
| `[[modules]]` | 是 | 1–64 个模块 |
| `[[permissions]]` | 否 | 模块按 ID 引用的宿主能力权限 |
| `[configuration]` | 否 | 包内自包含 JSON Schema；不存秘密 |
| `[data]` | 否 | 插件私有数据版本与纯 WASM 迁移 |
| `[[dependencies]]` | 否 | 同市场插件版本依赖 |
| `[[exports]]` | 否 | 合同扩展点；当前 Agent 没有跨插件导出调用适配器 |

`plugin.id` 格式为 `publisher.slug`，只允许小写 ASCII 字母、数字与段内连字符，总长最多 128；例如 `acme.text-tools`。publisher 必须与 ID 第一段相同。名称最多 128 字符，description 最多 8192。版本是完整 SemVer，不允许 `+build` 区分发布；`0.1.0-dev.1` 是预发布版本，不能假定普通版本范围自动接受它。正式版本最好用于第一次入门测试，避免混淆预发布筛选与打包错误。

必填元信息为 `id/publisher/name/version/description/license`。可选 `homepage/repository` 仅作说明，`icon` 与 `i18n` 指向包内文件。声明文件必须存在。

`portable` 禁止顶层 applications；`targeted` 要求非空列表，例如：

```toml
[compatibility]
platformApi = "^1.0.0"
scope = "targeted"
applications = [{ id = "eos-agent", version = ">=1.0.0" }]
```

插件 scope 与安装 scope 不同：前者表示适用产品范围，后者表示用户/工作区/组织安装隔离。当前 Agent 只提供用户安装 scope；包、授权和配置不会跨应用、账号或设备自动共享。

## modules 字段

| 字段 | 必填/默认 | 规则 |
| --- | --- | --- |
| `id` | 必填 | 插件内唯一，小写字母数字与连字符 |
| `type` | 必填 | `skills/v1`、`mcp-stdio/v1`、`mcp-http/v1`、`wasm-component/v1`、`webview/v1` |
| `required` | 必填 bool | true 的模块必须兼容；false 的模块由用户选择 |
| `activation` | 必填 | 结构接受 on-demand/on-command/on-event/on-startup；当前并未实现一套可订阅的事件调度 API，见下文 |
| `requires` | 必填数组 | `{capability, version}`；不用能力时写 `[]` |
| `permissions` | 必填数组 | 引用顶层 permission.id，不是 capability ID；不用权限时写 `[]` |
| `entry` | 除 HTTP MCP 外必填 | 包内相对文件路径；Skills 是目录 |
| `endpoint` | HTTP MCP 必填 | 固定 HTTPS URL；HTTP MCP 禁止 entry |
| `profile` | stdio MCP 必填 | 当前只运行 trusted-process；sandboxed-process 会不兼容 |
| `world` | WASM 必填 | 固定 `eos:plugin/component@1.0.0` |
| `args/env` | 默认 `[]/{}` | stdio MCP 的固定参数、非秘密环境变量；当前不做配置模板插值 |
| `applications` | 默认 `[]` | 模块自己的可选应用范围；不赋予应用能力 |
| `contributions` | 默认 `[]` | 页面使用 `{kind,id,title}`；kind 为 panel/page/settings，id 不含点/斜线 |
| `limits` | 可选 | memoryBytes/fuel/timeoutMs/concurrency；并非每个执行器都实施每个字段 |

optional 模块即使在包中也不会自动选中；用户勾选后才激活。当前 Agent 安装提交时注册所选 Skills、探测 MCP、加载 WASM 工具，页面在点击打开时显示。`activation` 没有提供定时器或任意事件订阅；`on-event/on-startup` 会被策略标记为需要授权，不能据此宣称存在后台任务调度能力。

WASM 实施的限制：内存默认/上限 64 MiB，fuel 默认 1,000,000、上限 10,000,000；单次调用默认/上限 30 秒，加载 describe 和迁移 5 秒。输入/输出各最多 1 MiB。每次调用新建实例，不能把全局内存当持久存储。`concurrency` 当前不是可依赖的每模块并发配额实现。其他模块不要把 limits 当成操作系统沙箱承诺。

## 能力要求与权限的区别

`requires` 决定“这个宿主能否提供功能”；`permissions` 决定“模块请求调用哪些功能”。声明 requires 不会生成授权，权限确认也不会实现不存在的能力。

当前可声明的 Agent 能力为：

| 能力 | 版本 | 用法 |
| --- | --- | --- |
| `eos.ai.skills` | 1.0.0 | Skills 注册；页面/WASM 的 invoke 仅可指向当前插件命名空间 |
| `eos.ai.skills.tool-policy` | 1.0.0 | 显式调用技能时执行 allowed-tools 限制 |
| `eos.ui.panel` | 1.0.0 | 仅实际启用原生页面隔离的宿主声明 |

例如页面请求激活自己的技能：

```toml
[[permissions]]
id = "invoke-summary"
capability = "eos.ai.skills"
actions = ["invoke"]
resources = { skills = ["tutorial.hello-skills/writing/summarize"] }
reason = "在当前插件内启用摘要技能"
```

相应页面模块需 `permissions = ["invoke-summary"]` 且声明 requires。资源名是完整技能名；资源对象 `{}` 表示允许本插件命名空间内技能，非空对象仅支持一个 `skills` 数组，按完整名称精确匹配。不支持通配符、其他键或跨插件调用。

SDK 调用参数与返回值见[模块手册](MODULES.md)。这里的 invoke 是选择/激活技能，不等同于一次模型完成请求，不返回 AI 生成文本。

## 普通配置

```toml
[configuration]
schema = "schemas/config.json"
```

```json
{
  "type": "object",
  "properties": { "title": { "type": "string", "maxLength": 80 } },
  "required": ["title"],
  "additionalProperties": false
}
```

Schema 必须是包内 JSON 对象，至多 1 MiB，`$ref` 只能引用 `#` 开头的本地定义。安装对话框填写 JSON；Schema 的 default 不会自动填充值。页面用 `configuration.get`，WASM 用 `host.call("eos.plugin.configuration", "get", "{}")` 读取。当前不会自动把普通配置灌进 Skills 文本、MCP args/env 或 endpoint。

声明 credentials 会要求 `eos.credentials.<kind>` 能力；当前 Agent 不提供。不要把 API key 放入普通配置、env、包、页面或日志。

## 私有数据与迁移

页面的 storage 是**安装级**私有 JSON 键值对象，所选页面模块共享；不是每个 module 单独隔离。键最多 256 字节，总序列化数据至多 1 MiB。卸载默认保留数据，停用不清数据。不要读写插件管理器的磁盘文件来调试数据。

```toml
[data]
version = 1
migrations = [{ fromVersion = 0, toVersion = 1, module = "migrate-data" }]
```

当前 Schema 只接受 version 和 migrations，没有 recovery 字段；运行时依靠事务快照恢复。迁移模块必须是清单中的 WASM 模块，每段前进且不超过目标数据版本，迁移调用为 `invoke("migrate", {fromVersion,toVersion,data} 的 JSON 字符串)`，返回升级后的**数据对象** JSON；未保存过数据时 input.data 可能为 null。每条迁移记录需使用不同 module ID（解析器拒绝重复的迁移 module）。迁移时授权清空，不允许外部业务副作用；缺迁移路径、非法返回、崩溃或降级都会失败。先在测试安装中保存 v0 数据，再升级核对保留字段，不能只验证全新安装。

## 包和制品目标

`.eos-plugin` 是 tar.gz，根目录需 plugin.toml、README.md、LICENSE 及全部声明入口。不能包含符号/硬链接、设备文件、绝对路径、`..`、反斜线、盘符、保留设备文件名、大小写/Unicode 归一化碰撞或特殊权限位。默认上限：压缩 200 MiB、解压 500 MiB、单文件 200 MiB、10,000 个归档条目、16 层路径。

上传目标不是清单字段，由上传请求另行声明：

```json
{"os":"any","arch":"any","abis":["skills/v1@1.0.0"]}
```

原生包按机器声明，如 `linux/aarch64`、`windows/x86_64`。**当前 Core 在 macOS 报告 `os: "macos"`**，因此上传使用 macos；架构使用 aarch64/x86_64，不是 arm64/amd64。精确目标优先于 any/any；同优先级多匹配拒绝。abis 是执行器 `type@abi`，不是 WIT world；列出的每项都必须存在于宿主，因而不要把可选页面的 ABI 塞进本应支持无页面宿主的通用制品。

同一发布的所有目标需要完全相同的 plugin.toml 原始字节、相同市场多语言投影；修改换行也会改变清单摘要。一个目标元组只有一个制品。当前依赖由服务端解析并固定到安装计划；`dependency-lock.json` 在目标规范中要求，但当前包读取器未强制它、也不据它解析依赖。不要把它当成已生效的锁定机制；作者先使用无依赖模板，依赖版本以确认计划为准。

## 市场多语言

```toml
[plugin.i18n]
zh = "i18n/zh.json"
en = "i18n/en.json"
```

```json
{"$eos.marketplace":{"schemaVersion":1,"name":"三点摘要","description":"将输入整理为三条要点"}}
```

语言键使用规范小写，例如 zh、en、pt-br。最多 32 种投影；每个展示资源至多 16 KiB；name 至多 256 UTF-8 字节、description 至多 8192 字节，不允许空白、控制字符、重复/未知保留字段。普通 UI JSON 键可共存，不会自动翻译。当前 CLI validate 检查资源存在，不检查 marketplace 投影内容；服务端上传时才严格检查。更新市场名称/简介需要新版本上传审核签名，不能修改历史包。
