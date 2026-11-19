# 原生 MCP 开发项目

从仓库根目录执行：

```sh
cargo build --locked --manifest-path examples/mcp-echo/Cargo.toml --release
python3 examples/mcp-echo/stage.py
python3 examples/mcp-echo/smoke.py
```

只打包 package/。native/echo.exe 在所有平台使用相同文件名，Unix 也可执行带 .exe 后缀的本机二进制；文件内容仍为平台原生格式。
stage 保留执行位，不交叉编译。上传目标必须反映实际编译环境，不是 any/any。
进程初始化不读用户文件、不联网；trusted-process 仍具有当前用户的 OS 权限，并不是沙箱。

开发教程见 [模块开发](../../docs/developer/MODULES.md)。
