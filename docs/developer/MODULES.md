# 模块开发手册

下列示例都在仓库 `examples/`，使用演示 publisher。先完成[快速开始](QUICKSTART.md)，再按需要增加模块。一个插件可包含多种模块，是否能安装仍取决于真实宿主描述和制品目标。

## Skills：提示与知识

布局为 `skills/<local-name>/SKILL.md`，模块 entry 指向 `skills`。宿主只扫描子目录中的 SKILL.md，不把 `skills/SKILL.md` 当作一个技能。建议每个技能一层目录且 name 唯一。

前置元数据使用简单单行键值，不是完整 YAML 引擎：

```markdown
---
name: summarize
description: 将用户提供的文字整理为三条要点。
short-description: 三点摘要
allowed-tools: []
user-invocable: true
disable-model-invocation: false
argument-hint: 待整理文字
---

这里写明确的处理步骤、事实边界和输出格式。
```

name 必须非空；description 建议始终提供；布尔值写 true/false；不要用多行 YAML、锚点或列表块。tags 使用逗号分隔文本。`allowed-tools` 可写 `[]`、`[read_file, shell_exec]` 或逗号列表；重复或非法声明会导致技能不能加载。工具名称必须来自目标宿主，声明只收窄，不能增加原来没有的权限。

| allowed-tools | 显式技能调用行为 |
| --- | --- |
| 省略 | 继承本轮既有工具策略 |
| `[]` | 本轮无工具，模型目录与执行入口均限制，续跑保留 |
| 非空列表 | 与本轮既有权限取交集 |

凡依赖该策略，模块 requires 都要加入 `eos.ai.skills.tool-policy ^1.0.0`。仅凭提示正文写“不要用工具”不等于宿主限制。

安装后完整名称为 `pluginId/moduleId/frontmatter.name`，例如 `tutorial.hello-skills/writing/summarize`。测试时明确选择完整技能名，避免只靠自然语言名称猜触发。技能本身不自动运行旁边的 scripts、不自动读取 references，也不自动获得文件权限。

## 独立页面：一个 HTML 与消息端口

当前宿主读取 entry 的 HTML 字符串，通过 `srcdoc` 放入 sandbox iframe。CSP 只允许内联脚本/样式、data 图片/字体，禁止网络和表单。**不会提供相对资源服务器**；普通 Vite 输出的 `/assets/*.js`、外链 CSS、动态 import、CDN 依赖不能直接运行。把依赖和 SDK 全部内联进单个 HTML，入口至多 4 MiB。

SDK 名称为 `@eos/plugin-sdk`，当前 package.json 为 private，不应执行 `npm install @eos/plugin-sdk` 并假定 npm 有可用发布。示例构建时从仓库 `sdk/typescript/src/client.ts` 引用并内联。插件最终用户不需要仓库或 Node。

```toml
[[modules]]
id = "panel"
type = "webview/v1"
required = true
entry = "ui/index.html"
activation = "on-command"
requires = [{ capability = "eos.ui.panel", version = "^1.0.0" }]
permissions = []
contributions = [{ kind = "panel", id = "notes", title = "便签" }]
```

源码中的使用方式（由构建工具打包）：

```ts
import { connectPlugin } from "../../../sdk/typescript/src/client";

const client = await connectPlugin();
const config = await client.request<{ title: string }>("configuration.get");
await client.request("storage.set", { key: "draft", value: "hello" });
const draft = await client.request<string | null>("storage.get", { key: "draft" });
await client.request("storage.delete", { key: "draft" });
// 页面销毁或自定义退出时关闭端口。
window.addEventListener("pagehide", () => client.close());
```

### 当前 SDK 方法

| request(method, params) | 成功结果 | 说明 |
| --- | --- | --- |
| configuration.get, `{}` | 安装配置 JSON | 无配置通常是 `{}` |
| storage.get, `{key}` | 值或 null | key 非空，最多 256 字节 |
| storage.set, `{key,value}` | null | value 必须可 JSON 序列化；安装总数据 1 MiB |
| storage.delete, `{key}` | null | 不存在的键也成功 |
| capability.invoke, 见下文 | 能力定义的 JSON | 必须由本模块声明权限并获授权 |
| log, 任意 JSON | `{accepted:true}` | 当前只是确认接收，不是可查询的持久日志服务 |

SDK Promise 失败为 Error，message 携带宿主错误文本；当前没有稳定的 typed error.code 对象。握手超时 10 秒、请求超时 30 秒、最多 10 个未完成请求、消息最多约 1 MiB；宿主还实施每秒 10 条限制。超限被丢弃可能表现为超时，适当防抖保存，不能把每个键盘事件都直接变成存储调用。

握手 nonce 由宿主注入；只接收父窗口传来的专用 MessagePort。插件不能访问账号 token、宿主 DOM、Wails/Tauri 原生桥或任意文件系统。单独打开 package/ui/index.html 会报 PLUGIN_HOST_REQUIRED；页面调试用下面的模拟宿主。

### 完整页面示例

以下命令从仓库根目录开始，Node 22 为本次验证版本：

```sh
cd examples/notes-panel
npm ci
npm run build
cd ../..
mkdir -p ../plugin-artifacts
cargo run --locked -p eos-plugin-cli -- pack examples/notes-panel/package ../plugin-artifacts/notes-panel-0.1.0.eos-plugin
cargo run --locked -p eos-plugin-cli -- validate ../plugin-artifacts/notes-panel-0.1.0.eos-plugin
```

`package/` 只放发布资源；开发源码、npm 依赖和模拟宿主在外。安装填写 `{"title":"我的便签"}`。点击“打开便签”，输入内容并保存，关闭再打开应保留。缺标题时配置校验失败，不会自动应用 Schema default。

本地开发可在仓库根运行：

```sh
python3 -m http.server 8765 --bind 127.0.0.1 --directory examples/notes-panel
```

打开 `http://127.0.0.1:8765/dev-host.html`。模拟宿主用同样的 iframe sandbox/CSP 和真实 SDK，提供演示配置与 storage；**数据只在该页面会话内存中保留，刷新即重置，不能验证原生桥隔离、真实授权或持久化**。改 TS 后重新 build，再刷新页面，无生产数据写入。

### 调用自己的技能

在同一个插件增加 Skills 模块、[权限声明](MANIFEST.md)后：

```ts
await client.request("capability.invoke", {
  capability: "eos.ai.skills",
  operation: "invoke",
  input: { name: "tutorial.hello-skills/writing/summarize", arguments: "待整理文字" }
});
```

名称必须属于当前插件。当前返回技能激活结果，**不是模型摘要文本**；`arguments` 的存在也不代表宿主替页面发起了新一轮对话。需要模型输出的 UI 流程必须由宿主另行提供能力，不能绕过 SDK 直接拿账号调用模型。

## 本机 MCP：固定原生入口

```toml
[[modules]]
id = "echo"
type = "mcp-stdio/v1"
profile = "trusted-process"
required = true
entry = "native/echo"
activation = "on-demand"
requires = []
permissions = []
args = []
env = {}
```

entry 是包内实际可执行文件，不是系统命令、npm 包名或 shell 表达式。Unix 需执行位；Windows 使用真实 `.exe` 路径。宿主清空继承环境，把工作目录设为可执行文件父目录，再传 args/env；不会把 `{pluginDir}`、`${config.foo}` 等字符串做插值。不要依赖目标机 PATH、Python、Node 或登录环境。发布原生可执行文件时随包带齐目标平台运行依赖，且遵守包的无链接限制。

trusted-process 以当前用户权限运行，可直接访问本机资源；`permissions=[]` 不会把它变成沙箱。安装会展示这一授权，启动 MCP 时会探测 initialize/tools/list，因此即使写 on-demand 也不应在进程启动或握手阶段产生业务副作用。工具实际调用继续走 Agent 的工具审批。

stdio 使用每行一条 JSON-RPC，stdout 只能输出协议；诊断不要写入 stdout。当前宿主丢弃该 MCP 子进程 stderr，本机排障请先独立运行或用合成输入重现，不要承诺在 eos-core.log 里能找到子进程 stderr。

可编译示例 `examples/mcp-echo` 不依赖目标机脚本解释器：

```sh
cargo build --locked --manifest-path examples/mcp-echo/Cargo.toml --release
python3 examples/mcp-echo/stage.py
cargo run --locked -p eos-plugin-cli -- pack examples/mcp-echo/package ../plugin-artifacts/mcp-echo-0.1.0.eos-plugin
python3 examples/mcp-echo/smoke.py
```

stage.py 仅把当前机器构建的二进制复制为发布目录的 `native/echo.exe`，Unix 上也沿用这个文件名以保持跨平台清单一致；它不交叉编译。smoke.py 发 initialize、tools/list、tools/call，必须得到 `echo:hello`。按实际编译的 OS/架构上传，不能标为 any/any。多平台发布必须维持同一清单原始字节；可为所有平台统一使用同一个包内文件名并由构建流程保证可执行格式，或分别设计固定包装入口；不要为同一 Release 上传清单不同的 Windows/Unix 包。

## 远程 MCP：固定 HTTPS 服务

```toml
[[modules]]
id = "remote"
type = "mcp-http/v1"
required = true
endpoint = "https://your-mcp.example.com/mcp"
activation = "on-demand"
requires = []
permissions = []
```

把示意域名换成你管理的真实服务。当前使用 streamable-http，禁止私网地址、重定向、URL 内用户名密码和隐式环境代理，DNS 解析后固定连接地址。纯 HTTP/localhost MCP 不适用于当前执行器；普通浏览器能打开不代表 MCP 协议可用。

当前 Agent 没有 OAuth/秘密代理；此模板适合不需要私密凭据的受控 HTTPS 服务。不要把 token 塞 URL 或 env 绕过。远程服务的权限、保留数据和费用由服务本身负责。

## WASM Component：纯计算工具

要求 **Component Model 组件**，不是普通 Core WASM 模块，也不是旧扩展的 WASI 插件。固定 WIT：

```wit
package eos:plugin@1.0.0;
interface host {
  call: func(capability: string, operation: string, input: string) -> result<string, string>;
}
world component {
  import host;
  export describe: func() -> string;
  export invoke: func(method: string, input: string) -> result<string, string>;
}
```

当前执行器不提供 WASI imports；编译产物需要匹配该 world，不能为了让 wasi:filesystem 或 wasi:cli 导入通过而开放宿主权限。无须调用宿主时可以没有 host 导入。

`describe()` 返回 JSON 字符串：

```json
{"tools":[{"name":"ack","description":"返回确认结果","inputSchema":{"type":"object","additionalProperties":false}}]}
```

1–32 个工具；name 为 1–32 个 ASCII 字母、数字或下划线；inputSchema 是对象；顶层只接受 tools，每项只接受 name、description、inputSchema；未知字段会拒绝。`invoke("ack", "{}")` 返回 `ok("{\"ok\":true}")`，或 error 字符串。成功内容必须是合法 JSON。宿主自动生成带安装/代次摘要的工具名称，更新后会变化，不要硬编码完整工具名。

模板 `examples/wasm-ack` 使用可读的 `.wat` 组件，不依赖额外组件编译工具；describe/invoke 的 canonical ABI 内存布局已写好：

```sh
cargo run --locked -p eos-plugin-cli -- pack examples/wasm-ack ../plugin-artifacts/wasm-ack-0.1.0.eos-plugin
cargo run --locked -p eos-plugin-cli -- validate ../plugin-artifacts/wasm-ack-0.1.0.eos-plugin
```

安装后应能在工具目录找到该插件的 ack 工具并得到 `{"ok":true}`。把工具变成实际业务时，输入解析、错误和输出由你实现；当前实例每次调用重建，资源限额见[清单参考](MANIFEST.md)。只有 pack/validate 不会验证 WIT 或 describe，必须执行[运行验收](DEBUGGING.md)。

WASM 可用 host.call 获取普通配置；调用 eos.ai.skills/invoke 需权限，返回的是技能激活确认。迁移使用 method=migrate；契约与测试要求见[数据迁移](MANIFEST.md)。当前不提供通用文件系统、任意网络或插件间 RPC。
