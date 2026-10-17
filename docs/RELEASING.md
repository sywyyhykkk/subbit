# 发版流程与校验

适用于桌面安装包、应用内更新及独立服务目录的发布。先阅读 [开发约定](../AGENTS.md) 与 [开发经验](ENGINEERING_LESSONS.md)，尤其是签名封装、资源体积和测试隔离。检查的对象必须是本次源码和最终产物。

## 1. 汇总改动与版本

1. 检查工作区、当前分支、远端主分支及尚未发布的提交；保留其他任务的工作。发布分支按用途命名，如 `release/0.0.3`，所有 Agent 均不得以 Agent、模型或 AI 工具名称命名分支前缀。
2. 将本次需要发布的功能与主分支已有更新汇总到发布分支，处理冲突，检查没有丢失已有分类、预设、UI 修复或开发文档。
3. 同步 `package.json`、`package-lock.json` 根版本及 `packages[""].version`、`src-tauri/Cargo.toml`、`src-tauri/Cargo.lock` 中 Subbit 自身版本、`src-tauri/tauri.conf.json`。
4. 新建 `docs/releases/<版本>.md`，补全中英文用户更新说明、升级方式及安装条件；README 的下载链接、版本与目录数量同步。不要将开发过程、原始聊天或个人路径写入公告。
5. 确认签名私钥仍由 `TAURI_SIGNING_PRIVATE_KEY` GitHub Actions Secret 提供，应用内验证公钥未被意外替换；不打印或提交私钥。

## 2. 本地检查

在项目根目录运行以下检查。全新环境先执行 `npm ci`；不要为了发版顺手升级依赖。

```sh
npm run check
npm run catalog:check
npm run test:pricing
npm run test:dates
npm run test:levels
npm run test:reload
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml
python3 scripts/smoke-mcp.py
git diff --check
```

所有可能写入数据的试验必须使用独立 `SUBBIT_CONFIG_DIR`，运行前确认数据与 location 指针均指向隔离目录。`scripts/smoke-mcp.py` 自行创建临时目录与合成记录；不使用真实账单、日常安装实例或真实自动备份。

重点检查本次实际触及的交互：大小面板、中英文及系统语言、深浅色、套餐切换与手动金额、弹窗滚动、外链、保存冲突和 MCP 修改后的刷新。单元测试、浏览器预览、桌面原生行为分别记录，不互相代替。

## 3. PR 与必需 CI

提交并推送本次全部应入库文件，通过 PR 合入 `main`，不要强推主分支或改动保护规则。当前必需检查为：

| 检查 | 平台 | 必须通过的内容 |
| --- | --- | --- |
| `check (macos-14)` | macOS Apple Silicon | 类型、目录、四组前端测试、前端构建、Rust 测试、debug 可执行文件 MCP、release App 构建、App 完整签名检查 |
| `check (windows-latest)` | Windows x64 | 相同的代码 / 测试 / MCP 检查，以及 NSIS 安装包构建 |

实现见 [ci.yml](../.github/workflows/ci.yml)。检查必须对应 PR 最新提交；只看旧提交的绿色状态不算通过。审核遵守当前仓库规则，仓库所有者已有的 PR 审核例外不代表可以跳过必需 CI。

有失败时查看具体 job 日志，修复后等待新提交的检查完成。CI 未通过不得合并、打正式发布标签或宣布发布成功。

## 4. 标签与三平台构建

PR 合并且必需检查通过后，获取最新 `main`，确认版本一致，在预定发布提交上创建 `v<版本>` 标签并推送。不要让本地旧分支或旧标签决定发布内容。

标签触发 [release.yml](../.github/workflows/release.yml)，分别使用 macOS Apple Silicon、macOS Intel 和 Windows x64 runner。每个构建都再次执行本地检查中的代码校验、前端测试、构建与 Rust 测试，并检查标签和版本字段一致。

macOS 使用 `npm run tauri build -- --bundles app,dmg`，两种架构分别出包。Windows 使用 `--bundles nsis`。发布 job 依赖三个构建全部成功，不应手动上传部分产物冒充完整版本。

## 5. 安装包和签名验收

| 产物 | 验收内容 |
| --- | --- |
| macOS DMG | `hdiutil verify`；挂载后检查 App 版本；`lipo -archs` 与文件标注架构一致；包内 MCP 检查 |
| DMG 内 `.app` | `codesign --verify --deep --strict --verbose=2` 通过，并查看签名信息；签名后不改资源 |
| macOS `.app.tar.gz` | 更新签名文件非空；解压后的 App 再次通过完整资源签名检查 |
| Windows EXE | 在 Windows runner 静默安装成功；安装后可执行文件存在；安装版 MCP 检查；原生窗口实际出现并正常退出测试 |
| 更新清单 | `latest.json` 的版本、三个平台键、对应下载 URL 和更新签名完整，所有目标产物可下载 |

必须区分三件事：镜像完整性、App 代码 / 资源签名、Tauri 更新签名。任何一项都不能代替另一项；MCP 能运行也不能证明 GUI 能首次启动。

当前 macOS 使用 ad-hoc 签名，没有 Developer ID 签名和 Apple 公证。完整签名检查通过不等于已获 Apple 信任。首次下载后的信任提示和资源签名损坏要分别诊断，不以关闭 Gatekeeper、删除隔离属性或要求用户重新签名代替修复。

## 6. 公共发布与安装检查

1. 三平台构建成功后，发布 job 汇集所有产物，运行 `scripts/release-manifest.mjs` 生成 `latest.json`，以 `docs/releases/<版本>.md` 创建 GitHub Release。
2. 核对公开 Release 的标签、公告、最新版本状态和附件。必须包括两份 DMG、两份 macOS 更新归档及其签名、Windows EXE 及其签名，以及 `latest.json`。
3. 从公开地址重新下载本次附件，检查实际大小、版本和签名。不能只核对 runner 内或本地已有的旧包；有异常先修复产物或发布链路。
4. 真实安装验收另行覆盖：浏览器下载（保留正常隔离信息）、安装到 Applications、首次启动、托盘 / 主窗口、外链及设置。应用内更新应在独立测试安装和合成数据上执行，确认版本更新和资产、设置、导入回执保留；不覆盖日常安装版来试验。
5. 比较同平台同架构的新旧安装包和 App 大小。资源或依赖造成明显增长时先查明，不宣称代码优化等于测得内存下降。
6. 尚未实际执行的架构、系统版本、浏览器首次启动或更新路径必须明确记为未验证；不要把 CI 构建成功扩大成所有设备可直接安装。

同版本重建仅在明确要求时进行；替换安装包与更新归档后同步清单和签名，并从公开链接重新验证。版本和公告是否保持不变，遵循本次任务要求。

## 7. 独立目录发布

分类、服务、套餐与品牌图标由 [catalog.yml](../.github/workflows/catalog.yml) 独立发布。相关变更合并到 `main` 后，工作流校验目录、生成单个包含图标的 `catalog.json`，发布至 `catalog` 分支。

首次部署或失败重试可手动触发该工作流。检查原始下载地址返回完整目录、分类 / 服务数量及图标数据；已有支持目录更新的应用应在失败时保留原缓存，离线仍可用。目录发布不得重写用户资产、设置或汇率。新增此能力本身仍需要先发布一次应用版本。

## 8. 发布完成记录

完成后记录发布标签及提交、PR、必需 CI 与三平台发布运行链接、公开附件和更新清单核验结果。区分自动检查、实际安装 / 更新和仍未验证的场景；确认应入库改动已提交推送，忽略的本地辅助产物继续保持忽略。
