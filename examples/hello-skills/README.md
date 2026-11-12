# 三点摘要（教学示例）

将用户提供的文字整理为最多三条要点。需要宿主模型及 Skills/tool-policy 能力，模型计费沿用宿主设置。

安装后显式调用 `tutorial.hello-skills/writing/summarize` 并提供文字；本轮无工具，不读取文件或执行外部动作。

验证输入：已完成搜索接口。导出尚未决定。小林周五提供方案。

预期：保留三条事实，不把导出写为已决定，不请求工具。空输入应要求提供文字。

本目录可直接 pack。tutorial 未注册；上架前替换获批发布者和 ID。开发教程见 ../../docs/developer/QUICKSTART.md（仓库内链接，非包内依赖）。

许可：MIT，见 LICENSE。
