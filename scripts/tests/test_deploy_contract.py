import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[2]


class DeployContractTest(unittest.TestCase):
    def test_workflow_packages_both_binaries_and_exposes_guarded_reset(self) -> None:
        workflow = (ROOT / ".github/workflows/deploy.yml").read_text()

        self.assertIn("reset_strategy_state:", workflow)
        self.assertIn("default: false", workflow)
        self.assertIn("--bins", workflow)
        self.assertIn("release/bin/strategy_admin", workflow)
        self.assertIn("release/deploy/install.sh", workflow)
        self.assertIn("RESET_STRATEGY_STATE", workflow)
        self.assertIn("ALPHAPULSE_TEST_DATABASE_URL", workflow)
        self.assertIn("ALPHAPULSE_TEST_REDIS_URL", workflow)
        self.assertIn("--test persistence_integration -- --ignored --test-threads=1", workflow)
        self.assertIn("--test strategy_admin -- --ignored --test-threads=1", workflow)

    def test_remote_installer_provisions_persistence_before_starting_app(self) -> None:
        installer = (ROOT / "deploy/install.sh").read_text()

        for required in (
            "postgresql",
            "redis-server",
            "ALPHAPULSE_DATABASE_URL",
            "ALPHAPULSE_REDIS_URL",
            "ALPHAPULSE_REQUIRE_DATABASE",
            "strategy_admin backup",
            "strategy_admin reset-restored-v3",
            'ln -sfn "$RELEASE" "$APP_DIR/current"',
            'systemctl restart "$SERVICE_NAME"',
            'curl --fail --silent --show-error "$SNAPSHOT_URL"',
            '.paper.strategy_version == "v0.1.3"',
            '.persistence.status == "healthy"',
        ):
            self.assertIn(required, installer)

        self.assertLess(
            installer.index("strategy_admin backup"),
            installer.index("strategy_admin reset-restored-v3"),
        )
        self.assertLess(
            installer.index("strategy_admin reset-restored-v3"),
            installer.index('ln -sfn "$RELEASE" "$APP_DIR/current"'),
        )

    def test_systemd_waits_for_database_and_cache(self) -> None:
        service = (ROOT / "deploy/alphapulse-okx.service").read_text()

        self.assertIn("After=network-online.target postgresql.service redis-server.service", service)
        self.assertIn("Requires=postgresql.service", service)
        self.assertIn("Wants=network-online.target redis-server.service", service)


if __name__ == "__main__":
    unittest.main()
