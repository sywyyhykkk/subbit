# Subbit

[English](README.md) | [简体中文](README.zh-CN.md)

轻量的 macOS / Windows 个人数字资产订阅管理工具。Rust + Tauri 2 + Svelte 5，无需账号或自建服务器。

## 下载与安装

下载 [Subbit 0.0.6 正式版](https://github.com/sywyyhykkk/subbit/releases/tag/v0.0.6)：

- [macOS Apple Silicon DMG](https://github.com/sywyyhykkk/subbit/releases/download/v0.0.6/Subbit_0.0.6_macos-arm64.dmg)：macOS 12+，Apple M 系列芯片。
- [macOS Intel DMG](https://github.com/sywyyhykkk/subbit/releases/download/v0.0.6/Subbit_0.0.6_macos-x86_64.dmg)：macOS 12+，Intel 处理器。
- [Windows x64 安装 EXE](https://github.com/sywyyhykkk/subbit/releases/download/v0.0.6/Subbit_0.0.6_windows-x64-setup.exe)：支持 Windows 10/11、Intel/AMD 64 位。运行安装程序，再从开始菜单启动。

Mac 可在苹果菜单 → 关于本机中查看芯片类型，选择对应的 DMG，打开后将 Subbit 拖入 Applications。

无需开发环境。Windows 缺少 WebView2 时会联网安装；请下载安装包，而不是 GitHub 自动生成的 Source code 源码压缩包。此版本尚无发布者签名及 Apple 公证，首次启动可能出现 Gatekeeper 或 SmartScreen 提示。安装条件和验证范围见 [发布说明](docs/releases/0.0.6.md)。

0.0.2–0.0.5 可在 **设置 → 关于 → 检查更新** 中升级至 0.0.6；从 0.0.1 升级需手动安装一次。


## 功能

- 菜单栏 / 系统托盘随手查看，独立窗口管理。
- 按小时、天、月、年折算账单花销和报销后的实际花销。
- 到期日期可选，按日历周期自动推进；关闭续期后，到期次日退出当前费用统计。
- 支持买断软件，按购买日起经过的天数均摊，时间越长平均成本越低。
- 支持 Apple 商店、谷歌商店、银行卡、支付宝、微信、PayPal、加密货币等付款方式。
- 57 个服务、12 个分类，包含设计创作工具 Spine，以及 AWS、GCP、阿里云、Vultr、DigitalOcean、Hetzner、腾讯云、华为云、Azure、OVHcloud 云服务器预设；支持自定义名称 / 套餐 / 图标。
- 已核实固定价格的套餐自动带出美元参考价，按所选币种和固定 / 缓存汇率换算；月付、年付分别匹配，支持手动改价，不覆盖已保存账单。按量计费、询价或尚未核实价格的套餐手动填写，地区价格及税费以实际账单为准。
- 26 种法币 + USDT、USDC、BTC、ETH、SOL，缓存汇率及固定换算率。
- 全额 / 固定金额 / 百分比报销，中英文、深浅色、等级和登录时启动。
- 数据放在系统文档目录的 `Subbit/subscriptions.json`，支持改位置、完整导入导出，每次修改保留 `.bak`。
- 本地 stdio MCP，可供 Agent 查询目录、预览导入、批量导入和汇总成本。

## 花销报告与备份（0.0.5）

设置右侧的统计按钮可查看分类花销、主要资产及订阅 / 买断构成，并导出简洁 PNG 报告。统计涵盖全部有效资产，展示当前折算费用，不代表历史付款流水。列表可按账单花销或实际花销排序，并记住选择。

资产备份最多保留 3 份：最近 2 份普通备份和 1 份导入前备份。在「设置 → 数据」预览时间与资产数量后恢复。普通保存和恢复不覆盖导入前恢复点，下一次有效导入才替换它。相同导入请求重试不改数据、版本号或备份。

## 域名与离线模式（0.0.4）

新增资产时选择「域名」，一条记录对应一个域名，独立填写管理商、费用和到期日。默认年付，支持多年周期；搜索同时匹配域名和管理商。已有资产无需迁移，未填写的管理商字段不会写入；含新字段的数据需要 0.0.4 或更高版本读取。

输入域名后，Subbit 直接尝试获取该网站的 favicon，不使用第三方图标聚合服务。成功后压缩并随资产保存；失败时显示地球图标，离线时使用已保存图标。手动设置的图标不会被覆盖，获取图标也不会阻塞保存。

在「设置 → 通用」开启「离线模式」，确认影响后生效。默认关闭；开启后停止汇率、目录、网站图标、应用更新和从 Subbit 打开外部网络链接，并取消进行中的汇率、目录与图标请求。应用正在检查或下载安装更新时，需等待该操作结束再切换。开关独立保存在应用配置中，导入旧备份或切换数据文件不会将其关闭。

固定汇率优先，其次使用已有缓存；缺失的法币报价使用安装包内置的 [Frankfurter 快照](https://api.frankfurter.dev/v2/rates?base=USD)（2026-10-16 / 17），加密货币缺报价时仍需补齐或设置固定汇率。目录优先使用有效本地缓存，再回退到内置目录。离线数据可能过时，但资产编辑、费用计算、备份导出及本地 MCP 继续可用。外部 Agent 是否联网由其自身控制；离线运行不包含 Windows 缺少 WebView2 时的离线安装。

Agent 可通过 MCP 预览并导入汇率和完整目录，接口接收数据本身，不让 Subbit 下载 URL。见 [MCP 导入说明](docs/MCP.md#offline-rate-and-catalog-imports)。大小面板都支持支付方式筛选和每日成本，并记住筛选、排序偏好。

## 开发

先安装 Node.js 24+、Rust，以及 [Tauri 系统依赖](https://v2.tauri.app/start/prerequisites/)。

```sh
npm ci
npm run tauri dev
```

仅运行 `npm run dev` 是浏览器界面预览；本地文件与系统托盘等功能需要桌面应用。

```sh
npm run check
npm run catalog:check
npm run test:pricing
npm run test:dates
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml
python3 scripts/smoke-mcp.py
npm run tauri build -- --config '{"bundle":{"createUpdaterArtifacts":false}}'
```

## 费用口径

年成本：每 N 月付费为金额 × 12 ÷ N；每 N 年付费为金额 ÷ N。日成本为年成本 ÷ 365，月成本为年成本 ÷ 12，小时成本为年成本 ÷ 8,760。

买断按包含购买当日在内的日历天数均摊：金额 ÷ 已拥有天数，最少一天；再换算为月或年成本。这表示平均成本，不是银行卡流水，也不代表还会发生扣费。报销使用相同币种和周期，计算时直接抵扣，不追踪到账。

月末续期保持原始日期锚点，如 1 月 31 日 → 2 月 28 日 → 3 月 31 日；闰年同理。到期当天仍有效。不填写到期日期时，从开始日期推算下一个周期。原始锚点保存在 JSON 中，展示的下次日期随当前时间计算。

自动汇率在打开应用时按缓存时效检查，后台不轮询。断网继续用缓存；没有汇率时显示待补齐，不按零或 1:1 计入。固定汇率单位为「1 原币 = X USD」。币种与支付方式分开：美元定价、用 USDT 支付时，计价币种仍应选 USD。

## 给 Agent 使用：MCP 接入

Subbit 内置本地 MCP 服务，让支持 MCP 的 Agent 读取订阅、汇总成本，或根据你提供的账单批量导入记录。配置完成后，Agent 客户端会自动执行 Subbit 可执行文件并传入 `--mcp` 参数来启动服务，**无需手动在终端运行命令**。服务使用 stdio 通信，与桌面应用共享本地数据，不打开窗口或网络端口。

1. 将 Subbit 安装或移动到最终位置，再打开应用。
2. 进入 **设置 → Agent / MCP**，开启 MCP 并保存。
3. 复制应用生成的配置，填入 Agent 客户端的 MCP 配置。配置包含当前可执行文件的真实路径和 `args: ["--mcp"]`；如果客户端使用不同格式，按其要求转换。Windows 使用 `subbit.exe` 路径。
4. 在 Agent 客户端重新连接，客户端会按配置自动启动 MCP 服务。随后直接用自然语言让 Agent 查询；导入前，先让 Agent 读取预设目录和已有订阅。

| 工具 | Agent 可以做什么 |
| --- | --- |
| `get_catalog` | 查询服务、套餐、已核实的美元参考价、币种和记录格式 |
| `list_subscriptions` | 读取已保存订阅及数据版本 |
| `get_summary` | 按指定币种汇总每小时、每天、每月或每年的成本 |
| `preview_import` | 写入前校验待导入记录，报告重复项 |
| `import_subscriptions` | 批量追加记录，通过固定 `importId` 避免重试时重复导入 |
| `get_exchange_rates` | 读取本地汇率、固定汇率和数据版本 |
| `preview_exchange_rates` / `import_exchange_rates` | 本地校验、预览并导入汇率 |
| `preview_catalog` / `import_catalog` | 本地校验、预览并导入完整目录 |

可以对 Agent 说：“用 Subbit 帮我汇总每月订阅成本，以人民币显示。”或“根据这份账单生成 Subbit 记录，先展示导入预览，再导入。”金额应来自实际账单或你明确选用的参考价；MCP 不会自动补齐缺失字段，也不会主动获取汇率。Subbit 本身在本地保存数据，但返回给云端 Agent 的记录会由该 Agent 处理。

详细配置与导入示例见 [MCP 接入文档](docs/MCP.md)，完整字段见 [订阅记录 JSON Schema](catalog/subscription.schema.json)。

## 如何贡献预设

预设同时供桌面应用和 MCP 使用。新增服务或补全套餐时：

1. 在 `catalog/services/` 复制一个类似的 JSON 文件。使用与文件名一致的稳定小写短横线 ID，填写官方名称、搜索别名、分类、官方来源链接、核对日期和地区信息。
2. 为套餐设置稳定 ID，使用 `kind: "recurring"` 表示周期订阅、`"lifetime"` 表示买断。下架套餐标记 `retired: true`，不要直接删除。
3. 有可靠固定价格时，填写美元基准 `prices`、来源 `priceSource` 和核对日期 `priceReviewedAt`。年付填写全年总额，买断不填写周期；按席位收费需标记。按量计费、询价或未核实价格留空。
4. 在 `catalog/icons/` 添加 SVG 或 PNG，并记录图标来源和许可。确需新分类时，在 `catalog/categories.json` 同时提供中英文名称。
5. 运行 `npm run catalog:check`、`npm run test:pricing` 和 `npm run check`，然后提交 Pull Request，附上预设文件、图标及官方来源链接。

完整字段规则见 [预设贡献指南](catalog/CONTRIBUTING.md)。目录更新不会覆盖已有记录中的名称、套餐和金额。如果只供自己使用，直接在应用中选择“自定义服务”即可。

目录 PR 合并到 `main` 后，GitHub Actions 会独立发布分类、服务、套餐和图标，无需发布应用新版。支持此功能的应用和 MCP 会获取并缓存目录；离线或更新失败时继续使用本地缓存，没有缓存时使用内置目录。也可在「设置 → 数据 → 更新目录」手动刷新。目录缓存与资产文件分开保存，不修改已有资产、设置和汇率。旧版应用需先升级一次以获得此功能。

## 安装包

Tag 发布流程通过 GitHub Actions 的 `TAURI_SIGNING_PRIVATE_KEY` Secret 签署更新包并发布 `latest.json`。签名私钥应在仓库外独立备份。

安装包位于 `src-tauri/target/release/bundle/`。macOS 开发包未经过 Apple 开发者签名和公证。Windows 使用系统 WebView2，缺少时由安装程序下载，不捆绑离线浏览器。应用图标使用 Subbit 专属「账单兔」。

[完整开发说明与存储约定](README.md) · [MCP 接入](docs/MCP.md) · [增加服务 / 分类](catalog/CONTRIBUTING.md) · [MIT License](LICENSE)

开发与发布：[Agent 开发约定](AGENTS.md) · [开发经验](docs/ENGINEERING_LESSONS.md) · [发版流程与校验](docs/RELEASING.md)。
