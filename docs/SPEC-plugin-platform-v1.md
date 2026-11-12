# SPEC：统一插件平台 v1

日期：2026-11-03。访问策略已按同日用户修正为公开读取、指定账号上传，见 ADR-002。状态：已采纳的目标合同。首个实现及实际支持矩阵见 HOST-INTEGRATION.md；实施证据见 IMPLEMENTATION-plugin-platform-v1.md。MUST/必须仍作为后续宿主和发布的验收约束，不能把未声明能力视为已经实现。

上层结构见 [ARCHITECTURE](../ARCHITECTURE.md)，实施顺序与用例见 [实施与验收](IMPLEMENTATION-plugin-platform-v1.md)。示例 ID、版本、路径、资源和端点均为规范示意，不代表已注册应用或已发布插件。

## 当前实现与本合同的关系（2026-11-12）

本合同包含扩展目标；开发者实际操作以[开发手册](developer/README.md)为准，源码依据见[核实记录](developer/CODE-VERIFICATION.md)。已实现严格 TOML/Schema、五类模块的 Agent 路径（页面按原生隔离声明）、市场校验/审核/签名、安装事务与租约。以下不能当作当前支持承诺：

- 当前没有本地目录/Git 开发安装、热重载、作者发布 CLI 或发布者自助注册。
- 没有通用事件调度、秘密/OAuth 代理、受限本机进程、服务器执行或跨插件导出调用。
- 当前包读取器不强制 dependency-lock.json，依赖固定在服务端解析后的安装计划；SBOM/通用扫描/发布者逐用户成员校验仍需扩展或人工审核，非已实现的自动保障。
- 当前没有频道、audience、verification 或通用能力定义的管理接口。真实路由见第 10 节。
- SDK 是仓库内 private TypeScript 源码；独立页面必须内联单 HTML，不能假定相对资源加载或 npm 发布已存在。
- Schema 没有 data.recovery 字段；activation/limits 字段的实际执行范围见[清单参考](developer/MANIFEST.md)。

## 当前市场访问策略

已发布目录与搜索允许匿名读取，普通登录账号可以安装和使用。上传独立绑定服务端配置的邮箱白名单，默认关闭，不提升账号为后台管理员。审核、签名发布、下架与撤销仍属于管理权限。匿名读取只返回公开发布资料，不授予运行权限；账户/应用/环境绑定下载和插件权限确认保持不变。旧内部成员名单不再参与读取裁决。

## 1. 范围与版本

完整平台合同覆盖：插件/发布者身份、包、目录、访问控制、发布信任、兼容性、依赖、宿主接入、执行器、权限、配置、数据、生命周期、升级、撤销和审计。实现按纵向闭环推进，不将未支持类型作为已支持类型返回。

分开管理：`schemaVersion`（清单结构）、`platformApi`（管理/消息合同）、插件 `version`、能力版本、执行器 ABI、宿主版本、插件数据版本。以上版本不互相代用。

- `schemaVersion = 1` 表示本规范；未知主版本拒绝。增加字段须发布可明确识别的 Schema 修订；未识别字段拒绝，不静默忽略权限或入口。
- `platformApi` 与能力要求采用 SemVer 范围；预发布版只有显式允许才能满足。
- 插件正式版本采用 SemVer；禁止用构建元数据区分两个可安装发布。一个 ID、一个完整版本只有一个 Release。
- 能力 ID 与 API 不在宿主小版本中隐式改变。破坏性改变必须提升能力主版本，版本范围不匹配时明确拒绝。

## 2. 身份、作用域与权限主体

`pluginId = publisher.slug`，全部小写 ASCII；两段分别匹配 `[a-z0-9]+(?:-[a-z0-9]+)*`，总长 3–128。`publisher` 必须等于 ID 第一段且与发布者注册归属一致。官方命名空间由平台保留；创建者不能通过 manifest.author 获得发布权。

`applicationId` 为组合层注册的稳定字符串；显示名称、本地化和当前安装版本不充当应用身份。通用 base 不内置 EOS 产品枚举。

安装主键由以下维度组成：

```text
environment + applicationId + hostInstanceId + subjectId + scope + pluginId
scope = user | workspace(workspaceId) | organization(organizationId)
```

- 资源权限分别由当前主体、工作区/组织成员关系与插件授权验证。组织策略部署不会隐式授予成员数据访问权。
- 物理制品可以按摘要共享只读缓存；安装、启用、数据、授权和凭据仍按安装主键隔离。缓存命中仍检查当前主体的访问资格和租约；缓存不是绕过下载授权的第二入口。受保护缓存只由管理器访问，不给插件任意枚举或文件路径。
- 秘密按实例/主体隔离。切换账户立即关闭旧实例与消息端口、撤销句柄，不把另一个用户的配置或授权附着给新账户。
- 服务端请求的应用身份由受信访问令牌/服务主体确认；用户传入的 HostDescriptor 用于兼容判断，不能授予内部目录、付费权益或管理权限。

## 3. 清单模型

保留一个根 `plugin.toml`。正式解析使用标准 TOML 库；拒绝重复键、错误类型、缺失必填值、未知字段和越界路径。TOML 规范对象通过同版本 JSON Schema 校验；当前标准 TOML + Schema + 严格 DTO 解析已执行这些检查；不复用旧手写清单解析器。

顶层字段：

- `schemaVersion`：必填。
- `[plugin]`：`id/publisher/name/version/description/license` 必填；`homepage/repository/icon/i18n` 可选；`repository` 只作来源说明。
- `[compatibility]`：`platformApi`、`scope = portable | targeted` 必填。targeted 必须有非空 `applications = [{id, version?}]`；portable 禁止根应用白名单。系统平台限制由执行模块/制品目标声明。
- `[[modules]]`：非空。每个模块有唯一局部 `id`、`type`、`required`、能力要求 `requires`、所需权限 ID 列表 `permissions` 和 `activation`；执行类型决定其余字段。
- `[[permissions]]`：`id/capability/actions/resources/reason`；资源是所请求的上限，不等于授权结果。权限只能引用已注册能力的资源 Schema。
- `[configuration]`：JSON Schema 文件相对路径及凭据需求定义；凭据字段独立于普通配置。
- `[data]`：`version` 与 `migrations`；无持久数据可以省略。当前没有单独 recovery 字段，恢复由事务快照实施。
- `[[dependencies]]`：`pluginId/version/required`，必要时包含所依赖的插件导出接口及版本。跨插件依赖不自动继承权限或秘密。
- `[[exports]]`：对其他插件开放时声明 `id/module/interface/version/methods`，每个方法绑定输入/输出 Schema 文件及提供者侧所需权限。未声明的内部模块、方法、数据与依赖不对外暴露；导出 API 版本与插件版本分开管理。

模块只在兼容、被选择且获授权后启用。必需模块不兼容则整个安装不兼容；可选模块不能启用时明确显示原因。optional 不等于允许运行中遇错就静默替换功能。

可选模块可声明自身 `applications` 限制，例如同一 portable 插件包含一个 Todo 专用可选面板。portable 的承诺只针对完整的必需功能；市场按当前宿主说明哪些可选模块可用。

能力要求形式为 `{ capability, version }`。选定模块的全部依赖形成安装计划。若同一能力的版本交集为空、入口有歧义或依赖缺失则拒绝，不能按列表顺序碰运气。

### 3.1 通用插件示例

```toml
schemaVersion = 1

[plugin]
id = "eos.document-tools"
publisher = "eos"
name = "文档工具"
version = "1.0.0"
description = "文档处理技能及可选操作面板"
license = "SEE LICENSE IN LICENSE"

[compatibility]
platformApi = "^1.0.0"
scope = "portable"

[[modules]]
id = "writing-skills"
type = "skills/v1"
required = true
entry = "skills"
activation = "on-demand"
requires = [{ capability = "eos.ai.skills", version = "^1.0.0" }]
permissions = []

[[modules]]
id = "document-panel"
type = "webview/v1"
required = false
entry = "ui/index.html"
activation = "on-command"
requires = [{ capability = "eos.ui.panel", version = "^1.0.0" }]
permissions = []
contributions = [{ kind = "panel", id = "documents", title = "文档工具" }]
```

该示例没有文件授权。技能不能据此自动读取工作区文件；需要读取时必须有模块声明、资源选择及宿主的相应操作授权。面板自身只展示自身数据，新增宿主数据访问必须增加对应能力与权限。

### 3.2 应用专属能力示例

```toml
schemaVersion = 1

[plugin]
id = "eos.todo-importer"
publisher = "eos"
name = "任务导入器"
version = "1.0.0"
description = "将用户提供的任务数据写入所选择的任务清单"
license = "SEE LICENSE IN LICENSE"

[compatibility]
platformApi = "^1.0.0"
scope = "targeted"
applications = [{ id = "eos-todo" }]

[[modules]]
id = "importer"
type = "wasm-component/v1"
required = true
entry = "components/importer.wasm"
world = "eos:plugin/component@1.0.0"
activation = "on-command"
requires = [{ capability = "eos.todo.tasks.write", version = "^1.0.0" }]
permissions = ["write-selected-task-list"]

[[permissions]]
id = "write-selected-task-list"
capability = "eos.todo.tasks.write"
actions = ["create"]
resources = { taskList = "user-selected" }
reason = "将确认过的导入内容写入你选择的任务清单"
```

此示例仍需提供由宿主 UI 或其他获授权模块输入数据的贡献；不能通过原生文件路径参数跳过文件选择。Todo 尚未实现这些能力，示例不能被标成已经可运行。

## 4. 包、制品、依赖与信任

```text
<artifact>.eos-plugin        # tar.gz；扩展名不承担真实性保证
  plugin.toml               # 唯一根清单
  LICENSE
  README.md
  skills/...                # 根据模块声明提供
  ui/...
  components/...
  native/<os>-<arch>/...
  schemas/...
  i18n/...
  dependency-lock.json      # 存在插件依赖时必需
```

同一 Release 的所有 Artifact 必须具有相同清单原始字节摘要。Artifact 目标元组由 OS、架构与执行器 ABI 组成，全部在签名元数据中固定。规则：精确目标优先，其次显式 portable；同优先级多个匹配拒绝，不能按文件名推断。架构不同的入口选取由已签名目标映射决定。

发布包含清单摘要、每个包摘要与长度、规范化文件清单摘要、运行时依赖、构建来源、许可证/SBOM、扫描结果引用。所有可执行依赖随包提供或引用固定摘要的受管理运行资源；禁止 `npx -y package@latest`、安装脚本联网取浮动依赖及安装生命周期任意 shell hook。

解包必须在执行前验证：压缩/解压长度、单文件长度、文件数、路径深度、绝对路径、`..`、反斜线、盘符/UNC、符号/硬链接、设备文件、重复规范化路径、大小写及 Unicode 规范化碰撞、危险权限位、缺失入口、未列入清单文件。先限量读取，不能读完巨型文件后才检查大小。

初始全局硬上限作为实施默认：压缩 200 MiB、解压 500 MiB、单文件 200 MiB、10,000 个文件、16 层目录；应用策略可以收紧，放宽需新规范修订并验证资源占用。已有后台 50 MiB/500 文件限制应显式迁移，不让前后端限额互相矛盾。

依赖求解输出无环图与精确 lock：`pluginId/version/artifactDigest/exportApi`。只从同环境、当前主体可访问的市场解析；传递依赖也必须获授权。可选依赖不满足时只有其显式关联可选模块不可用，必需依赖失败则拒绝安装。v1 每个安装作用域同一 pluginId 只激活一个版本，冲突报告依赖链。

依赖安装与本插件激活同属一个计划；依赖发布摘要改变使计划失效。卸载有活跃依赖者的插件须拒绝或在用户明确选择后级联停用；不删除其他安装作用域还在使用的制品。

使用成熟 TUF 实现验证 root/targets/snapshot/timestamp 角色、签名阈值、版本与有效期，固定实现版本并跑官方/项目夹具。初始信任根随宿主构建发布；根轮换由旧/新根满足各自阈值连续验证。记录最高已见元数据版本，拒绝回放、混包和过期元数据。包内“自带公钥”不能自证可信。

初始设计采用离线 root 2/3 阈值，targets 与在线 timestamp 分离；私钥不进入应用包、仓库、普通 API 进程或对象存储。签名服务只签已批准的固定发布集合，并记录审计；各环境独立根。密钥生成、保管和恢复由实施时的密钥仪式单独验证，本文不创建任何密钥。

按 ADR-002，已发布目录/展示详情公开，签名信任集合、制品下载及运行授权仍需账号与应用会话。包摘要校验不能替代签名；HTTPS 不能替代发布信任；发布者签名只能作为附加证明，平台认可的目标元数据仍为安装依据。

## 5. 宿主描述、能力与兼容性

`HostDescriptor`：

```text
applicationId, hostVersion, platformApiVersion,
environmentId, hostInstanceId, platform(os, arch, surface),
executors[{type, abi, profiles}],
capabilities[{id, version, inputSchemaDigest, outputSchemaDigest, limits}],
uiSlots[{id, kind, version}], policyRevision
```

不包含登录令牌、业务数据或用户文件路径。描述本机可执行能力，不作为服务端角色凭证。

最终兼容判断在本地重复执行，不能只相信服务器过滤结果：

1. 当前主体能访问插件及所选发布；环境与信任根一致。
2. Schema 和平台 API 匹配。
3. portable/targeted 及模块应用限制匹配。
4. 必需模块的能力版本、执行器 ABI、隔离等级、平台制品、资源限额和贡献插槽全部满足。
5. 依赖图可解且不存在版本或资源冲突。
6. 发布与依赖均未撤销；策略和授权租约有效。

结果包括 `compatible | incompatible | authorization-required`，以及稳定错误码、失败字段、实际/期望值、可用与不可用模块。市场页面可展示不兼容项并说明原因，但安装和激活都必须重新校验。

验证记录单独存储：`releaseDigest + applicationId + hostBuildDigest + executor/OS + suiteRevision + result + evidenceRef`。可声明兼容尚未验证，不可虚构“所有 EOS 应用验证通过”。

## 6. HostAdapter 与插件开发接口

所有宿主必须实现逻辑接口；语言 SDK 可异形但语义一致：

```text
describe() -> HostDescriptor
currentContext() -> SubjectContext
confirmPlan(plan) -> GrantDecision
selectResource(request) -> ScopedResourceHandle
invokeCapability(boundContext, capability, operation) -> Result
mountContribution(boundContext, contribution) -> Disposable
storage(boundContext) -> ScopedStorage
credential(boundContext, requirement) -> OpaqueCredentialHandle
observeContextChanges() -> Subscription
reportLifecycle(event), reportDiagnostic(event)
```

能力目录记录 owner、版本、输入/输出 Schema、可用主体、资源语义、是否有副作用、审批要求、幂等/取消语义和配额。宿主只注册实际实现且符合合同的能力。共享能力新增/变更需契约评审，产品私有能力使用产品命名空间。

插件上下文不暴露任意 `api.get(url)`、数据库连接、宿主 DOM 或 Go/Rust 内部对象。能力调用是经过身份绑定的 RPC；提供者仍执行用户/资源授权，插件授权不能把用户本来无权访问的数据开放给它。

逻辑消息需可绑定实例、代次和期限；当前页面 SDK 消息实际为 `{id,method,params}`，响应为 `{id,result}` 或 `{id,error:string}`，由宿主在内部封装安装 ID、moduleId 和 generation。不能把目标消息字段当作当前页面 SDK 的字段要求。传输层绑定主体和作用域，不让插件填写它们来换身份。取消作为明确消息传递；迟到响应及上一代实例的消息丢弃并记录。

能力副作用请求含 `operationId`，提供者声明是否支持去重和结果查询。超时或进程崩溃后未知结果不能自动重放；只有合同明确幂等的操作才能按同一 ID 重试。

激活方式：`on-demand/on-command/on-event/on-startup`。后台、启动和事件激活必须可见、可禁用并额外授权；事件内容按订阅能力裁剪，不广播所有会话。设置超时、速率和循环触发上限，取消订阅与实例生命周期绑定。

## 7. 执行器合同

所有执行器提供 `validate/prepare/start/stop/health/dispose`，明确超时、取消、资源限额、进程/实例归属和退出结果。注册成功与功能可用分别报告，不能仅检测到 `[tools]` 就显示 MCP 可用。

- **Skills**：独立来源命名空间 `pluginId/moduleId/localName`，避免普通技能同名覆盖。禁用/卸载必须撤销索引；指令不是权限，技能引用的动作仍通过宿主能力与审批。
- **MCP stdio**：包内固定入口和受管理运行时、显式环境变量白名单、受限工作目录、不继承账号秘密。隔离等级是签名合同的一部分；受信进程要求组织策略和用户批准，无法满足时拒绝。
- **MCP HTTP**：固定允许端点或经过授权的用户配置端点；凭据由宿主 OAuth/凭据库处理，模块拿到受限连接句柄。远程服务自己的存储与权限不因接入 EOS 就获得沙箱保证。
- **WASM Component**：固定 WIT world/ABI，按导入函数授予能力，CPU fuel/超时、内存和并发限额。宿主没实现某 WASI 接口则导入失败，不开启全量目录/网络来“使其可运行”。
- **Webview**：独立页面，通过隔离 origin、sandbox、CSP、Permissions Policy 和消息端口组合约束；不与宿主共享 React 等依赖、不在宿主 window 注入脚本、不提供原始 Wails/Tauri/浏览器原生桥。

Webview 默认禁止网络直连、表单、顶层跳转、弹窗、下载、摄像头、麦克风和剪贴板；必要行为经版本化能力代理和授权。完整包资源必须先验证，加载资源来自已验证字节，不在入口校验后又从可变 CDN 获取动态 import。

消息握手以当前 iframe/window 来源、一次性 nonce 与专用 MessageChannel 绑定；对 opaque origin 不把 `origin = null` 当信任。校验方法白名单、消息大小、generation 和速率，销毁视图即撤销端口。宿主不可同时为同源插件页面开放会破坏隔离的 sandbox 权限组合。

资源选择返回句柄而非任意路径；文件句柄验证真实路径、符号链接替换及父目录逃逸。网络代理验证域名、端口、DNS 解析、重定向与实际连接地址，默认阻止云元数据和未授权私网；本机工具连接需要单独能力。受信原生进程能够绕过代理的事实必须显示在其信任等级中。

插件导出能力时，消费者和提供者分别鉴权。提供者不能因收到另一个插件调用而继承调用者的全部权限；委托必须显式、缩小范围并设置有效期。

## 8. 权限、配置与数据

授权记录至少包含安装 ID、模块、发布清单摘要、能力、操作、资源句柄、执行位置、信任等级、授权主体、策略版本、有效期和撤销代次。

有效权限 = 请求上限 ∩ 用户授权 ∩ 当前主体权限 ∩ 组织/服务端策略 ∩ 宿主策略 ∩ 执行器限制。

平台提供配额内的插件私有键值存储作为基础上下文，只包含该安装自己的数据，不需要额外用户数据授权。访问文件、业务数据、其他插件数据、跨实例存储或秘密均不是该基础权限的一部分。

`trusted-process` 是粗粒度本机执行授权：进程可能访问当前用户可访问的系统资源，不能通过其他细粒度声明暗示已限制其直接 OS 访问。目标策略要求受信进程经过额外审核与批准。当前兼容引擎按 profile 和用户确认判定，不含“仅官方”专用执行闸；上传白名单与人工发布审核负责现阶段准入，不能将其描述为已有的按发布者自动隔离策略。

普通配置按 Schema 校验。秘密只存宿主凭据库/服务端秘密存储，支持轮换与撤销，不进入 manifest、包、同步配置、URL、模型上下文或日志。需要进程原始秘密的受信连接必须专门披露最小凭据注入范围。

默认不跨设备同步安装或数据；启用同步需产品提供显式功能及用户选择。允许同步包选择和非秘密配置，仍需在新设备重新取得资源句柄和本机授权。

升级时对比能力、操作、资源范围、模块、执行位置及信任等级。任何扩大必须新确认；拒绝后保持旧有效安装，不修改其授权或配置。缩小授权可以自动收紧，不能用“同一 ID”复用已撤销授权。

数据版本与插件版本独立。迁移在最小权限执行器内对暂存副本执行，禁止外部业务副作用，成功才提交。无法隔离的迁移标成不可逆并禁止静默升级；必须先导出/备份并明确用户确认，失败不宣称可自动回滚外部系统。

## 9. 安装、升级与卸载事务

本地管理入口：`search/detail/resolve/prepare/confirm/install/list/enable/disable/update/remove/diagnose`；旧 `source`/`name` 接口在切换时一次迁移，正式市场安装以 ID、版本、制品摘要和计划为准。

### 9.1 安装计划

`prepare` 生成持久化计划：安装目标、精确依赖图、选定模块与制品摘要、权限差异、资源/磁盘需求、数据迁移、策略版本、有效期及 `planDigest`。确认绑定这个摘要，不能只带一个 `confirm_permissions=true`。

确认后若目录指针变了，仍只能安装计划中的原摘要；原发布撤销、策略改变或计划过期则计划失效并重新确认。当前 Git 分支变化不能出现在正式市场确认流中。

### 9.2 状态机与提交点

```text
absent -> planned -> awaiting-consent -> downloading -> verifying
       -> staging -> prepared -> activating -> active
active -> disabling -> disabled -> enabling -> active
active/disabled -> update-staging -> update-prepared -> switching -> active(new)
active/disabled/failed -> removing -> removed(data-retained or purged)
any applicable state -> failed | incompatible | policy-blocked | revoked
```

确认前不执行插件。下载可在受限缓存内预取，不能激活。预备阶段校验并准备执行资源、数据副本和贡献变更；持久化事务日志后进入提交。共享管理器协调制品引用、安装记录及宿主资源。

所有同作用域相关依赖安装使用确定锁顺序。提交点切换 active generation；允许预备的执行器中新实例就绪前旧实例保留，切换期间冻结冲突副作用调用。激活前所有代理副作用能力关闭，只有 activate commit 后开放。宿主贡献不能依赖无法补偿的直接页面修改。

提交失败按事务日志撤销新注册资源、恢复旧数据/指针并报告失败。崩溃后重启按日志恢复或完成同一事务，不靠重新扫描目录猜状态。资源注册是幂等的，按安装 ID/generation 可枚举并清理。MCP、技能、UI 和订阅不得残留。

远程/外部 MCP 可能在连接时产生副作用；预激活只允许合同定义的握手，不调用业务工具。若执行器无法保证准备阶段边界，则切换须停旧再启动新，并显示中断；不能用“无缝升级”掩盖实际限制。

`trusted-process` 能直接访问 OS，代理开关无法拦截其启动副作用。因此 prepare 只做静态检查和资源暂存；用户确认后采用明确的停旧/启新切换，不在旧实例旁预启动。安装记录与受管资源可恢复，但新进程已执行的 OS/远端副作用不在自动回滚保证内。需要细粒度副作用保证的插件必须使用真实受限执行器。

禁用先封闭入口/代理句柄，取消调用、卸载贡献、停止可控进程；超时强制终止并报告未确认外部操作。重新启用重检策略与租约。卸载默认保留数据，清除数据是单独用户选择，不清除共享依赖正在使用的内容。

新版本连续启动失败可隔离该安装并提供显式回滚。只有原版本仍可信、授权有效且数据可恢复时才可回滚；安全撤销版本禁止通过回滚重新运行。

## 10. 市场数据与 API

逻辑数据模型（实际迁移表名由实施映射，已执行迁移保持不变）：

- `publishers/members`：命名空间、审核状态、发布角色。
- `applications/capability_definitions`：组合层注册的应用、能力、Schema、所有者。
- `plugins`：永久 ID、发布者、展示信息和访问受众。
- `plugin_releases`：不可变清单、SemVer、生命周期、来源和审查结果。
- `plugin_artifacts`：目标、包/清单/文件清单摘要、长度和对象键。
- `plugin_channels`：环境、受众、频道、发布 ID、revision。
- `plugin_audiences`：internal/organization/private/public 及允许主体。
- `plugin_revocations`：对象、理由、revision、生效时间与签名元数据关联。
- `plugin_verifications`：绑定发布与宿主构建的验证证据。
- `plugin_audit_events`：发布、ACL、撤销、轮换和管理行为。
- 本地安装、授权和配置为运行时主记录；仅在明确提供设备同步时上传最小必要镜像。

当前 API 前缀为 `/api/v1/plugin-market`；管理前缀为 `/api/v1/admin/plugin-market`。已实现路由如下（请求、响应、身份和示例见[发布指南](developer/PUBLISHING.md)）：

```text
GET  /catalog                         # 匿名已发布目录
GET  /plugins                         # 匿名网页列表
GET  /plugins/{id}                     # 匿名详情
POST /search                          # 匿名，带 HostDescriptor
GET  /upload-access                   # 账号检查上传资格
POST /uploads                         # 账号 + 白名单，multipart package/targets
POST /sessions                        # 账号 -> 5 分钟应用绑定会话
POST /resolve                         # 账号 + 应用会话
GET  /trust-bundle                     # 账号 + 应用会话；可带 manifestDigest query
GET  /policy                          # 账号 + 应用会话
POST /download-authorizations         # 账号 + 应用会话
GET  /artifacts/{digest}               # 再带 X-Plugin-Download-Token

GET  /api/v1/admin/plugin-market       # 管理目录；无尾部斜线
POST /uploads                         # 管理入口也需上传白名单
POST /releases/{id}/{version}/review   # JSON {revision}
POST /releases/{id}/{version}/publish  # 审核后独立签名
POST /releases/{id}/{version}/withdraw
POST /releases/{id}/{version}/revoke
```

发布者创建、单独 validate、releases 创建接口、频道、audience 和 verification 管理属于扩展目标，当前未暴露这些路由；`/trust/{roleMetadata}` 也不是实际 API。上传成功一次创建不可变完整 Release，状态为 validated；包格式检查不等于通用恶意代码扫描。

管理接口路径以上述管理前缀解释（目录 GET 使用前缀本身，无尾部斜线）。上传在隔离暂存区，不能通过猜测路径公开访问。当前服务检查上传资格、已批准发布者、完整制品、格式和限额；逐发布者成员授权及通用扫描仍为目标，审核者须人工核对来源和执行内容。签名服务只签固定本地目录，不能签任意未经审查 URL。

当前发布状态：上传成功为 `validated`，审核后 `approved`，签名后 `published`，可下架为 `withdrawn`；withdrawn 可经管理员重新 publish 相同字节。approved/published/withdrawn 可安全撤销为 `revoked`。当前无 draft/rejected 创建或处理入口；修正包要使用新版本。发布内容一旦创建即不可替换，未发布包可按保留策略回收，但 ID/版本占用和审计保留。

当前发布/审查更新使用 revision 防并发覆盖。频道指针的独立管理仍为目标。下架可经审查重新发布相同字节，撤销不可静默恢复；修复需新版本和新审查。

不存在全平台一个 `enabled` 控制所有客户端。市场发布状态、组织安装策略、宿主安装启用状态分开存储。新公开模式也不能因“已发布”而绕过组织 ACL 或产品权益。

下载许可初始 TTL 5 分钟，绑定主体、环境、摘要和 audience revision；应用退出/ACL 改变后新请求拒绝。私有对象桶不公开；优先鉴权代理流式下载，并验证每次请求/续传。若以后用预签名 URL，必须明确泄露窗口与不带账号 Cookie 的独立制品域。

ETag/缓存必须按环境和访问主体/受众 revision 分区，不能把内部列表命中到匿名访问。未授权私有插件返回与不存在一致的响应，避免枚举。

## 11. 策略租约、撤销与离线

签名运行策略绑定环境、发布摘要、策略 revision、受众与有效期。初始最大离线运行租约为 24 小时，组织/能力可收紧；当前应用级心跳每 4 分钟检查，插件页面打开时另有每 5 分钟同步；启动、启用、升级及高风险授权前也检查。服务端 revoke 或 ACL 改变不向离线客户端承诺即时生效。

- 普通下架：停止新安装/升级解析，已有安装在授权租约内可运行。
- 安全撤销：阻止激活，关闭代理能力、终止可控实例；依赖者进入 `policy-blocked`。
- 租约过期：关闭受管实例并显示 `policy-blocked`，恢复联网并取得有效策略后才可重新启用；不静默绕过。
- 设备时间回拨：结合已见签名时间、元数据版本及单调时间检测，发现无法确认有效期则要求联网；客户端被用户完全篡改不在软件隔离保证内。
- 包回滚：TUF 元数据版本始终前进，频道可以经批准指向旧但仍可信包；不能为了回滚接受旧签名快照。

只控制 EOS 管理的执行与代理能力，不声称能撤销受信进程已产生的外部副作用，或阻止已获原始数据的远端服务保留数据。

## 12. 错误、日志与可观测性

当前共享错误结构：`code/message/details/retryable`；HTTP 包在 error 中，SDK 页面接收 error 字符串。operationId/planId/instanceId 与 messageKey 是扩展诊断目标，不应假设每个错误对象都有这些字段；details 对用户可见内容脱敏。错误码至少包括：

```text
UNAUTHENTICATED, NOT_FOUND_OR_FORBIDDEN, SCHEMA_UNSUPPORTED,
HOST_INCOMPATIBLE, CAPABILITY_UNSUPPORTED, EXECUTOR_UNSUPPORTED,
ISOLATION_UNAVAILABLE, DEPENDENCY_CONFLICT, TARGET_AMBIGUOUS,
PLAN_EXPIRED, PLAN_CHANGED, CONSENT_REQUIRED, GRANT_REVOKED,
ARTIFACT_HASH_MISMATCH, SIGNATURE_INVALID, METADATA_EXPIRED,
ARCHIVE_UNSAFE, QUOTA_EXCEEDED, PACKAGE_REVOKED, POLICY_LEASE_EXPIRED,
ACTIVATION_FAILED, MIGRATION_FAILED, OPERATION_OUTCOME_UNKNOWN
```

生命周期事件记录安装 ID、版本/摘要、应用/构建、模块、generation、状态转移、耗时和错误，不记录秘密、完整用户文件或模型上下文。诊断默认本地，上传需遵循应用现有可观测性设置。

管理审计含操作者、角色、资源、revision、请求 ID、结果及发生时间。统一“诊断导出”可查看清单、兼容拒绝原因、版本、授权类别和脱敏日志；不能导出凭据。

## 13. 开发模式与迁移

目标开发模式若引入本地路径/固定 Git commit，必须显式启用；当前 CLI/Agent 尚无该入口。该目标要求：清楚标记“开发、未发布信任”。开发插件单独目录/配置，不能写正式缓存或在重启后悄悄获得生产信任；开发模式不放宽能力代理权限。

正式运行只使用新合同。一次性迁移按预先盘点结果执行：静态索引样例不自动上架；旧 Agent name 安装生成受控 ID 映射并转换注册引用；后台 `plugin.json` 转为 `plugin.toml` 与隔离入口。未知来源/冲突必须报告并停用，不能自动映射为官方插件或删除数据。

迁移工具不参与服务启动或日常请求。先备份、干跑、验证计划，再切换单一运行路径；旧已执行 SQL 保留，增加新迁移。旧源码和备份提供部署回滚，不把旧协议做成长驻 fallback。

## Marketplace display localization

Existing `plugin.i18n` language-to-resource paths and ordinary UI resources keep their semantics. A bounded JSON object may explicitly reserve the top-level `$eos.marketplace` namespace containing exactly `schemaVersion: 1`, `name`, and `description`. Ordinary UI keys alongside this namespace remain untouched. No manifest field is added; existing strict manifest readers continue to parse these packages. Current EOS hosts only validate declared i18n asset paths/existence and do not interpret their JSON contents.

Only a valid JSON object with this namespace is projected. Non-JSON UI assets, JSON without it, and files above the 1 MiB discovery bound produce no display projection. An explicitly discovered namespace must satisfy strict validation: reject duplicate/unknown namespace fields, unsupported version, blank text or control characters. Limits: 32 projected locales, 16 KiB per display resource, 256 UTF-8 bytes for name and 8192 for description; language tags are canonical lower-case (en, zh, pt-br). All artifacts of a release must yield identical projections. Rendering escapes plain text; resources are never executable HTML.

Public web APIs expose separate `display[manifestDigest][locale] = {name, description}` data derived at upload from validated package bytes, stored separately and returned only for reviewed published releases. Never modify the signed manifest, release identity, installation plan, permission reasons or artifact digests. Missing translations use original manifest text. Old ordinary UI resources are not reinterpreted. Historical signed packages are never rewritten. A new official plugin version still requires normal review and signing before publication.
