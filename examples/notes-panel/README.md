# 便签开发项目

在本目录执行 `npm ci`、`npm run build`，只把 `package/` 传给 eos-plugin pack。

仓库根运行 `python3 -m http.server 8765 --bind 127.0.0.1 --directory examples/notes-panel`，打开 `http://127.0.0.1:8765/dev-host.html`。
点击保存，点击关闭并重新打开，应恢复数据；浏览器整页刷新后模拟数据重置。更改 src 后重新 build。

`node smoke.mjs` 在 Node 中验证真实打包 SDK 的握手、配置读取、存储读写和错误显示；不是浏览器/CSP 或原生 WebView 安全验收。

真实 Agent 安装配置为 `{"title":"我的便签"}`，需要宿主实际声明 webview/v1。教程见 [模块开发](../../docs/developer/MODULES.md)。
