# AlphaPulse-OKX

AlphaPulse-OKX is a local OKX USDT perpetual futures radar for market scanning, signal ranking, macro context, and paper-trade review.

AlphaPulse-OKX 是一个本地运行的 OKX USDT 永续合约雷达，用于行情扫描、机会排序、宏观周期观察和纸面交易复盘。

- **Current version / 当前版本:** `v0.1.3 Optimized front end`
- **Runtime / 运行方式:** local only / 仅本地运行
- **Trading mode / 交易模式:** paper trading only / 仅纸面交易

## Overview / 项目概览

AlphaPulse-OKX connects to OKX public market data, builds a real-time radar for USDT perpetual contracts, and presents short-term signals together with BTC macro-cycle context. The app is designed for observation, review, and decision support. It does not connect to a real OKX account and it does not place exchange orders.

AlphaPulse-OKX 通过 OKX 公开行情数据构建本地实时雷达，覆盖 USDT 永续合约的短周期信号、排序、K 线/FVG 图表和 BTC 大周期背景。它面向观察、复盘和决策辅助，不连接真实 OKX 账户，也不会向交易所下单。

## Highlights / 功能亮点

| Area | English | 中文 |
| --- | --- | --- |
| Radar | Real-time terminal-style market radar with filters, status bar, dense table, selected-symbol detail, and TradingView entry. | 实时终端风格雷达，包含过滤器、状态栏、密集行情表、选中合约详情和 TradingView 入口。 |
| Signals | Tracks short-term price change, trend score, chop score, FVG zones, support/resistance, and trigger reasons. | 跟踪短周期涨跌、趋势分、震荡分、FVG 区域、支撑/阻力和触发原因。 |
| Macro | BTC macro summary on Radar, plus a full Macro page for cycle, valuation, and AHR999 context. | Radar 页展示 BTC 大周期摘要，并提供完整 Macro 页用于周期、估值和 AHR999 分析。 |
| Strategy | Strategy workspace for signal attribution, pattern review, shadow positions, and active paper positions. | Strategy 工作台用于信号归因、形态复盘、影子仓位和纸面持仓观察。 |
| Paper trade | Local paper trading with orders, positions, equity, realized PnL, unrealized PnL, fees, and position history. | 本地纸面交易，支持订单、持仓、权益、已实现/未实现盈亏、手续费和历史仓位。 |
| Review | Review page for equity curve, trade history, performance metrics, and strategy-version comparison. | Review 页用于权益曲线、交易历史、绩效指标和策略版本对比。 |
| UI | Light, Dark, and System themes, plus English/Chinese switching through compact controls. | 支持 Light、Dark、System 主题，以及紧凑的中英文切换控件。 |

## Pages / 页面

| Page | Purpose | 说明 |
| --- | --- | --- |
| Monitor / Radar | Main real-time radar console for scanning, sorting, filtering, charting, and symbol detail. | 主雷达控制台，用于扫描、排序、过滤、看图和查看合约详情。 |
| Macro | BTC cycle, valuation, AHR999, and macro permission context. | BTC 周期、估值、AHR999 和宏观许可状态。 |
| Strategy | Signal attribution, pattern statistics, strategy insights, and active/shadow positions. | 信号归因、形态统计、策略洞察、真实纸面仓位和影子仓位。 |
| Trade | Paper order entry, open positions, and local account state. | 纸面下单、持仓管理和本地账户状态。 |
| Review | Paper-trade performance, closed positions, equity/PnL curve, and strategy comparison. | 纸面交易表现、已平仓记录、权益/盈亏曲线和策略对比。 |

## What It Does Not Do / 边界说明

- It does not read an OKX account. / 不读取 OKX 账户。
- It does not require an OKX API key. / 不需要 OKX API Key。
- It does not place, close, or automate real exchange orders. / 不会真实开仓、平仓或自动交易。
- It is not financial advice. Signals are for observation and review only. / 不提供投资建议，所有信号仅用于辅助观察和复盘。
- Paper-trading state is local runtime state, not an exchange-side account. / 纸面交易状态是本地运行态，不是交易所账户状态。

## Tech Stack / 技术栈

| Layer | Stack |
| --- | --- |
| Backend | Rust, Axum, Tokio, OKX public REST/WebSocket market data |
| Frontend | React, TypeScript, Vite, Lightweight Charts, Recharts, Tailwind CSS, lucide-react |
| Runtime | Backend on `127.0.0.1:8787`, frontend on `127.0.0.1:5173` |
| Data | OKX public market data, optional Coinglass macro valuation data |

## Quick Start / 快速开始

Requirements / 依赖：

- Rust stable toolchain / Rust stable 工具链
- Node.js current LTS and npm / Node.js 当前 LTS 与 npm
- Network access to OKX public market data / 可访问 OKX 公开行情数据

Optional macro valuation data uses `COINGLASS_API_KEY`. The app can run without it, but some external valuation metrics may be unavailable.

宏观估值数据可选使用 `COINGLASS_API_KEY`。不配置也可以运行，只是部分外部估值指标可能不可用。

```bash
cp .env.example .env.local
```

Start the backend / 启动后端：

```bash
cargo run -p alphapulse_okx_backend
```

Start the frontend / 启动前端：

```bash
cd frontend
npm ci
npm run dev
```

Open / 打开：

```text
http://127.0.0.1:5173
```

## API / 本地接口

The backend listens on `http://127.0.0.1:8787`.

后端默认监听 `http://127.0.0.1:8787`。

| Method | Endpoint | Purpose / 用途 |
| --- | --- | --- |
| `GET` | `/api/health` | Health check / 健康检查 |
| `GET` | `/api/snapshot` | Radar snapshot / 雷达快照 |
| `GET` | `/api/macro/btc` | BTC macro data / BTC 宏观数据 |
| `GET` | `/api/symbols/:inst_id/chart?timeframe=m15&limit=180&filled=true` | Symbol chart candles and FVG data / 合约 K 线与 FVG 数据 |
| `GET` | `/api/paper` | Paper account state / 纸面账户状态 |
| `POST` | `/api/paper/orders` | Submit a paper order / 提交纸面订单 |
| `POST` | `/api/paper/positions/:inst_id/close` | Close a paper position / 平纸面仓位 |
| `GET` | `/ws` | Realtime WebSocket stream / 实时 WebSocket 数据流 |

## Verification / 验证

Backend / 后端：

```bash
cargo test -p alphapulse_okx_backend
cargo check -p alphapulse_okx_backend
```

Frontend / 前端：

```bash
cd frontend
npm run lint
npm test
npm run build
```

## Project Layout / 目录结构

```text
.
├── backend/              # Rust backend and OKX market-data scanner
├── frontend/             # React/Vite frontend app
├── docs/superpowers/     # Planning and implementation notes
├── .env.example          # Optional local environment example
├── Cargo.toml            # Rust workspace
└── README.md
```

## Release Notes / 版本说明

### `v0.1.3 Optimized front end`

- Reworked the radar into a compact terminal-style interface aligned with the Radar color system.
- Added denser controls for theme and language switching.
- Improved market metric layout so price/change values stay inside their panels.
- Added strategy and review surfaces for signal analysis and paper-trade review.
- Updated documentation into a bilingual GitHub-style README.

### 中文说明

- 将 Radar 优化为更紧凑的终端风格界面，并统一颜色风格。
- 将主题和语言切换改为更紧凑的控制方式。
- 修复 Market 指标区域的字体和数值溢出问题。
- 增加 Strategy 和 Review 相关能力，用于信号分析和纸面交易复盘。
- 将 README 更新为中英文双语 GitHub 项目说明。

## License / 许可证

No license file is currently included. Add a license before distributing or reusing this project outside the repository owner's intended scope.

当前仓库尚未包含 LICENSE 文件。若要对外分发或复用，请先补充许可证。
