# AlphaPulse-OKX

OKX USDT 永续合约交易雷达。本项目第一版是本地行情扫描和决策辅助系统，不是自动交易系统。

## 状态

🚧 开发中

## 功能范围

- 使用 OKX 公共行情数据扫描 USDT 永续合约。
- Rust 后端负责 REST/WebSocket 行情接入、缓存、指标计算、评分和本地 API。
- TypeScript 前端负责雷达面板、过滤、详情展示和浏览器通知。
- 第一版不需要 OKX API Key，不能读取账户，不能开单、平仓或自动交易。

## Local Development

Backend:

```bash
cargo run -p alphapulse_okx_backend
```

Frontend:

```bash
cd frontend
npm install --cache ../../npm-cache
npm run dev
```

Open `http://127.0.0.1:5173`.

The backend listens on `http://127.0.0.1:8787` and exposes:

- `GET /api/health`
- `GET /api/snapshot`
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
npm test
npm run build
```
