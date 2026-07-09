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

Do not store server passwords, personal access tokens, or API keys in this repository.
