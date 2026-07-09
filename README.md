# AlphaPulse-OKX

AlphaPulse-OKX is a local OKX USDT perpetual futures radar for market scanning, signal ranking, macro context, and paper-trade review.

AlphaPulse-OKX 是一个本地运行的 OKX USDT 永续合约雷达，用于行情扫描、机会排序、宏观周期观察和纸面交易复盘。

- **Current version / 当前版本:** `v0.1.4 preview`
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
- Paper-trading state is local simulated state, not an exchange-side account. When `DATABASE_URL` is configured it is persisted to PostgreSQL and cached through Redis. / 纸面交易状态是本地模拟状态，不是交易所账户状态。配置 `DATABASE_URL` 后会持久化到 PostgreSQL，并通过 Redis 缓存实时快照。

## Tech Stack / 技术栈

| Layer | Stack |
| --- | --- |
| Backend | Rust, Tokio, Axum, serde/serde_json, sqlx, PostgreSQL, Redis, tracing, rust_decimal, chrono |
| Frontend | React, TypeScript, Vite, Lightweight Charts, Recharts, Tailwind CSS, lucide-react |
| Runtime | Backend on `127.0.0.1:8787`, frontend on `127.0.0.1:5173`, optional PostgreSQL on `5432`, optional Redis on `6379` |
| Data | OKX public market data, PostgreSQL persistence for strategy/paper state, Redis live snapshot cache, optional Coinglass macro valuation data |

## Quick Start / 快速开始

Requirements / 依赖：

- Rust stable toolchain / Rust stable 工具链
- Node.js current LTS and npm / Node.js 当前 LTS 与 npm
- Docker Desktop or Docker Engine for local PostgreSQL/Redis / 本地 PostgreSQL/Redis 建议使用 Docker
- Network access to OKX public market data / 可访问 OKX 公开行情数据

Optional macro valuation data uses `COINGLASS_API_KEY`. The app can run without it, but some external valuation metrics may be unavailable.

宏观估值数据可选使用 `COINGLASS_API_KEY`。不配置也可以运行，只是部分外部估值指标可能不可用。

```bash
cp .env.example .env.local
docker compose up -d postgres redis
```

The backend runs without `DATABASE_URL` and `REDIS_URL`, but that is an in-memory fallback. For paper trading history, keep PostgreSQL enabled. Set `ALPHAPULSE_REQUIRE_DATABASE=true` when you want startup to fail instead of silently falling back to memory.

后端在不配置 `DATABASE_URL` 和 `REDIS_URL` 时仍可运行，但这是内存降级模式。需要保留纸面交易历史时应启用 PostgreSQL。设置 `ALPHAPULSE_REQUIRE_DATABASE=true` 后，如果数据库不可用，后端会启动失败而不是静默降级。

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

## Production Deploy / 生产部署

The app can be deployed as a single-server package:

- systemd runs the Rust backend on `127.0.0.1:8787`.
- Nginx serves the built frontend and proxies `/api/*` plus `/ws` to the backend.
- The GitHub Actions workflow `.github/workflows/deploy.yml` builds, packages, and deploys the release through SSH when manually triggered.

Required repository secrets:

- `DEPLOY_HOST`
- `DEPLOY_PORT`
- `DEPLOY_USER`
- `DEPLOY_SSH_KEY`

The frontend uses same-origin `/api` and `/ws` by default. Set `VITE_BACKEND_BASE_URL` only when the backend must be hosted on a different origin.

生产环境按单机包部署：

- systemd 将 Rust 后端常驻在 `127.0.0.1:8787`。
- Nginx 提供前端静态文件，并把 `/api/*` 和 `/ws` 反代到后端。
- GitHub Actions 工作流 `.github/workflows/deploy.yml` 可手动触发构建、打包并通过 SSH 发布。

仓库需要配置 `DEPLOY_HOST`、`DEPLOY_PORT`、`DEPLOY_USER`、`DEPLOY_SSH_KEY` 这几个 Secrets。不要把服务器密码、Personal Access Token 或 API Key 写入仓库。

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

## Persistence / 持久化

The backend now supports PostgreSQL persistence and Redis caching:

- PostgreSQL tables are created at startup when `DATABASE_URL` is configured.
- `VersionedPaperState` is snapshotted to `app_state_snapshots` and restored on restart.
- Strategy versions, strategy runs, open positions, fills, closed trades, equity snapshots, risk guard events, and event logs are written into dedicated tables.
- Redis stores the latest dashboard snapshot under `alphapulse:dashboard:snapshot` with `ALPHAPULSE_REDIS_TTL_SECS`.
- WebSocket uses REST snapshot + WS delta recovery. The server sends heartbeat pings; the frontend reconnects with backoff and refreshes `/api/snapshot` after reconnect.

后端现在支持 PostgreSQL 持久化和 Redis 缓存：

- 配置 `DATABASE_URL` 后，启动时自动创建 PostgreSQL 表。
- `VersionedPaperState` 会写入 `app_state_snapshots`，重启后恢复。
- 策略版本、运行、当前持仓、成交、平仓历史、权益快照、风控事件和事件日志会写入独立表。
- Redis 使用 `alphapulse:dashboard:snapshot` 缓存最新 dashboard 快照，TTL 由 `ALPHAPULSE_REDIS_TTL_SECS` 控制。
- WebSocket 采用 REST snapshot + WS delta 恢复模式。后端发送 heartbeat ping；前端断线退避重连，并在重连后重新拉取 `/api/snapshot`。

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

### `v0.1.4 preview`

- Added preview persistence support with PostgreSQL snapshots and Redis dashboard caching.
- Added startup configuration for database-required mode, Redis TTL, and local Docker services.
- Added deploy documentation for systemd, Nginx, and the manual GitHub Actions deploy flow.
- Improved realtime recovery with snapshot refresh after WebSocket reconnect.
- Expanded tests around persistence, domain configuration, strategy versions, and frontend reconnect/review behavior.

### 中文说明

- 增加 PostgreSQL 快照和 Redis dashboard 缓存的预览版持久化能力。
- 增加数据库强制模式、Redis TTL 和本地 Docker 服务相关配置。
- 补充 systemd、Nginx 和 GitHub Actions 手动部署流程文档。
- 优化 WebSocket 重连后的 snapshot 恢复逻辑。
- 补充持久化、域名配置、策略版本、前端重连和 Review 行为相关测试。

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
