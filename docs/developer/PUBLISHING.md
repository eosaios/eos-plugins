# 上传、审核与发布

作者准备包与验收材料；获准上传的账号提交；管理员审核并签名发布。作者不需要平台签名私钥，也不能用包内自带公钥让正式客户端信任包。

## 1. 首次发布前确认

- 维护者已批准 publisher 命名空间，确认你有权使用该身份；当前没有自助注册/成员授权页面或注册 API。上传服务检查发布者是否 approved，不能将白名单身份当作已具备逐发布者归属校验的证明；管理员必须人工核对来源和命名空间。
- 当前账号 active，邮箱在目标环境 `EOS_PLUGIN_MARKET_UPLOAD_EMAILS`。管理员也要满足上传白名单。
- 明确是测试还是正式市场；账号、数据、签名根和应用环境一致。网页市场入口由维护者提供；正式独立市场的入口配置见仓库 README，但本手册不证明当前服务器已部署到哪一版。
- 已按[调试文档](DEBUGGING.md)完成该阶段验收。首次 Agent 安装先在隔离测试市场审核发布后验证，不能用生产市场测试随机包。

## 2. 冻结发布目录与制品

更新 plugin.version；pack/validate；保留原包，不在上传后重新构建来替换。示例命令：

```sh
mkdir -p ../plugin-artifacts
cargo run --locked -p eos-plugin-cli -- pack examples/hello-skills ../plugin-artifacts/hello-skills-0.1.0.eos-plugin
cargo run --locked -p eos-plugin-cli -- validate ../plugin-artifacts/hello-skills-0.1.0.eos-plugin
python3 - <<'PY'
import hashlib
from pathlib import Path
p = Path('../plugin-artifacts/hello-skills-0.1.0.eos-plugin')
print(hashlib.sha256(p.read_bytes()).hexdigest(), p.name)
PY
```

示例 tutorial 身份仅本地使用，上传前改成获批 ID。一个上传最多 8 个制品，HTTP multipart 请求总上限 200 MiB（含封装开销，实际文件总量应略小于此值）；每个包仍受运行库 200 MiB 压缩上限约束。

同一 ID/version 不可覆盖，**已 validated 的未发布版本也占用版本号**。多个平台一次上传全部目标，plugin.toml 原始字节必须一致。每个 package 字段与 targets 数组同位置配对。目标写法见[制品目标](MANIFEST.md)。

## 3. 网页上传

1. 登录维护者提供的目标环境市场。
2. 有上传资格时使用上传入口，选择 `.eos-plugin` 文件。
3. 为每个文件选择 OS/架构，填写 ABI JSON 数组；纯 Skills 为 `["skills/v1@1.0.0"]`。原生包使用实际编译目标；不要标通用。
4. 提交并保存返回 ID、version、manifestDigest、每个 artifact.digest、revision 与状态。
5. 成功时为 `validated`（待审核），不会立刻出现在公开目录。

管理后台的上传页也可以使用，但当前后台上传 UI 不提供 ABI 编辑字段，默认 abis=[]。若要明确填写 ABI，用独立市场上传页或下列 API。不要因为“后台上传成功”便认为目标 ABI 已自动识别。

## 4. 上传 API

这些是当前路由，均相对于**账号 API origin**，不是独立市场静态网页地址。成功响应为 `{"data":...}`，失败响应为 `{"error":{"code","message","details","retryable"}}`。

| 方法和路径 | 身份 | 内容 |
| --- | --- | --- |
| GET `/api/v1/plugin-market/upload-access` | 账号 Bearer | data 为 bool |
| POST `/api/v1/plugin-market/uploads` | 账号 Bearer + 上传白名单 | multipart package（可重复）与 targets JSON |
| GET `/api/v1/admin/plugin-market` | 管理员 | data.releases 为记录数组，每项含 release |
| POST `/api/v1/admin/plugin-market/uploads` | 管理员入口且上传白名单 | 与普通上传相同 |
| POST `/api/v1/admin/plugin-market/releases/{id}/{version}/review` | 管理员 | JSON `{revision}` |
| POST `.../{id}/{version}/publish` | 管理员 | JSON `{revision}`，签名可能耗时 |
| POST `.../{id}/{version}/withdraw` | 管理员 | 普通下架，JSON `{revision}` |
| POST `.../{id}/{version}/revoke` | 管理员 | 安全撤销，JSON `{revision}` |

以下为明确指向测试市场的命令模板；API origin 由维护者确认。账号令牌应通过既有登录流程取得、放在仅当前用户可读的 header 文件，不打印、不提交。不要把网页 localStorage 中的令牌复制到共享日志。

```sh
# headers.txt 内容为 Authorization: Bearer <当前测试账号令牌>
# 先由维护者安全创建这个文件，且保持文件权限仅当前用户可读。
export EOS_PLUGIN_API_ORIGIN=https://api-test.eosaios.com
curl --fail-with-body --silent --show-error \
  --header @../plugin-artifacts/headers.txt \
  "$EOS_PLUGIN_API_ORIGIN/api/v1/plugin-market/upload-access"

curl --fail-with-body --silent --show-error \
  --header @../plugin-artifacts/headers.txt \
  --form 'package=@../plugin-artifacts/hello-skills-0.1.0.eos-plugin' \
  --form 'targets=[{"os":"any","arch":"any","abis":["skills/v1@1.0.0"]}]' \
  "$EOS_PLUGIN_API_ORIGIN/api/v1/plugin-market/uploads" \
  --output ../plugin-artifacts/upload-receipt.json
```

上传/管理 API 不要求 X-Plugin-Market-Session；不要把应用运行会话混成上传权限。网络中断后先查是否已经创建记录，避免盲目重复上传；重复版本应返回 RESOURCE_CONFLICT。修正包需要新版本。

## 5. 管理员审核与签名

管理后台的“插件市场”能查看清单、权限、固定制品和摘要。审核不是自动安全扫描：当前实现执行格式校验与人工审核，未集成通用恶意代码扫描/SBOM 构建或发布者逐用户成员校验，审查者需检查作者归属、原生程序、网络端点、许可证、固定依赖和验收证据。

当前状态转换：

```text
上传并校验 -> validated -> review -> approved -> publish -> published
published -> withdraw -> withdrawn -> publish -> published（相同字节）
approved / published / withdrawn -> revoke -> revoked（终止，不可直接恢复）
```

当前没有创建 draft、单独 validate、reject、删除版本、修改频道或修改 audience 的作者接口。不要照旧 SPEC 的目标路由调用这些操作。

后台按钮会带当前 revision。用 API 时先 GET 管理目录读取 release.revision，再 review，使用 review 返回的新 revision 执行 publish。例如（id/version 需替换成实际值，revision 需取实时返回值）：

```sh
curl --fail-with-body --silent --show-error \
  --header @../plugin-artifacts/admin-headers.txt \
  --header 'Content-Type: application/json' \
  --data '{"revision":1}' \
  "$EOS_PLUGIN_API_ORIGIN/api/v1/admin/plugin-market/releases/acme.summary/0.1.0/review"

curl --fail-with-body --silent --show-error --max-time 120 \
  --header @../plugin-artifacts/admin-headers.txt \
  --header 'Content-Type: application/json' \
  --data '{"revision":2}' \
  "$EOS_PLUGIN_API_ORIGIN/api/v1/admin/plugin-market/releases/acme.summary/0.1.0/publish"
```

这是管理员操作模板，不用于作者自行升级权限。RESOURCE_CONFLICT 时刷新记录后重新评估；不要重试旧 revision。签名使用配置好的独立工作进程及平台信任链，CLI sign 不是作者发布 API。

## 6. 发布后验证

1. 查看 GET `/api/v1/plugin-market/plugins/{id}` 或公开列表，确认版本/摘要；普通公开目录只显示 published。
2. 同环境 Agent 搜索、准备、检查权限/配置、确认安装、实际调用。网页下载不等于安装，下载本地文件不执行插件。
3. 网页“在 EOS Agent 中安装”会带固定版本和摘要；宿主重新核实计划。它不会把测试包自动切到生产环境。
4. 保留发布回执、哈希、目标、宿主构建与验收结果。生产发布另行取得该次范围授权，不能因编写了本文或完成测试而自动执行。

正常升级发新 SemVer，经相同流程。若仅包回退，旧版本必须仍可信、授权有效、数据可恢复；当前没有作者频道指针 API。数据版本降级会拒绝，不能承诺总能一键回滚。

普通下架停止新解析，不立即杀已有租约；安全撤销在在线策略同步/离线租约到期后阻止激活并关闭受管实例。已运行受信进程或远端服务产生的外部副作用不能撤回。修复撤销版本需新版本、新审核与签名。

## 安装侧接口，供环境维护者核对

安装由 Agent 处理，作者通常不手工调用这些 API：

| 方法和路径 | 说明 |
| --- | --- |
| GET `/api/v1/plugin-market/catalog` | 匿名已发布原始目录 |
| GET `/api/v1/plugin-market/plugins` | 匿名网页列表；GET `plugins/{id}` 为详情 |
| POST `/api/v1/plugin-market/search` | 匿名按 HostDescriptor 搜索/兼容性 |
| POST `/api/v1/plugin-market/sessions` | 账号 Bearer + `{applicationId}`，创建 5 分钟应用会话 |
| POST `/api/v1/plugin-market/resolve` | 账号 Bearer + X-Plugin-Market-Session；固定候选发布与依赖 |
| GET `/api/v1/plugin-market/trust-bundle?manifestDigest=...` | 同上；TUF 元数据与签名目标集合 |
| GET `/api/v1/plugin-market/policy` | 同上；运行策略 |
| POST `/api/v1/plugin-market/download-authorizations` | 同上 + `{digest}`，短期下载许可 |
| GET `/api/v1/plugin-market/artifacts/{digest}` | 同上 + X-Plugin-Download-Token；受控字节下载 |

不要用旧的 `/trust/{roleMetadata}`、`/plugins/{id}/releases`、`/publishers` 或 `/channels` 作为现有接口。下载、验签、计划摘要确认和本地事务由统一管理器负责；作者不应自己拼装正式安装记录。
