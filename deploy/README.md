# Deployment

`.github/workflows/deploy.yml` builds and tests the Rust backend and Vite frontend, packages both backend binaries, and deploys through SSH.

Required GitHub repository secrets:

- `DEPLOY_HOST`: server IP or hostname.
- `DEPLOY_PORT`: SSH port, usually `22`.
- `DEPLOY_USER`: SSH user, currently expected to be `root` because the installer provisions system services.
- `DEPLOY_SSH_KEY_B64`: base64-encoded private SSH key whose public key is authorized on the server.

The manual `reset_strategy_state` input defaults to `false`. Set it to `true` only for the one-time restored-v3 cutover (or another deliberate destructive reset). The installer first exports and verifies v3/v4 strategy rows, then atomically replaces them with a fresh restored-v3 checkpoint. Ordinary deployments preserve all PostgreSQL history.

The installer provisions PostgreSQL and Redis when needed. It creates `/etc/alphapulse-okx.env` with an owner-only generated database credential when no database URL exists. PostgreSQL is the source of truth; Redis is a rebuildable live-snapshot cache.

Deployed layout:

- Backend: `/opt/alphapulse-okx/current/bin/alphapulse_okx_backend`
- Administration CLI: `/opt/alphapulse-okx/current/bin/strategy_admin`
- Frontend: `/opt/alphapulse-okx/current/frontend`
- Verified backups: `/opt/alphapulse-okx/backups`
- Backend service: `alphapulse-okx`
- Environment file: `/etc/alphapulse-okx.env`

After switching the release symlink, deployment verifies that `/api/snapshot` reports restored v3 and healthy PostgreSQL-backed persistence. A failed verification restores the previous release symlink.

Do not store server passwords, private keys, database credentials, or API keys in the repository.
