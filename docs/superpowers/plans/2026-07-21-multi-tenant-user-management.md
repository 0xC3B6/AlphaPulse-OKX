# AlphaPulse Multi-Tenant User Management Implementation Plan

**Goal:** Deliver the approved invite-only, multilingual, responsive Web beta with managed OIDC login, server-side sessions, workspace-isolated paper trading, capabilities and entitlements, user settings, a restricted platform-admin area, auditability, and a safe legacy-data migration.

**Architecture:** Keep the existing Rust/Axum modular monolith and React/Vite frontend. Split shared market state from workspace-private runtime state, make PostgreSQL with forced RLS the private-data source of truth, use Redis for opaque sessions and rebuildable caches, and expose all private behavior through a server-derived `WorkspaceContext`. Implement the free Beta plan and entitlement model now; do not integrate payment or real trading.

**Tech Stack:** Rust 2021, Axum 0.7, Tokio, SQLx/PostgreSQL, redis-rs, generic OIDC Authorization Code + PKCE, React 18, TypeScript, Vite, Vitest, Testing Library, Nginx, GitHub Actions.

**Approved design:** `docs/superpowers/specs/2026-07-21-multi-tenant-user-management-design.md`

---

## Scope Boundary

This plan implements Phase 0 through Phase 2 of the approved design:

- security and migration foundation;
- personal workspace and user isolation;
- managed login and invite-only onboarding;
- free Beta entitlement and close-only/read-only behavior;
- responsive desktop/mobile Web pages in every locale supported by the latest local `main` (`zh`, `en`, and `ja` at plan finalization);
- platform-admin invitations, user status, entitlement grants, and audit search;
- production TLS/session/security/deployment checks.

This plan does not implement:

- OKX API keys or real trading;
- payment, invoices, refunds, or product pricing;
- team invitations or editable workspace roles;
- native iOS/Android applications;
- platform-admin impersonation or private paper-data access;
- a second frontend or mandatory PWA installation.

## Current Repository Constraints

- `backend/src/server.rs` exposes anonymous business routes, one `/ws` stream, and permissive CORS.
- `backend/src/state.rs` keeps shared market data and one private `PaperState` in the same `RadarState`.
- `backend/src/persistence.rs` creates schema at startup and binds the whole process to one configured `tenant_id/account_id`.
- `backend/src/risk_safety.rs` already has an early `AccountScope`, but it is configuration-driven rather than session/workspace-driven.
- PostgreSQL and Redis are already provisioned by `deploy/install.sh`, and CI already runs ignored integration tests against both services.
- `frontend/src/App.tsx` owns all application state; `frontend/src/api.ts` uses same-origin HTTP and one WebSocket; no client router exists.
- Local `main` at plan finalization is `43b0f3f` (`feat(ui): unify BTC quote and add Japanese locale`). It already defines `zh`, `en`, and `ja`, uses an English-base Japanese translation overlay, and contains the latest ConsoleShell/monitor behavior.
- `ConsoleShell` and CSS already provide the product shell and responsive foundation; the implementation must extend that exact local-main structure instead of restoring an older two-language snapshot.

## Execution Rules

- Treat the latest local `main` as authoritative. At implementation start, re-read `git rev-parse main`, `git log`, and `git status`; do not reset to `origin/main`, an older tag, `c5a37ae`, or any historical design snapshot.
- If local `main` has advanced after this plan, refresh file-level assumptions against it before editing. Preserve any working-tree changes and stop for ownership clarification if they overlap the task.
- Create a feature branch directly from that verified local `main` using the `codex/` prefix, for example `codex/multi-tenant-foundation`.
- Use test-first increments. Run the narrow test before and after each behavioral change.
- Commit after each task; keep database expand/backfill and contract enforcement reviewable.
- Keep every production migration forward-compatible with the previous release until the planned maintenance cutover.
- Never log or commit OIDC secrets, session IDs, invitation tokens, CSRF tokens, database credentials, or user-private data.
- Keep authentication provider-neutral through OIDC discovery. Select and configure the managed provider outside the repository before the OIDC cutover.
- Do not deploy or push merely by executing this plan; deployment remains a separate explicit user decision.
- Do not add payment or real-trading code while implementing this plan.

## Target File Structure

### Backend foundation

- Create `backend/migrations/`: SQLx migrations replacing startup DDL.
- Create `backend/src/tenancy.rs`: user/workspace identifiers and `WorkspaceContext`.
- Create `backend/src/control_plane.rs`: users, identities, workspaces, invitations, preferences, plans, subscriptions, and platform roles.
- Create `backend/src/session.rs`: opaque Redis session storage and CSRF state.
- Create `backend/src/oidc.rs`: provider-neutral OIDC authorization/callback adapter.
- Create `backend/src/authorization.rs`: capabilities, roles, entitlement decisions, and standard denials.
- Create `backend/src/audit.rs`: append-only security and administration audit writer.
- Create `backend/src/http/`: auth, bootstrap, account settings, admin, and WebSocket handlers.
- Create `backend/src/workspace_runtime.rs`: on-demand workspace-private paper runtime registry.
- Modify `backend/src/persistence.rs`: shared pool plus workspace-scoped repositories and migrations.
- Modify `backend/src/state.rs`: shared market state only; remove the global private paper singleton.
- Modify `backend/src/runtime.rs`: shared market ingestion plus active-workspace strategy scheduling.
- Modify `backend/src/server.rs`: service composition, middleware, protected routes, and split WebSockets.

### Frontend foundation

- Create `frontend/src/auth/`: bootstrap context, protected routes, invitation and login pages.
- Create `frontend/src/settings/`: profile, preferences, sessions, export/delete, and onboarding pages.
- Create `frontend/src/admin/`: invitations, users, entitlements, and audit pages.
- Modify `frontend/src/main.tsx`: browser routing and bootstrap provider.
- Modify `frontend/src/App.tsx`: become the authenticated product-console entry rather than the whole-site router.
- Modify `frontend/src/api.ts`: typed API errors, CSRF, credentials, bootstrap/settings/admin clients, and split WebSockets.
- Modify `frontend/src/types.ts`: bootstrap, identity, entitlement, account state, and UUID identifiers.
- Modify `frontend/src/i18n.ts`: auth/settings/admin/error copy for every current `Language` value and removal of affected hard-coded shell copy without downgrading the Japanese locale added on local `main`.
- Modify `frontend/src/styles.css`: responsive auth, settings, and admin layouts while preserving the existing product console.

### Deployment and operations

- Modify `.env.example`: OIDC, public URL, auth mode, session, and initial operational settings.
- Modify `deploy/install.sh`: migration backup/cutover and fail-closed production configuration checks.
- Modify `deploy/nginx.alphapulse-okx.conf`: TLS, same-origin proxying, WebSocket routes, security headers, and no permissive CORS dependency.
- Modify `deploy/alphapulse-okx.service`: required environment and hardening where needed.
- Modify `.github/workflows/deploy.yml`: new integration/security checks and readiness verification.
- Modify `scripts/tests/test_deploy_contract.py`: assert the new production contract.

---

### Task 1: Replace Startup DDL with SQLx Migrations

**Files:**

- Modify: `backend/Cargo.toml`
- Create: `backend/migrations/202607210001_baseline.sql`
- Modify: `backend/src/persistence.rs`
- Modify: `backend/tests/persistence.rs`
- Modify: `backend/tests/persistence_integration.rs`

- [ ] **Step 1: Add a failing migration-runner test**

Add an ignored PostgreSQL integration test that starts with a clean schema, calls `PersistenceLayer::initialize()`, and verifies both `_sqlx_migrations` and the current trading tables exist. Add a unit assertion that production schema creation no longer iterates `postgres_schema_statements()`.

- [ ] **Step 2: Run the focused tests and confirm failure**

```bash
cargo test -p alphapulse_okx_backend --test persistence
cargo test -p alphapulse_okx_backend --test persistence_integration migration -- --ignored --test-threads=1
```

Expected: the migration-table assertion fails because initialization still executes inline statements.

- [ ] **Step 3: Add the baseline migration**

Move the current idempotent schema contract from `postgres_schema_statements()` into `202607210001_baseline.sql`. It must safely adopt an existing production database and create a fresh test database without losing current data.

- [ ] **Step 4: Switch initialization to the embedded migrator**

Enable SQLx migration support and call `sqlx::migrate!("./migrations").run(&pool)`. Keep `postgres_schema_statements()` only as a temporary test reference during this task, then remove it after parity is proven.

- [ ] **Step 5: Verify schema parity and current persistence behavior**

```bash
cargo test -p alphapulse_okx_backend --test persistence
cargo test -p alphapulse_okx_backend --test persistence_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend
```

- [ ] **Step 6: Commit**

```bash
git add backend/Cargo.toml backend/migrations backend/src/persistence.rs backend/tests/persistence.rs backend/tests/persistence_integration.rs Cargo.lock
git commit -m "refactor: run persistence schema through sqlx migrations"
```

### Task 2: Add the Control-Plane Schema and Repositories

**Files:**

- Create: `backend/migrations/202607210002_control_plane.sql`
- Create: `backend/src/tenancy.rs`
- Create: `backend/src/control_plane.rs`
- Create: `backend/src/entitlements.rs`
- Create: `backend/src/audit.rs`
- Modify: `backend/src/lib.rs`
- Create: `backend/tests/control_plane.rs`
- Create: `backend/tests/control_plane_integration.rs`

- [ ] **Step 1: Write model and repository contract tests**

Cover UUID parsing, active/suspended status, personal workspace creation, owner membership, platform roles, Beta plan seeding, manual subscription grants, entitlement overrides, idempotent usage events, and append-only audit events.

- [ ] **Step 2: Run tests and confirm missing modules/tables fail**

```bash
cargo test -p alphapulse_okx_backend --test control_plane
cargo test -p alphapulse_okx_backend --test control_plane_integration -- --ignored --test-threads=1
```

- [ ] **Step 3: Add control-plane tables**

Create UUID-keyed tables for:

- `users`, `identities`, `user_preferences`, `user_notification_preferences`;
- `workspaces`, `memberships`, `platform_roles`;
- `invitations`, `legal_acceptances`, `data_requests`;
- `plans`, `plan_entitlements`, `workspace_subscriptions`, `subscription_overrides`, `usage_events`;
- `audit_events`.

Add status/check constraints, unique `issuer + subject`, hashed invitation tokens, one owner membership per personal workspace, expiry indexes, idempotency indexes, and a seeded free `beta` plan. Do not put private paper data in these tables.

- [ ] **Step 4: Implement repositories**

Repositories accept the shared pool and explicit IDs. They return typed domain errors and never expose raw invitation hashes or session secrets. Audit writes are insert-only.

- [ ] **Step 5: Verify**

```bash
cargo fmt --all --check
cargo test -p alphapulse_okx_backend --test control_plane
cargo test -p alphapulse_okx_backend --test control_plane_integration -- --ignored --test-threads=1
cargo clippy -p alphapulse_okx_backend --all-targets --all-features -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add backend/migrations backend/src/tenancy.rs backend/src/control_plane.rs backend/src/entitlements.rs backend/src/audit.rs backend/src/lib.rs backend/tests/control_plane.rs backend/tests/control_plane_integration.rs
git commit -m "feat: add multi-tenant control plane"
```

### Task 3: Expand and Backfill Workspace Ownership

**Files:**

- Create: `backend/migrations/202607210003_workspace_scope_expand.sql`
- Modify: `backend/src/bin/strategy_admin.rs`
- Modify: `backend/tests/strategy_admin.rs`
- Create: `backend/tests/workspace_migration_integration.rs`
- Modify: `docs/recovery/` only if a new deterministic migration manifest is required

- [ ] **Step 1: Write a failing legacy-backfill integration test**

Seed the current pre-tenant schema with one restored-v3 account, run migrations, and assert:

- one pending-claim legacy user and personal workspace exist only when legacy rows exist;
- all private rows receive the same legacy `workspace_id`;
- balances, positions, fills, closed trades, equity, events, and snapshots are unchanged;
- migration aborts if it encounters an unsupported ambiguous legacy ownership shape instead of guessing.

- [ ] **Step 2: Add the expand/backfill migration**

Add nullable `workspace_id UUID` to the private trading tables, create the legacy owner/workspace when necessary, backfill in one transaction, add workspace-first indexes, and validate there are no null or cross-workspace relations. Retain old `tenant_id/account_id` columns temporarily for rollback compatibility.

Do not enable forced RLS in this task; the running code does not yet set database workspace context.

- [ ] **Step 3: Extend the administration CLI**

Add commands that:

- create the one-time bootstrap invitation without printing or storing it anywhere except the operator's stdout;
- mark that invitation to claim the Legacy Workspace and grant platform-admin capability;
- export a deterministic pre/post migration verification manifest;
- refuse the claim if the workspace already has a real owner.

- [ ] **Step 4: Verify migration and backup behavior**

```bash
cargo test -p alphapulse_okx_backend --test workspace_migration_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend --test strategy_admin -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend --test persistence_integration -- --ignored --test-threads=1
```

- [ ] **Step 5: Commit**

```bash
git add backend/migrations backend/src/bin/strategy_admin.rs backend/tests/strategy_admin.rs backend/tests/workspace_migration_integration.rs docs/recovery
git commit -m "feat: backfill legacy data into a personal workspace"
```

### Task 4: Implement OIDC Invitations and Opaque Redis Sessions

**Files:**

- Modify: `backend/Cargo.toml`
- Modify: `backend/src/config.rs`
- Create: `backend/src/oidc.rs`
- Create: `backend/src/session.rs`
- Create: `backend/src/http/mod.rs`
- Create: `backend/src/http/auth.rs`
- Modify: `backend/src/lib.rs`
- Modify: `backend/src/server.rs`
- Modify: `.env.example`
- Create: `backend/tests/auth.rs`
- Create: `backend/tests/session_integration.rs`

- [ ] **Step 1: Write failing auth-flow tests**

Use a fake OIDC verifier behind a trait. Cover valid invitation exchange, expired/revoked/reused invitations, pending-invite state, state/nonce/PKCE validation, already-provisioned login, identity collision, and atomic first-user provisioning.

- [ ] **Step 2: Write failing session tests**

Verify opaque token hashing, per-user session indexes, 24-hour idle timeout, 30-day absolute timeout, rotation, logout, revoke-all, CSRF binding, and Redis fail-closed behavior.

- [ ] **Step 3: Add provider-neutral OIDC configuration**

Add required production settings for public base URL, issuer, client ID, client secret, callback URL, and auth mode. `ALPHAPULSE_ENV=production` must refuse startup unless auth mode is OIDC and all required values are present. Never accept tokens directly from the frontend as application sessions.

- [ ] **Step 4: Implement invitation and callback endpoints**

Implement:

- `GET /invite/:token`: hash/validate token, create short-lived pending state, and redirect to a token-free URL;
- `GET /auth/login`: create state/nonce/PKCE and redirect to the provider;
- `GET /auth/callback`: verify provider response and atomically provision or load the user;
- `POST /api/auth/logout`: revoke the current session.

Set only `__Host-alphapulse_session` with `Secure`, `HttpOnly`, `SameSite=Lax`, `Path=/`, and no Domain in production.

- [ ] **Step 5: Verify**

```bash
cargo test -p alphapulse_okx_backend --test auth
cargo test -p alphapulse_okx_backend --test session_integration -- --ignored --test-threads=1
cargo clippy -p alphapulse_okx_backend --all-targets --all-features -- -D warnings
```

- [ ] **Step 6: Commit**

```bash
git add backend/Cargo.toml Cargo.lock backend/src/config.rs backend/src/oidc.rs backend/src/session.rs backend/src/http backend/src/lib.rs backend/src/server.rs backend/tests/auth.rs backend/tests/session_integration.rs .env.example
git commit -m "feat: add invite-only oidc sessions"
```

### Task 5: Add Request Context, Capabilities, Entitlements, and Audit

**Files:**

- Create: `backend/src/authorization.rs`
- Create: `backend/src/http/middleware.rs`
- Create: `backend/src/http/errors.rs`
- Create: `backend/src/http/bootstrap.rs`
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/src/server.rs`
- Create: `backend/tests/authorization.rs`
- Modify: `backend/tests/server.rs`

- [ ] **Step 1: Write failing policy tests**

Cover owner, platform admin, suspended user, suspended workspace, missing membership, missing capability, expired entitlement, quota exhaustion, and cross-workspace resource access. Assert platform admin has no implicit `paper.read` or RLS bypass.

- [ ] **Step 2: Define standard API errors**

Return structured JSON containing stable code, localized-message key, request ID, and safe details for:

`AUTH_REQUIRED`, `ACCOUNT_SUSPENDED`, `WORKSPACE_FORBIDDEN`, `CAPABILITY_FORBIDDEN`, `PLAN_REQUIRED`, `QUOTA_EXCEEDED`, `VERSION_CONFLICT`, `RATE_LIMITED`, and `UPSTREAM_UNAVAILABLE`.

- [ ] **Step 3: Build authenticated middleware**

The middleware loads the opaque session, active user, membership, workspace, capabilities, entitlement state, and CSRF token. It inserts typed `AuthenticatedUser` and `WorkspaceContext` extensions. Browser-supplied workspace IDs never replace this context.

- [ ] **Step 4: Implement bootstrap**

`GET /api/bootstrap` returns user, workspace, membership, capabilities, quotas, entitlement status, locale/theme/timezone, CSRF token, and product availability flags.

- [ ] **Step 5: Audit all security decisions that change state**

Write login, logout, session revocation, invite redemption, suspension, entitlement grant, and denied admin-action events with request ID and actor. Never store secrets or full private request bodies.

- [ ] **Step 6: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test authorization
cargo test -p alphapulse_okx_backend --test server
cargo test -p alphapulse_okx_backend
git add backend/src/authorization.rs backend/src/http backend/src/server.rs backend/tests/authorization.rs backend/tests/server.rs
git commit -m "feat: enforce workspace capabilities and audit"
```

### Task 6: Convert Persistence to Workspace-Scoped Repositories and Forced RLS

**Files:**

- Create: `backend/migrations/202607210004_workspace_contract.sql`
- Modify: `backend/src/persistence.rs`
- Modify: `backend/src/risk_safety.rs`
- Modify: `backend/src/paper.rs`
- Modify: `backend/src/state.rs`
- Modify: `backend/src/config.rs`
- Modify: `backend/tests/persistence.rs`
- Modify: `backend/tests/persistence_integration.rs`
- Modify: `backend/tests/risk_safety.rs`
- Modify: `frontend/src/types.ts`

- [ ] **Step 1: Add failing cross-workspace and RLS tests**

Using one connection pool and two workspace contexts, prove that identical logical run/account keys can coexist, each workspace sees only its rows, missing `app.workspace_id` sees none, and even the application table owner is constrained by `FORCE ROW LEVEL SECURITY`.

- [ ] **Step 2: Remove process-global account scope**

Change `PersistenceLayer` to own only shared PostgreSQL/Redis connections. Every private repository method accepts a typed workspace/account scope. Remove `ALPHAPULSE_TENANT_ID` and `ALPHAPULSE_ACCOUNT_ID` as runtime ownership selectors after migration compatibility is no longer needed.

- [ ] **Step 3: Normalize private identifiers and relations**

The contract migration must:

- make `workspace_id` non-null on private tables;
- use UUIDs for externally addressable private resources;
- create workspace-scoped unique constraints and composite foreign keys;
- rename `event_log` to `domain_event_log` and keep `audit_events` separate;
- rename or replace fixed app snapshots with workspace-scoped recovery snapshots;
- keep `strategy_catalog_versions` shared and bind workspace strategy profiles/runs explicitly;
- remove transitional defaults that could silently assign new rows to the Legacy Workspace.

- [ ] **Step 4: Apply RLS in every private transaction**

Begin a transaction, run `SET LOCAL app.workspace_id = $1`, then execute all private queries. Enable and force RLS for every private table. Background jobs use the same scoped transaction contract.

- [ ] **Step 5: Update risk/account scope**

Replace string `tenant_id/account_id` configuration scope with UUID workspace/account IDs. Keep account event ordering, version checks, close-only behavior, and Redis namespace isolation.

- [ ] **Step 6: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test persistence
cargo test -p alphapulse_okx_backend --test risk_safety
cargo test -p alphapulse_okx_backend --test persistence_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend
git add backend/migrations backend/src/persistence.rs backend/src/risk_safety.rs backend/src/paper.rs backend/src/state.rs backend/src/config.rs backend/tests/persistence.rs backend/tests/persistence_integration.rs backend/tests/risk_safety.rs frontend/src/types.ts .env.example
git commit -m "refactor: scope paper persistence by workspace"
```

### Task 7: Split Shared Market State from Workspace Paper Runtimes

**Files:**

- Create: `backend/src/workspace_runtime.rs`
- Modify: `backend/src/state.rs`
- Modify: `backend/src/runtime.rs`
- Modify: `backend/src/auto_strategy.rs`
- Modify: `backend/src/server.rs`
- Modify: `backend/src/lib.rs`
- Create: `backend/tests/workspace_runtime.rs`
- Modify: `backend/tests/state_prices.rs`
- Modify: `backend/tests/auto_strategy.rs`

- [ ] **Step 1: Write failing isolation and lifecycle tests**

Prove that two workspaces share symbol prices but have independent paper balances, positions, account versions, risk state, event queues, and private broadcasts. Verify loading one workspace cannot publish to another.

- [ ] **Step 2: Extract shared market state**

Keep symbols, latest prices, scan timestamps, OKX connection state, and public market broadcast in `SharedMarketState`.

- [ ] **Step 3: Add an on-demand runtime registry**

`WorkspaceRuntimeRegistry` loads private state from PostgreSQL, creates one FIFO account-event queue and private broadcast per active workspace, and evicts only safe idle runtimes. Workspaces with open positions or active strategies stay scheduled.

- [ ] **Step 4: Update scanner and automatic strategy scheduling**

Market data is fetched once. Price updates fan out to active workspace runtimes. Each strategy decision and transition runs with that workspace's capabilities, entitlement mode, risk state, and persistence transaction.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test workspace_runtime
cargo test -p alphapulse_okx_backend --test state_prices
cargo test -p alphapulse_okx_backend --test auto_strategy
cargo test -p alphapulse_okx_backend
git add backend/src/workspace_runtime.rs backend/src/state.rs backend/src/runtime.rs backend/src/auto_strategy.rs backend/src/server.rs backend/src/lib.rs backend/tests/workspace_runtime.rs backend/tests/state_prices.rs backend/tests/auto_strategy.rs
git commit -m "refactor: isolate paper runtimes per workspace"
```

### Task 8: Enforce Beta Entitlement and Expiry Modes in Paper Trading

**Files:**

- Modify: `backend/src/entitlements.rs`
- Modify: `backend/src/authorization.rs`
- Modify: `backend/src/workspace_runtime.rs`
- Modify: `backend/src/state.rs`
- Modify: `backend/src/auto_strategy.rs`
- Create: `backend/tests/entitlement_runtime.rs`
- Modify: `backend/tests/auto_strategy.rs`
- Modify: `backend/tests/state_prices.rs`

- [ ] **Step 1: Write the state-machine tests**

Cover:

- Active: open, close, and strategy decisions allowed;
- Grace/Close-only: new opens and new strategy entries denied, manual closes and stop/take-profit exits allowed, prices continue updating;
- Expired/Read-only: all trade writes and strategy execution paused, prices and valuation continue, read/export remains available;
- Renewed: trading capability returns, but strategies remain paused until explicit user resume.

- [ ] **Step 2: Implement one server-side access mode**

Compute `Active`, `CloseOnly`, or `ReadOnly` from subscription, override, quota, and account safety. Do not duplicate expiry logic in handlers, runtime, and frontend.

- [ ] **Step 3: Integrate account risk without weakening it**

Subscription close-only is an additional restriction. Existing persistence, stale-market, reconciliation, kill-switch, and stop-protection rules remain authoritative and can make the account more restrictive.

- [ ] **Step 4: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test entitlement_runtime
cargo test -p alphapulse_okx_backend --test auto_strategy
cargo test -p alphapulse_okx_backend --test state_prices
git add backend/src/entitlements.rs backend/src/authorization.rs backend/src/workspace_runtime.rs backend/src/state.rs backend/src/auto_strategy.rs backend/tests/entitlement_runtime.rs backend/tests/auto_strategy.rs backend/tests/state_prices.rs
git commit -m "feat: enforce beta entitlement modes"
```

### Task 9: Protect HTTP APIs and Split Real-Time Channels

**Files:**

- Modify: `backend/src/server.rs`
- Create: `backend/src/http/market.rs`
- Create: `backend/src/http/paper.rs`
- Create: `backend/src/http/websocket.rs`
- Modify: `backend/src/http/mod.rs`
- Modify: `backend/tests/server.rs`
- Create: `backend/tests/websocket_isolation.rs`

- [ ] **Step 1: Add failing protected-route tests**

Verify health remains public, product endpoints require a session, writes require CSRF, owner requests receive their workspace only, suspended/expired modes return standard errors, and arbitrary workspace IDs are ignored or rejected.

- [ ] **Step 2: Compose route groups**

Keep only health, invitation landing, OIDC redirects/callbacks, and legal pages public. Protect bootstrap, product, settings, admin, and WebSocket routes with the appropriate middleware and capabilities.

- [ ] **Step 3: Split WebSocket streams**

Add:

- `/ws/market`: authenticated shared market snapshot/events;
- `/ws/workspace`: current workspace private snapshot/events.

Both validate session and Origin. The private endpoint derives workspace from the session, sends monotonically sequenced events, and closes immediately after session revocation or workspace suspension.

- [ ] **Step 4: Remove permissive CORS**

Use same-origin defaults. If a development override is necessary, restrict it to configured origins and never combine wildcard origin with credentials.

- [ ] **Step 5: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test server
cargo test -p alphapulse_okx_backend --test websocket_isolation
cargo test -p alphapulse_okx_backend
git add backend/src/server.rs backend/src/http backend/tests/server.rs backend/tests/websocket_isolation.rs
git commit -m "feat: protect tenant APIs and realtime streams"
```

### Task 10: Add Frontend Bootstrap, Authentication Routes, and Typed Errors

**Files:**

- Modify: `frontend/package.json`
- Modify: `frontend/package-lock.json`
- Create: `frontend/src/auth/AuthProvider.tsx`
- Create: `frontend/src/auth/ProtectedRoute.tsx`
- Create: `frontend/src/auth/LoginPage.tsx`
- Create: `frontend/src/auth/InvitePage.tsx`
- Create: `frontend/src/auth/AuthErrorPage.tsx`
- Create: `frontend/src/auth/auth.test.tsx`
- Modify: `frontend/src/main.tsx`
- Modify: `frontend/src/App.tsx`
- Modify: `frontend/src/api.ts`
- Modify: `frontend/src/api.test.ts`
- Modify: `frontend/src/realtime.ts`
- Modify: `frontend/src/realtime.test.ts`
- Modify: `frontend/src/types.ts`

- [ ] **Step 1: Refresh against the latest local main before touching shared files**

Re-read `git rev-parse main`, `git status`, and the current `App.tsx`, `App.test.tsx`, `MonitorPage.tsx`, `ConsoleShell.tsx`, and `i18n.ts`. Confirm the feature branch starts from the latest local `main` and includes the BTC-quote/Japanese-locale work. Do not overwrite or silently revert later local-main changes.

- [ ] **Step 2: Write failing frontend auth tests**

Cover bootstrap loading, authenticated product render, `AUTH_REQUIRED` redirect with return URL, invitation status, login language selection, suspended account screen, logout, and CSRF on non-GET requests.

- [ ] **Step 3: Add routing and bootstrap context**

Add browser routing. `AuthProvider` loads `/api/bootstrap`, keeps CSRF only in memory, exposes user/workspace/capabilities/entitlement, and renders public or protected routes without duplicating the product console.

- [ ] **Step 4: Add typed API errors**

`requestJson` must send same-origin credentials, attach `X-CSRF-Token` to state-changing requests, parse stable backend error codes, and preserve request IDs for support. The UI maps codes to translated copy instead of showing raw backend messages.

- [ ] **Step 5: Split real-time clients**

Connect shared and private sockets independently. A private socket auth failure stops automatic reconnect and returns to bootstrap/login; a transient network close keeps bounded reconnect behavior.

- [ ] **Step 6: Verify and commit**

```bash
cd frontend
npm test -- auth/auth.test.tsx api.test.ts realtime.test.ts
npm run lint
npm run build
git add package.json package-lock.json src/auth src/main.tsx src/App.tsx src/api.ts src/api.test.ts src/realtime.ts src/realtime.test.ts src/types.ts
git commit -m "feat: add authenticated frontend bootstrap"
```

### Task 11: Add Multilingual User Settings and Onboarding

**Files:**

- Create: `backend/src/http/account.rs`
- Modify: `backend/src/http/mod.rs`
- Create: `backend/tests/account_api.rs`
- Create: `frontend/src/settings/OnboardingPage.tsx`
- Create: `frontend/src/settings/ProfilePage.tsx`
- Create: `frontend/src/settings/PreferencesPage.tsx`
- Create: `frontend/src/settings/SessionsPage.tsx`
- Create: `frontend/src/settings/DataRequestsPage.tsx`
- Create: `frontend/src/settings/settings.test.tsx`
- Modify: `frontend/src/ConsoleShell.tsx`
- Modify: `frontend/src/i18n.ts`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write backend and frontend tests first**

Cover profile/preferences updates, server-wins locale after login, all currently supported locale values, revoke one/all other sessions, legal acceptance versioning, export/delete requests, notification opt-in remaining optional, and account-menu navigation.

- [ ] **Step 2: Implement account APIs**

Add capability-protected endpoints for preferences, sessions, legal acceptance, export, and deletion. Sensitive actions require a fresh-enough session or OIDC re-auth marker and always write audit events.

- [ ] **Step 3: Implement onboarding**

The Web flow is: invite/login, language/timezone, legal/risk acceptance, private paper-account initialization, optional browser notification, then the existing console.

- [ ] **Step 4: Reuse and complete current i18n**

Add all new copy to the current `zh`, `en`, and `ja` contract. Preserve the existing Japanese overlay/fallback structure unless local `main` changes it before implementation. Move affected hard-coded ConsoleShell/user-management strings into the existing translation structure; do not introduce a second i18n framework.

- [ ] **Step 5: Add responsive settings layouts**

Desktop uses the existing shell. Mobile uses the same routes/components with single-column forms, compact navigation, safe full-screen confirmations, and localized table overflow. Do not add native-app or PWA-install behavior.

- [ ] **Step 6: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test account_api
cd frontend
npm test -- settings/settings.test.tsx App.test.tsx styles.test.ts
npm run lint
npm run build
git add ../backend/src/http/account.rs ../backend/src/http/mod.rs ../backend/tests/account_api.rs src/settings src/ConsoleShell.tsx src/i18n.ts src/styles.css src/App.test.tsx src/styles.test.ts
git commit -m "feat: add bilingual account settings"
```

### Task 12: Add the Restricted Platform-Admin Area

**Files:**

- Create: `backend/src/http/admin.rs`
- Modify: `backend/src/http/mod.rs`
- Create: `backend/tests/admin_api.rs`
- Create: `frontend/src/admin/AdminLayout.tsx`
- Create: `frontend/src/admin/InvitationsPage.tsx`
- Create: `frontend/src/admin/UsersPage.tsx`
- Create: `frontend/src/admin/EntitlementsPage.tsx`
- Create: `frontend/src/admin/AuditPage.tsx`
- Create: `frontend/src/admin/admin.test.tsx`
- Modify: `frontend/src/api.ts`
- Modify: `frontend/src/types.ts`
- Modify: `frontend/src/i18n.ts`
- Modify: `frontend/src/styles.css`

- [ ] **Step 1: Write authorization-boundary tests**

An owner without platform capability cannot see or call admin features. A platform admin can manage invitations, user/workspace status, manual Beta grants, and audit search, but cannot fetch positions, fills, strategy runs, private snapshots, or impersonate a user.

- [ ] **Step 2: Implement admin APIs**

Require explicit admin capabilities. Return invitation plaintext only once at creation. Suspension revokes sessions and private sockets. Entitlement changes require reason and expiry. Every write creates an audit event.

- [ ] **Step 3: Implement the responsive admin Web pages**

Keep the admin area in the same React app and translation system. Hide its navigation unless bootstrap capabilities allow it, while still enforcing every permission on the server.

- [ ] **Step 4: Verify and commit**

```bash
cargo test -p alphapulse_okx_backend --test admin_api
cd frontend
npm test -- admin/admin.test.tsx
npm run lint
npm run build
git add ../backend/src/http/admin.rs ../backend/src/http/mod.rs ../backend/tests/admin_api.rs src/admin src/api.ts src/types.ts src/i18n.ts src/styles.css
git commit -m "feat: add audited beta administration"
```

### Task 13: Harden Production Deployment and Operations

**Files:**

- Modify: `backend/Cargo.toml`
- Modify: `backend/src/config.rs`
- Modify: `backend/src/server.rs`
- Create: `backend/src/rate_limit.rs`
- Create: `backend/tests/security_headers.rs`
- Modify: `.env.example`
- Modify: `deploy/nginx.alphapulse-okx.conf`
- Modify: `deploy/install.sh`
- Modify: `deploy/alphapulse-okx.service`
- Modify: `deploy/README.md`
- Modify: `.github/workflows/deploy.yml`
- Modify: `scripts/tests/test_deploy_contract.py`

- [ ] **Step 1: Write failing deployment/security contract tests**

Assert production requires HTTPS public URL, OIDC config, Redis, database, auth mode OIDC, secure cookie behavior, precise Origin policy, readiness health, split WebSocket proxying, no permissive CORS, and no private snapshot in the deployment health probe.

- [ ] **Step 2: Add request IDs and rate limits**

Assign request IDs, return them in errors, and rate-limit invitation/auth, admin, and paper-write routes separately. Use Redis-backed limits where consistency matters; fail safely if the session/security store is unavailable.

- [ ] **Step 3: Harden Nginx**

Add HTTPS/WSS, HTTP-to-HTTPS redirect, exact server name, HSTS, CSP, frame denial, MIME sniffing protection, and a safe Referrer Policy. Proxy `/api`, `/auth`, `/invite`, `/ws/market`, and `/ws/workspace` to the backend. Parameterize certificate/public-host configuration; do not commit certificates.

- [ ] **Step 4: Make deployment migration-aware**

Before switching the release symlink:

- create and verify a PostgreSQL/strategy backup;
- run migrations while the service is stopped for the maintenance cutover;
- verify migration version and legacy manifest;
- restart and poll a public readiness endpoint that reports only dependency health;
- restore the previous binary only when its schema compatibility contract permits it.

- [ ] **Step 5: Extend CI**

Run control-plane, session, workspace migration, RLS, WebSocket isolation, and strategy-admin integration tests against the existing PostgreSQL/Redis services. Keep full Rust and frontend suites.

- [ ] **Step 6: Verify and commit**

```bash
cargo fmt --all --check
cargo test -p alphapulse_okx_backend
cargo clippy -p alphapulse_okx_backend --all-targets --all-features -- -D warnings
python3 -m unittest scripts.tests.test_deploy_contract -v
bash -n deploy/install.sh
git add backend/Cargo.toml Cargo.lock backend/src/config.rs backend/src/server.rs backend/src/rate_limit.rs backend/tests/security_headers.rs .env.example deploy .github/workflows/deploy.yml scripts/tests/test_deploy_contract.py
git commit -m "chore: harden multi-tenant beta deployment"
```

### Task 14: Run the Full Beta Cutover Rehearsal

**Files:**

- Create: `docs/runbooks/multi-tenant-beta-cutover.md`
- Create: `docs/runbooks/multi-tenant-incident-response.md`
- Modify: `README.md` only where current local-only/auth instructions are now incorrect
- Modify: tests or code only for defects discovered by the rehearsal

- [ ] **Step 1: Rehearse from a production-shaped backup**

In an isolated PostgreSQL/Redis environment:

1. restore a copy of the current legacy database;
2. record the pre-migration manifest;
3. run the complete migration;
4. create the bootstrap invitation and claim the Legacy Workspace;
5. compare row counts, balance, positions, fills, closed trades, equity, PnL, and snapshot checksums;
6. create a second invited user and prove all cross-workspace access paths fail;
7. revoke sessions and prove HTTP and both WebSockets close/fail correctly;
8. expire and renew Beta entitlement through Active, Close-only, Read-only, and Renewed;
9. exercise `zh`, `en`, and `ja` flows at desktop and phone widths;
10. perform backup restore and the documented rollback decision.

- [ ] **Step 2: Run every automated gate**

```bash
cargo fmt --all --check
cargo test -p alphapulse_okx_backend
cargo test -p alphapulse_okx_backend --test persistence_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend --test control_plane_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend --test session_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend --test workspace_migration_integration -- --ignored --test-threads=1
cargo test -p alphapulse_okx_backend --test strategy_admin -- --ignored --test-threads=1
cargo clippy -p alphapulse_okx_backend --all-targets --all-features -- -D warnings
cd frontend
npm test
npm run lint
npm run build
cd ..
python3 -m unittest scripts.tests.test_deploy_contract -v
bash -n deploy/install.sh
```

- [ ] **Step 3: Verify explicit non-goals**

Search the built product and code paths to confirm there is no OKX private credential field, real-order endpoint, payment-provider integration, native-app project, admin impersonation, or platform-admin private paper-data route.

- [ ] **Step 4: Document the operator runbooks**

The cutover runbook must include backup paths, migration version checks, bootstrap invitation handling, smoke tests, audit queries, session revocation, entitlement recovery, and rollback limits. The incident runbook must cover suspected tenant leakage, session compromise, invitation abuse, stale market data, Redis outage, and database recovery.

- [ ] **Step 5: Commit final verification artifacts**

```bash
git add docs/runbooks README.md
git commit -m "docs: add multi-tenant beta cutover runbooks"
```

---

## Release Gate

The invite-only Beta is not ready until all of these are true:

- the implementation branch is based on the latest verified local `main`, and none of its existing frontend or locale behavior has been reverted;
- every private table has non-null workspace ownership and forced RLS;
- platform-admin tests prove no private paper-data access;
- the Legacy Workspace manifest matches balances, positions, fills, trades, equity, and PnL;
- OIDC invitation, session rotation/revocation, CSRF, Origin, and rate-limit tests pass;
- Active, Close-only, Read-only, and Renewed behavior pass end to end;
- desktop and phone browser flows pass in every locale supported by the implementation baseline;
- backup, restore, and migration rollback rehearsal succeeds;
- production HTTPS/WSS and security headers are verified;
- no real trading, OKX private credentials, payment integration, native App, or impersonation exists.

Payment-provider selection, product pricing, and team roles start only after this gate and require their own approved design/plan update.
