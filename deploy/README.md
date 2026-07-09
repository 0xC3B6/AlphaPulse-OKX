# Deployment

The GitHub Actions workflow at `.github/workflows/deploy.yml` builds the Rust backend, builds the Vite frontend, packages both outputs, and deploys them to a Linux server through SSH.

Required GitHub repository secrets:

- `DEPLOY_HOST`: server IP or hostname.
- `DEPLOY_PORT`: SSH port, usually `22`.
- `DEPLOY_USER`: SSH user, usually `root` for first setup.
- `DEPLOY_SSH_KEY`: private SSH key whose public key is installed in the server user's `~/.ssh/authorized_keys`.

The deployed layout is:

- Backend binary: `/opt/alphapulse-okx/current/bin/alphapulse_okx_backend`
- Frontend static files: `/opt/alphapulse-okx/current/frontend`
- systemd service: `alphapulse-okx`
- Nginx site: `/etc/nginx/sites-enabled/alphapulse-okx`
- Optional backend environment file: `/etc/alphapulse-okx.env`

Recommended `/etc/alphapulse-okx.env`:

```bash
RUST_LOG=info
DATABASE_URL=postgres://alphapulse:change-me@127.0.0.1:5432/alphapulse
REDIS_URL=redis://127.0.0.1:6379/0
ALPHAPULSE_REQUIRE_DATABASE=true
ALPHAPULSE_REDIS_TTL_SECS=30
ALPHAPULSE_WS_HEARTBEAT_SECS=15
COINGLASS_API_KEY=
```

PostgreSQL is the source of truth for paper state and event history. Redis is a live snapshot cache and can be restarted without losing final trading history.

Do not store server passwords, personal access tokens, or API keys in this repository.
