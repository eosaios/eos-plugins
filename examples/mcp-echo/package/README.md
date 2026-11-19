# MCP Echo（教学示例）

需要 mcp-stdio/v1 trusted-process。以当前用户权限执行原生程序，不是操作系统沙箱。
工具 echo 接受 {"text":"hello"}，返回文本 echo:hello；stdout 仅输出 MCP JSON-RPC。
启动和握手不读取用户数据、不联网。原生包只适合实际构建的 OS/架构，不能标为 any/any。

tutorial 未注册；上架前换获批身份。许可 MIT，见 LICENSE。
