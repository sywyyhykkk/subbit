# Subbit

轻量的 macOS / Windows 个人数字资产订阅管理工具。Rust + Tauri 2 + Svelte 5，无需账号或自建服务器。

- 菜单栏 / 系统托盘随手查看，独立窗口管理。
- 按小时、月、年折算账单花销和报销后的实际花销。
- 到期日期可选，按日历周期自动推进；关闭续期后，到期次日退出当前费用统计。
- 支持买断软件，按购买日起经过的天数均摊，时间越长平均成本越低。
- 支持 Apple 商店、谷歌商店、银行卡、支付宝、微信、PayPal、加密货币等付款方式。
- 36 个服务、8 个分类、自定义名称 / 套餐 / 图标。
- 26 种法币 + USDT、USDC、BTC、ETH、SOL，缓存汇率及固定换算率。
- 全额 / 固定金额 / 百分比报销，中英文、深浅色、等级和登录时启动。
- 数据放在系统文档目录的 `Subbit/subscriptions.json`，支持改位置、完整导入导出，每次修改保留 `.bak`。
- 本地 stdio MCP，可供 Agent 查询目录、预览导入、批量导入和汇总成本。

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
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo build --locked --manifest-path src-tauri/Cargo.toml
python3 scripts/smoke-mcp.py
npm run tauri build
```

## 费用口径

年成本：每 N 月付费为金额 × 12 ÷ N；每 N 年付费为金额 ÷ N。月成本为年成本 ÷ 12，小时成本为年成本 ÷ 8,760。

买断按包含购买当日在内的日历天数均摊：金额 ÷ 已拥有天数，最少一天；再换算为月或年成本。这表示平均成本，不是银行卡流水，也不代表还会发生扣费。报销使用相同币种和周期，计算时直接抵扣，不追踪到账。

月末续期保持原始日期锚点，如 1 月 31 日 → 2 月 28 日 → 3 月 31 日；闰年同理。到期当天仍有效。不填写到期日期时，从开始日期推算下一个周期。原始锚点保存在 JSON 中，展示的下次日期随当前时间计算。

自动汇率在打开应用时按缓存时效检查，后台不轮询。断网继续用缓存；没有汇率时显示待补齐，不按零或 1:1 计入。固定汇率单位为「1 原币 = X USD」。币种与支付方式分开：美元定价、用 USDT 支付时，计价币种仍应选 USD。

## 安装包与扩展

安装包位于 `src-tauri/target/release/bundle/`。macOS 开发包未经过 Apple 开发者签名和公证。Windows 使用系统 WebView2，缺少时由安装程序下载，不捆绑离线浏览器。应用图标暂用占位图标。

[完整开发说明与存储约定](README.md) · [MCP 接入](docs/MCP.md) · [增加服务 / 分类](catalog/CONTRIBUTING.md) · [MIT License](LICENSE)
