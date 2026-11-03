# 内部市场运行与交付

当前代码默认不启用市场。配置完成后仍需账号登录、内部成员授权和应用会话；没有公共市场开关，也不会把 plugins.json 样例自动上架。

## 服务端配置

在现有 base + pro 组合服务中设置：

| 环境变量 | 含义 |
| --- | --- |
| EOS_PLUGIN_MARKET_PRIVATE_DIR | 绝对路径，必须位于公共 storage 目录之外 |
| EOS_PLUGIN_MARKET_ENVIRONMENT | 明确的环境 ID，例如 test；与客户端相同 |
| EOS_PLUGIN_MARKET_TRUST_ROOT | 已经过密钥仪式确认的公开 TUF root.json 绝对路径 |
| EOS_PLUGIN_MARKET_SIGNER | 固定 eos-plugin 签名工作进程的绝对路径 |
| EOS_PLUGIN_MARKET_SIGNING_KEYS | 在线 targets/snapshot/timestamp 私钥路径列表，使用系统路径分隔符 |
| EOS_PLUGIN_MARKET_APPLICATIONS | 中性 base 使用 JSON 应用 ID 列表；pro 从 eos-products 注入 eos-agent |

私有目录使用 0700。只允许服务身份和隔离签名设施访问私钥。离线根私钥不应出现在在线服务机器、源码、镜像或环境变量中。API 只向固定签名程序传递受审查的本地目录，工作进程清空继承环境、限制完成时间；请求不能指定程序、密钥或远程签名 URL。

新增 SQL 为 base 的 `202611030001_plugin_market.sql`、`202611030002_plugin_market_runtime_privileges.sql` 与 pro 的 `202611030010_plugin_market_official_publisher.sql`，不重放历史种子。pro 注册官方 eos 命名空间，不自动授予任何用户内部成员资格。

## 客户端信任

生产 Core 编译时固定 `eos-core-app-server/trust/plugin-market-production-root.json`，不接受运行环境覆盖；该公开根由正式密钥仪式产生，根角色为 2/3 阈值，与测试根和三类在线角色密钥分离。根私钥以加密文件保存在离线运维设备，解密凭据使用设备钥匙串；在线服务器只保存 targets/snapshot/timestamp 密钥。测试使用独立的 `EOS_PLUGIN_MARKET_TEST_ROOT` 文件。禁止首次下载自动信任，测试凭据和根不能用于生产。

Core 从 Go 壳接收当前账户身份和短期应用会话；不返回给前端或插件、不写日志。市场网络使用应用明确配置的出口；关闭代理时不读取环境代理。

## 内部发布流程

1. 用 `eos-plugin pack <directory> <output.eos-plugin>` 打包，`validate` 离线检查。包需根清单、README、LICENSE 和实际模块入口；不执行安装脚本。
2. 管理后台上传每个目标平台的固定制品。发布者须已批准；同一 ID/version 不可覆盖，所有平台清单字节必须一致。
3. 管理员检查清单、执行位置、权限、配置 Schema、固定依赖、来源与制品后审核。当前只有受信内部管理员能上传和审核；不接受外部自助上传。
4. 发布后获得固定签名目录；下架停止新解析但保留已有有效运行，安全撤销则进入撤销策略并阻止下载和运行。
5. 通过内部成员页面分配账户和应用。成员变更立即清除市场会话；客户端在线请求遭拒时停用本地插件。

目录更新使用 revision CAS，发布与审核有审计；系统续签使用专用 system 审计身份。签名目录和运行租约至多 24 小时，在剩余 6 小时内通过已认证请求续签。应用每 4 分钟同步策略；离线不宣称即时撤销，超过租约停止运行。

## 根轮换和回滚

根轮换须满足旧根和新根的阈值签名。将连续的 N.root.json 公开链与已批准的新 root.json 一起提供给签名设施；客户端使用 tough 校验完整链。不要省略中间根或复用测试根。目录和 TUF 版本不可回退；修复代码或恢复历史插件版本应生成更高版本的签名元数据。

当前仓库的 init-dev 命令仅生成隔离测试信任设施，使用 2/3 根阈值和分角色在线密钥。它不会在服务启动时执行，也不能用于生产密钥初始化。

## 旧数据和验收

先执行 `eos-plugin inventory <explicit-legacy-root>`；它只报告清单相对路径、摘要和格式，不读取或输出凭据、不执行代码、不转换和删除数据。发现任何旧包都必须单独审查和迁移，不能自动登记为官方来源。旧 Agent source/name 安装 RPC 和后台旧格式的上传/发布/启用/删除入口已关闭；管理员仅保留历史清单的只读盘点入口；后台历史 SQL 与旧包保留供显式转换和回滚调查。

上线前使用隔离测试环境验证完整服务包、管理页面、固定公开根与实际客户端；保留 base/pro/Core/客户端 SHA、制品哈希和数据库迁移清单。当前本地验收不是生产部署或日常客户端安装证据；生产推广、外部开放和客户端发版分别处理。
