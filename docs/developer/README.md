# EOS 插件开发手册

核实日期：2026-11-12。面向插件作者；不要求阅读 EOS 实现源码。本文描述经过源码核实的开发方式；实际可用模块以你使用的宿主版本和能力声明为准。

## 阅读顺序

1. [快速开始](QUICKSTART.md)：从空目录做一个 Skills 插件，打包、检查兼容性、在测试市场安装并调用。
2. [清单与兼容性](MANIFEST.md)：逐项解释清单、权限、配置、目标平台、多语言和当前能力边界。
3. [模块开发](MODULES.md)：Skills、独立页面及 SDK、原生 MCP、远程 MCP、WASM 的输入输出与完整示例。
4. [调试与排障](DEBUGGING.md)：本地检查、页面模拟宿主、真实 Agent 验收、日志和错误处理。
5. [上传、审核与发布](PUBLISHING.md)：作者与管理员各自操作、真实 API、不可变版本、下架和撤销。

## 先确认能做什么

当前 EOS Agent 的源码提供 Skills、本机受信进程 MCP、固定 HTTPS MCP、无 WASI 的 WASM Component。独立页面仅在宿主实际报告 `webview/v1` 和 `eos.ui.panel` 时可用；macOS 原生隔离校验通过才会声明，不能据此承诺 Windows/Linux 页面支持。

任务读写、任意文件/网络代理、OAuth/秘密代理、跨插件导出调用、服务器执行尚不是当前 Agent 公布的能力。`portable` 表示按能力适配，不表示全部 EOS 产品均已支持。Todo 导入器等设计示例不能作为可运行模板。

市场允许公开浏览；安装需要登录；上传还需要邮箱白名单和已批准的发布者。当前没有发布者自助注册接口。请在首次测试市场上传前让维护者确认命名空间与环境。

## 工具与样例

`eos-plugin` 是打包/校验工具，不是应用安装器。当前没有 `init`、`install`、`dev`、`watch` 或 `publish` 子命令，也没有桌面端本地目录热重载入口。真实安装走测试市场的审核签名流程；参见[调试路径](DEBUGGING.md)。

| 示例目录 | 用途 | 构建 |
| --- | --- | --- |
| `examples/hello-skills` | 纯文本整理技能 | 直接 pack |
| `examples/notes-panel` | 普通配置、私有存储、单文件页面 | npm ci 后构建 package/ |
| `examples/mcp-echo` | 不依赖目标机 Node/Python 的原生 MCP | Cargo 构建并复制到 package/ |
| `examples/wasm-ack` | 最小 Component Model 工具 | 直接 pack；WAT 由当前执行器支持 |

样例使用未注册的 `tutorial` 命名空间，不能直接上架；开发自己的插件时替换所有 ID/名称及相互引用。模板许可证仅覆盖模板文件，不改变 EOS 平台本身的许可证。

## 下一步

从[快速开始](QUICKSTART.md)完成第一个插件；已有插件要接入新模块时，查阅[模块开发](MODULES.md)。提交发布前按[调试验收清单](DEBUGGING.md)记录实际结果，再完成[上传与发布](PUBLISHING.md)。
