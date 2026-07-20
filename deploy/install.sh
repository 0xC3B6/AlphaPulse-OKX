#!/usr/bin/env bash
set -Eeuo pipefail

: "${APP_SHA:?APP_SHA is required}"
: "${RELEASE_SOURCE:?RELEASE_SOURCE is required}"

if [[ ! "$APP_SHA" =~ ^[0-9a-f]{40}$ ]]; then
  echo "APP_SHA must be a full lowercase Git commit SHA" >&2
  exit 2
fi
if [[ ! -d "$RELEASE_SOURCE" ]]; then
  echo "RELEASE_SOURCE does not exist: $RELEASE_SOURCE" >&2
  exit 2
fi

RESET_STRATEGY_STATE="${RESET_STRATEGY_STATE:-false}"
if [[ "$RESET_STRATEGY_STATE" != "true" && "$RESET_STRATEGY_STATE" != "false" ]]; then
  echo "RESET_STRATEGY_STATE must be true or false" >&2
  exit 2
fi

APP_DIR=/opt/alphapulse-okx
SERVICE_USER=alphapulse
SERVICE_NAME=alphapulse-okx
ENV_FILE=/etc/alphapulse-okx.env
RELEASE="$APP_DIR/releases/$APP_SHA"
SNAPSHOT_URL=http://127.0.0.1:8787/api/snapshot
PREVIOUS_RELEASE=""

if [[ -L "$APP_DIR/current" ]]; then
  PREVIOUS_RELEASE="$(readlink -f "$APP_DIR/current" || true)"
fi

rollback() {
  local exit_code=$?
  trap - ERR
  if [[ -n "$PREVIOUS_RELEASE" && -d "$PREVIOUS_RELEASE" ]]; then
    echo "Deployment failed; restoring $PREVIOUS_RELEASE" >&2
    ln -sfn "$PREVIOUS_RELEASE" "$APP_DIR/current"
    systemctl restart "$SERVICE_NAME" || true
  fi
  exit "$exit_code"
}
trap rollback ERR

missing_packages=()
for package in nginx ca-certificates curl jq openssl postgresql redis-server; do
  if ! dpkg-query -W -f='${Status}' "$package" 2>/dev/null | grep -q "ok installed"; then
    missing_packages+=("$package")
  fi
done
if (( ${#missing_packages[@]} > 0 )); then
  apt-get update
  DEBIAN_FRONTEND=noninteractive apt-get install -y "${missing_packages[@]}"
fi

systemctl enable --now postgresql
systemctl enable --now redis-server

postgres_ready=false
redis_ready=false
for _ in $(seq 1 30); do
  if runuser -u postgres -- pg_isready --quiet; then
    postgres_ready=true
    break
  fi
  sleep 1
done
for _ in $(seq 1 30); do
  if redis-cli ping 2>/dev/null | grep -q '^PONG$'; then
    redis_ready=true
    break
  fi
  sleep 1
done
if [[ "$postgres_ready" != "true" || "$redis_ready" != "true" ]]; then
  echo "PostgreSQL or Redis did not become ready" >&2
  exit 1
fi

if ! id -u "$SERVICE_USER" >/dev/null 2>&1; then
  useradd --system --home-dir "$APP_DIR" --shell /usr/sbin/nologin "$SERVICE_USER"
fi

mkdir -p "$APP_DIR/releases" "$APP_DIR/backups"
rm -rf "$RELEASE"
mkdir -p "$RELEASE"
cp -a "$RELEASE_SOURCE/." "$RELEASE/"
chmod 0755 "$RELEASE/bin/alphapulse_okx_backend" "$RELEASE/bin/strategy_admin" "$RELEASE/deploy/install.sh"
chown -R "$SERVICE_USER:$SERVICE_USER" "$APP_DIR"

touch "$ENV_FILE"

set_env() {
  local key="$1"
  local value="$2"
  local temporary
  temporary="$(mktemp)"
  grep -v "^${key}=" "$ENV_FILE" > "$temporary" || true
  printf '%s=%s\n' "$key" "$value" >> "$temporary"
  install -o root -g "$SERVICE_USER" -m 0640 "$temporary" "$ENV_FILE"
  rm -f "$temporary"
}

if ! grep -Eq '^(ALPHAPULSE_DATABASE_URL|DATABASE_URL)=.+' "$ENV_FILE"; then
  database_password="$(openssl rand -hex 32)"
  runuser -u postgres -- psql --set=ON_ERROR_STOP=1 --set=db_password="$database_password" <<'SQL'
SELECT format('CREATE ROLE alphapulse LOGIN PASSWORD %L', :'db_password')
WHERE NOT EXISTS (SELECT FROM pg_catalog.pg_roles WHERE rolname = 'alphapulse') \gexec
SELECT format('ALTER ROLE alphapulse WITH LOGIN PASSWORD %L', :'db_password') \gexec
SELECT 'CREATE DATABASE alphapulse OWNER alphapulse'
WHERE NOT EXISTS (SELECT FROM pg_database WHERE datname = 'alphapulse') \gexec
SQL
  set_env ALPHAPULSE_DATABASE_URL "postgresql://alphapulse:${database_password}@127.0.0.1:5432/alphapulse"
fi
if ! grep -Eq '^(ALPHAPULSE_REDIS_URL|REDIS_URL)=.+' "$ENV_FILE"; then
  set_env ALPHAPULSE_REDIS_URL redis://127.0.0.1:6379/0
fi
set_env ALPHAPULSE_REQUIRE_DATABASE true
if ! grep -q '^ALPHAPULSE_REDIS_TTL_SECS=' "$ENV_FILE"; then
  set_env ALPHAPULSE_REDIS_TTL_SECS 30
fi
chown "root:$SERVICE_USER" "$ENV_FILE"
chmod 0640 "$ENV_FILE"

install -m 0644 "$RELEASE/deploy/alphapulse-okx.service" /etc/systemd/system/alphapulse-okx.service
install -m 0644 "$RELEASE/deploy/nginx.alphapulse-okx.conf" /etc/nginx/sites-available/alphapulse-okx
ln -sfn /etc/nginx/sites-available/alphapulse-okx /etc/nginx/sites-enabled/alphapulse-okx
rm -f /etc/nginx/sites-enabled/default
systemctl daemon-reload
systemctl enable --now nginx
nginx -t
systemctl reload nginx

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

strategy_admin() {
  runuser --preserve-environment -u "$SERVICE_USER" -- "$RELEASE/bin/strategy_admin" "$@"
}

if [[ "$RESET_STRATEGY_STATE" == "true" ]]; then
  systemctl stop "$SERVICE_NAME" 2>/dev/null || true
  backup_root="$APP_DIR/backups/${APP_SHA}-$(date -u +%Y%m%dT%H%M%SZ)"
  mkdir -p "$backup_root"
  chown "$SERVICE_USER:$SERVICE_USER" "$backup_root"
  strategy_admin backup --output "$backup_root"
  backup_manifest="$(find "$backup_root" -type f -name manifest.json -print | sort | tail -n 1)"
  if [[ -z "$backup_manifest" ]]; then
    echo "Verified backup manifest was not produced" >&2
    exit 1
  fi
  strategy_admin reset-restored-v3 --backup-manifest "$backup_manifest"
fi

ln -sfn "$RELEASE" "$APP_DIR/current"
systemctl enable "$SERVICE_NAME"
systemctl restart "$SERVICE_NAME"

deployment_healthy=false
for _ in $(seq 1 30); do
  if snapshot="$(curl --fail --silent --show-error "$SNAPSHOT_URL" 2>/dev/null)" &&
    jq -e '
      .paper.strategy_version == "v0.1.3" and
      .paper.strategy_build_id == "legacy-v3-replay-2026-07-10" and
      .paper.persistence.status == "healthy" and
      .persistence.status == "healthy"
    ' >/dev/null <<<"$snapshot"; then
    deployment_healthy=true
    break
  fi
  sleep 2
done
if [[ "$deployment_healthy" != "true" ]]; then
  journalctl -u "$SERVICE_NAME" --no-pager -n 100 >&2 || true
  echo "Restored v3 did not pass its persistence-aware health check" >&2
  exit 1
fi

systemctl --no-pager --full status "$SERVICE_NAME"
trap - ERR
