# AlphaPulse-OKX

AlphaPulse-OKX es un radar local de futuros perpetuos de OKX USDT para el escaneo del mercado, clasificación de señales, contexto macro y revisión de paper-trading.

- **Versión actual:** `v0.1.3 Optimized front end`
- **Ejecución:** solo local
- **Modo de trading:** solo paper trading

## Descripción general

AlphaPulse-OKX se conecta a los datos públicos del mercado de OKX, construye un radar en tiempo real para contratos perpetuos de USDT y presenta señales a corto plazo junto con el contexto del macro-ciclo de BTC. La aplicación está diseñada para la observación, revisión y soporte de decisiones. No se conecta a una cuenta real de OKX y no coloca órdenes en el exchange.

## Aspectos destacados

| Área | Descripción |
| --- | --- |
| Radar | Radar de mercado estilo terminal en tiempo real con filtros, barra de estado, tabla densa, detalles del símbolo seleccionado y entrada a TradingView. |
| Señales | Rastrea el cambio de precio a corto plazo, puntuación de tendencia, puntuación de chop (rango), zonas FVG, soporte/resistencia y razones de activación. |
| Macro | Resumen macro de BTC en el Radar, más una página Macro completa para análisis de ciclo, valoración y contexto AHR999. |
| Estrategia | Espacio de trabajo de estrategia para atribución de señales, revisión de patrones, posiciones sombra y posiciones de paper trading activas. |
| Paper trade | Paper trading local con órdenes, posiciones, equidad, PnL realizado, PnL no realizado, comisiones e historial de posiciones. |
| Revisión | Página de revisión para la curva de equidad, historial de operaciones, métricas de rendimiento y comparación de versiones de estrategia. |
| UI | Temas Light, Dark y System, además de cambio de idioma Inglés/Chino mediante controles compactos. |

## Páginas

| Página | Propósito |
| --- | --- |
| Monitor / Radar | Consola principal del radar en tiempo real para escanear, ordenar, filtrar, graficar y ver detalles del símbolo. |
| Macro | Ciclo de BTC, valoración, AHR999 y contexto de permisos macro. |
| Estrategia | Atribución de señales, estadísticas de patrones, insights de estrategia y posiciones activas/sombra. |
| Trade | Entrada de órdenes de paper trading, posiciones abiertas y estado de la cuenta local. |
| Review | Rendimiento del paper trading, posiciones cerradas, curva de equidad/PnL y comparación de estrategias. |

## Lo que NO hace

- No lee una cuenta de OKX.
- No requiere una clave API de OKX.
- No coloca, cierra ni automatiza órdenes reales en el exchange.
- No es asesoramiento financiero. Las señales son solo para observación y revisión.
- El estado del paper trading es un estado de ejecución local, no una cuenta en el lado del exchange.

## Stack Tecnológico

| Capa | Stack |
| --- | --- |
| Backend | Rust, Axum, Tokio, datos de mercado públicos REST/WebSocket de OKX |
| Frontend | React, TypeScript, Vite, Lightweight Charts, Recharts, Tailwind CSS, lucide-react |
| Ejecución | Backend en `127.0.0.1:8787`, frontend en `127.0.0.1:5173` |
| Datos | Datos públicos del mercado de OKX, datos opcionales de valoración macro de Coinglass |

## Inicio Rápido

Requisitos:

- Toolchain de Rust stable
- Node.js LTS actual y npm
- Acceso de red a los datos públicos del mercado de OKX

Los datos opcionales de valoración macro utilizan `COINGLASS_API_KEY`. La aplicación puede ejecutarse sin ella, pero algunas métricas de valoración externa podrían no estar disponibles.

```bash
cp .env.example .env.local
```

Iniciar el backend:

```bash
cargo run -p alphapulse_okx_backend
```

Iniciar el frontend:

```bash
cd frontend
npm ci
npm run dev
```

Abrir:

```text
http://127.0.0.1:5173
```

## API / Interfaces Locales

El backend escucha en `http://127.0.0.1:8787`.

| Método | Endpoint | Propósito |
| --- | --- | --- |
| `GET` | `/api/health` | Verificación de estado (Health check) |
| `GET` | `/api/snapshot` | Instantánea del radar |
| `GET` | `/api/macro/btc` | Datos macro de BTC |
| `GET` | `/api/symbols/:inst_id/chart?timeframe=m15&limit=180&filled=true` | Velas del gráfico del símbolo y datos FVG |
| `GET` | `/api/paper` | Estado de la cuenta de paper trading |
| `POST` | `/api/paper/orders` | Enviar una orden de paper trading |
| `POST` | `/api/paper/positions/:inst_id/close` | Cerrar una posición de paper trading |
| `GET` | `/ws` | Stream de WebSocket en tiempo real |

## Verificación

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

## Estructura del Proyecto

```text
.
├── backend/              # Backend en Rust y escáner de datos de mercado de OKX
├── frontend/             # Aplicación frontend React/Vite
├── docs/superpowers/     # Notas de planificación e implementación
├── .env.example          # Ejemplo de entorno local opcional
├── Cargo.toml            # Workspace de Rust
└── README.md
```

## Notas de Versión

### `v0.1.3 Optimized front end`

- Se rediseñó el radar para convertirlo en una interfaz compacta estilo terminal alineada con el sistema de colores del Radar.
- Se agregaron controles más densos para el cambio de tema e idioma.
- Se mejoró la disposición de las métricas del mercado para que los valores de precio/cambio permanezcan dentro de sus paneles.
- Se agregaron superficies de estrategia y revisión para el análisis de señales y la revisión de paper trading.
- Se actualizó la documentación a un README bilingüe estilo GitHub.

## Licencia

Actualmente no se incluye ningún archivo de licencia. Agregue una licencia antes de distribuir o reutilizar este proyecto fuera del alcance previsto por el propietario del repositorio.
