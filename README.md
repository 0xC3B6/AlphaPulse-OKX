# AlphaPulse-OKX

AlphaPulse-OKX 是一个本地运行的 OKX USDT 永续合约交易雷达，用于行情扫描、机会排序和决策辅助。它不是自动交易系统，也不会连接真实账户下单。

## Version

当前版本：`v0.1`

## What It Does

- 扫描 OKX USDT 永续合约，并维护动态池与固定关注列表。
- 计算短周期涨跌、趋势分、震荡分、FVG 区域、支撑/阻力和触发原因。
- 提供实时雷达控制台：状态栏、过滤器、密集行情表、选中合约详情面板。
- 在 Radar 页展示 BTC 大周期摘要，并提供完整 Macro 页用于周期、估值和 AHR999 分析。
- 内置 K 线/FVG 图表、TradingView 弹窗入口和浏览器通知。
- 支持本地纸面交易：开仓、平仓、持仓、权益、已实现/未实现盈亏。
- 支持 Light / Dark / System 主题，以及中文 / English 切换。

## What It Does Not Do

- 不读取 OKX 账户。
- 不需要 OKX API Key。
- 不会真实开单、平仓或自动交易。
- 不提供投资建议；所有信号只用于辅助观察和复盘。

## Tech Stack

- Backend: Rust, Axum, Tokio, OKX REST/WebSocket public market data.
- Frontend: React, TypeScript, Vite, Lightweight Charts.
- Runtime: local-only backend on `127.0.0.1:8787`, frontend on `127.0.0.1:5173`.

## Local Development

Optional macro valuation data uses `COINGLASS_API_KEY`. The app can run without it, but some external valuation metrics may be unavailable.

```bash
cp .env.example .env.local
```

Backend:

```bash
cargo run -p alphapulse_okx_backend
```

Frontend:

```bash
cd frontend
npm ci
npm run dev
```

Open:

```text
http://127.0.0.1:5173
```

## API

The backend listens on `http://127.0.0.1:8787`.

- `GET /api/health`
- `GET /api/snapshot`
- `GET /api/macro/btc`
- `GET /api/symbols/:inst_id/chart?timeframe=m15&limit=180&filled=true`
- `GET /api/paper`
- `POST /api/paper/orders`
- `POST /api/paper/positions/:inst_id/close`
- `GET /ws`

## Verification

Backend:

```bash
cargo test -p alphapulse_okx_backend
cargo check -p alphapulse_okx_backend
```

Frontend:

```bash
cd frontend
npm run lint
npm test
npm run build
```

## Notes

- Frontend currently expects the backend at `http://127.0.0.1:8787`.
- The scanner refreshes local state continuously while the backend process is running.
- Browser notification permission is controlled by the browser and is shown in the console status bar.
- Paper trading state is local in-memory runtime state, not an exchange-side account.
