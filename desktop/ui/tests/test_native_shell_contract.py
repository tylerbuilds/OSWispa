import json
import re
import unittest
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[3]
UI_ROOT = REPO_ROOT / "desktop" / "ui"
TAURI_ROOT = REPO_ROOT / "desktop" / "src-tauri"


class NativeShellContractTests(unittest.TestCase):
    def test_native_adapter_loads_between_bridge_and_application(self) -> None:
        for name in ("index.html", "signal.html", "history.html"):
            source = (UI_ROOT / name).read_text(encoding="utf-8")
            self.assertLess(source.index('src="bridge.js"'), source.index('src="tauri-adapter.js"'))
            self.assertLess(source.index('src="tauri-adapter.js"'), source.index('src="app.js"'))

    def test_adapter_projects_only_lifecycle_state(self) -> None:
        source = (UI_ROOT / "tauri-adapter.js").read_text(encoding="utf-8")
        self.assertIn("event?.payload?.state", source)
        self.assertIn("Object.freeze({ state })", source)
        self.assertNotRegex(
            source,
            r"fetch\s*\(|XMLHttpRequest|WebSocket|sendBeacon|\.core\.invoke|\.fs\b|\.shell\b",
        )

    def test_tauri_config_is_local_strict_and_stably_identified(self) -> None:
        config = json.loads((TAURI_ROOT / "tauri.conf.json").read_text(encoding="utf-8"))
        self.assertEqual(config["identifier"], "com.tylerbuilds.oswispa")
        self.assertFalse(config["bundle"]["active"])
        self.assertFalse(config["bundle"]["createUpdaterArtifacts"])
        self.assertEqual(config["app"]["security"]["capabilities"], ["signal-lifecycle"])

        windows = {window["label"]: window for window in config["app"]["windows"]}
        self.assertEqual(set(windows), {"settings", "signal", "history"})
        self.assertFalse(windows["signal"]["focus"])
        self.assertFalse(windows["signal"]["focusable"])
        self.assertTrue(windows["signal"]["skipTaskbar"])
        for window in windows.values():
            self.assertNotRegex(window["url"], r"^https?://")

        csp = config["app"]["security"]["csp"]
        for directive in (
            "default-src 'none'",
            "script-src 'self'",
            "style-src 'self'",
            "object-src 'none'",
            "base-uri 'none'",
            "form-action 'none'",
            "frame-ancestors 'none'",
        ):
            self.assertIn(directive, csp)

    def test_signal_has_the_only_webview_capability(self) -> None:
        capability = json.loads(
            (TAURI_ROOT / "capabilities" / "signal-lifecycle.json").read_text(encoding="utf-8")
        )
        self.assertEqual(capability["windows"], ["signal"])
        self.assertEqual(
            capability["permissions"],
            ["core:event:allow-listen", "core:event:allow-unlisten"],
        )

    def test_single_instance_is_the_only_plugin_and_initialises_first(self) -> None:
        manifest = (TAURI_ROOT / "Cargo.toml").read_text(encoding="utf-8")
        plugin_dependencies = set(re.findall(r"(?m)^(tauri-plugin-[\w-]+)\s*=", manifest))
        self.assertEqual(plugin_dependencies, {"tauri-plugin-single-instance"})

        source = (TAURI_ROOT / "src" / "lib.rs").read_text(encoding="utf-8")
        singleton = source.index(".plugin(tauri_plugin_single_instance::init")
        setup = source.index(".setup(move |app|")
        engine = source.index("spawn_engine(app.handle()")
        self.assertLess(singleton, setup)
        self.assertLess(setup, engine)
        self.assertNotIn("invoke_handler", source)

    def test_macos_microphone_metadata_contains_no_identity_or_secret(self) -> None:
        info = (TAURI_ROOT / "Info.plist").read_text(encoding="utf-8")
        entitlements = (TAURI_ROOT / "Entitlements.plist").read_text(encoding="utf-8")
        config = (TAURI_ROOT / "tauri.conf.json").read_text(encoding="utf-8")
        self.assertIn("NSMicrophoneUsageDescription", info)
        self.assertIn("com.apple.security.device.audio-input", entitlements)
        self.assertIn('"entitlements": "./Entitlements.plist"', config)
        self.assertNotRegex(info + entitlements + config, r"(?i)certificate|thumbprint|private.?key")


if __name__ == "__main__":
    unittest.main()
