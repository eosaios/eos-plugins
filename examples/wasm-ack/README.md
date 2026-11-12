# WASM 确认工具（教学示例）

需要 wasm-component/v1 ABI 1.0.0。安装后 ack 工具接受对象并返回 {"ok":true}。
组件使用 WAT 编码，不导入 WASI 或宿主能力，无文件/网络访问，不依赖目标机解释器。
示例的 ack 为固定返回，仅用于理解 describe/invoke 的组件 ABI；业务输入解析需自行实现。

tutorial 未注册；上架前换获批身份。本目录可直接 pack；执行验证需要真实宿主，validate 不验证 ABI。许可 MIT，见 LICENSE。
