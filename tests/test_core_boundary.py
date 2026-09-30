import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / "tools/check-core-boundary.py"
spec = importlib.util.spec_from_file_location("core_boundary", SCRIPT)
boundary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(boundary)


class CoreBoundaryTests(unittest.TestCase):
    def test_business_identifiers_sql_routes_and_comments_are_rejected(self):
        for source in (
            "struct ProxyUser { user_id: i64 }", "SELECT users.id FROM users",
            '"/api/users"', "loadSubscription()", "subscription_token",
            "quota_bytes", "AccountQuota", "// user traffic", "USER_ID",
            "SubscriptionURL", "monthly_quotas", "userId",
        ):
            with self.subTest(source=source):
                self.assertTrue(boundary.violations("src/transport.rs", source))

    def test_runtime_name_remains_forbidden_in_every_file(self):
        for relative in ("Cargo.toml", "README.md", "src/system/windows.rs"):
            for source in ("singbox", "SING-BOX"):
                self.assertTrue(boundary.violations(relative, source))

    def test_native_exceptions_are_expression_and_file_scoped(self):
        fixtures = {
            "src/system/windows.rs": "[Security.Principal.WindowsIdentity]::GetCurrent().User.Value",
            "src/system/deploy/native/windows.rs": "Get-LocalUser -Name $account; -UserId 'SYSTEM'; USER_RIGHTS",
            "src/system/deploy/native/unix.rs": '"/Users/example"; "UserShell"; <key>UserName</key>',
            "tests/usage_bounds.rs": '.pragma_update(None, "user_version", 1); "PRAGMA user_version"',
        }
        for relative, source in fixtures.items():
            with self.subTest(relative=relative):
                self.assertFalse(boundary.violations(relative, source))
                self.assertTrue(boundary.violations("src/transport.rs", source))
                self.assertTrue(boundary.violations(relative, source + "; user_id=1"))

    def test_api_names_and_existing_account_terms_do_not_match_business(self):
        self.assertFalse(boundary.violations("src/config.rs", "url.username(); account_name; usershow; useradd"))

    def test_new_file_is_scanned_and_current_core_passes(self):
        self.assertTrue(boundary.check(SCRIPT.parents[1] / "crates/agent-core"))
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "new.sql").write_text("SELECT quota FROM state")
            self.assertFalse(boundary.check(root))


if __name__ == "__main__":
    unittest.main()
