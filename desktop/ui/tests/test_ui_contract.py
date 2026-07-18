from __future__ import annotations

import re
import unittest
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit


UI_ROOT = Path(__file__).resolve().parents[1]
HTML_DOCUMENTS = tuple(UI_ROOT / name for name in ("index.html", "signal.html", "history.html"))
TEXT_ASSETS = tuple(UI_ROOT.glob("*.html")) + tuple(UI_ROOT.glob("*.css")) + tuple(UI_ROOT.glob("*.js")) + (UI_ROOT / "README.md",)

EXPECTED_SIGNAL_STATES = {
    "booting",
    "ready",
    "arming",
    "listening",
    "processing",
    "delivering",
    "inserted",
    "copied",
    "cancelled",
    "needs_attention",
}
EXPECTED_SETTINGS = {
    "general",
    "shortcut",
    "microphone",
    "models",
    "language",
    "dictionary",
    "privacy",
    "about",
}
EXPECTED_READY_STEPS = {
    "privacy_model",
    "microphone",
    "shortcut_insertion",
    "sample_receipt",
}
FORBIDDEN_TERMS = {
    "fluid" + "voice",
    "wispr" + " flow",
    "super" + "whisper",
    "voice" + "ink",
    "mac" + "whisper",
    "not" + "ch",
}


class ContractParser(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.h1_count = 0
        self.main_count = 0
        self.lang = None
        self.ids: list[str] = []
        self.labels_for: set[str] = set()
        self.label_depth = 0
        self.controls: list[tuple[str, str | None, bool, str]] = []
        self.resources: list[str] = []
        self.fragments: list[str] = []
        self.interactive_tags: list[str] = []
        self.inline_handlers: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        attributes = dict(attrs)
        if tag == "html":
            self.lang = attributes.get("lang")
        if tag == "h1":
            self.h1_count += 1
        if tag == "main":
            self.main_count += 1
        if element_id := attributes.get("id"):
            self.ids.append(element_id)
        if tag == "label":
            self.label_depth += 1
            if label_for := attributes.get("for"):
                self.labels_for.add(label_for)
        if tag in {"input", "select", "textarea"}:
            self.controls.append((tag, attributes.get("id"), self.label_depth > 0, attributes.get("type", "")))
        if tag in {"button", "a", "input", "select", "textarea"}:
            self.interactive_tags.append(tag)
        if tag == "script" and attributes.get("src"):
            self.resources.append(attributes["src"])
        if tag == "link" and attributes.get("href"):
            self.resources.append(attributes["href"])
        if tag in {"img", "source"} and attributes.get("src"):
            self.resources.append(attributes["src"])
        if tag == "a" and attributes.get("href"):
            self.fragments.append(attributes["href"])
        self.inline_handlers.extend(name for name, _ in attrs if name.lower().startswith("on"))

    def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        self.handle_starttag(tag, attrs)
        if tag == "label":
            self.label_depth -= 1

    def handle_endtag(self, tag: str) -> None:
        if tag == "label":
            self.label_depth -= 1


def parse_document(path: Path) -> ContractParser:
    parser = ContractParser()
    parser.feed(path.read_text(encoding="utf-8"))
    return parser


class DesktopUiContractTests(unittest.TestCase):
    def test_main_documents_have_one_h1_and_language(self) -> None:
        for document in HTML_DOCUMENTS:
            with self.subTest(document=document.name):
                parser = parse_document(document)
                self.assertEqual(parser.lang, "en")
                self.assertEqual(parser.main_count, 1)
                self.assertEqual(parser.h1_count, 1)
                self.assertEqual(len(parser.ids), len(set(parser.ids)), "duplicate id")
                self.assertFalse(parser.inline_handlers, "inline event handlers are not allowed")

    def test_resources_are_local_and_present(self) -> None:
        for document in HTML_DOCUMENTS:
            parser = parse_document(document)
            for resource in parser.resources:
                with self.subTest(document=document.name, resource=resource):
                    parsed = urlsplit(resource)
                    self.assertFalse(parsed.scheme or parsed.netloc or resource.startswith("//"))
                    self.assertTrue((document.parent / parsed.path).is_file())

    def test_local_fragments_resolve(self) -> None:
        for document in HTML_DOCUMENTS:
            parser = parse_document(document)
            for href in parser.fragments:
                parsed = urlsplit(href)
                if parsed.scheme or parsed.netloc or not parsed.fragment:
                    continue
                target = document if not parsed.path else document.parent / parsed.path
                with self.subTest(document=document.name, href=href):
                    self.assertTrue(target.is_file())
                    self.assertIn(parsed.fragment, parse_document(target).ids)

    def test_form_controls_have_labels(self) -> None:
        for document in (UI_ROOT / "index.html", UI_ROOT / "history.html"):
            parser = parse_document(document)
            for tag, control_id, nested, control_type in parser.controls:
                if control_type == "hidden":
                    continue
                with self.subTest(document=document.name, tag=tag, control_id=control_id):
                    self.assertTrue(nested or (control_id and control_id in parser.labels_for))

    def test_signal_is_non_interactive_and_transcript_free(self) -> None:
        signal = UI_ROOT / "signal.html"
        parser = parse_document(signal)
        self.assertFalse(parser.interactive_tags)
        self.assertNotIn("transcript", signal.read_text(encoding="utf-8").lower())

    def test_expected_surface_identifiers_exist(self) -> None:
        app_source = (UI_ROOT / "app.js").read_text(encoding="utf-8")
        index_source = (UI_ROOT / "index.html").read_text(encoding="utf-8")
        for state in EXPECTED_SIGNAL_STATES:
            self.assertRegex(app_source, rf"(?m)^\s{{4}}{re.escape(state)}:", state)
        for setting in EXPECTED_SETTINGS:
            self.assertIn(f'data-settings-tab="{setting}"', index_source)
            self.assertIn(f'data-settings-panel="{setting}"', index_source)
        for step in EXPECTED_READY_STEPS:
            self.assertIn(f'data-ready-step="{step}"', index_source)

    def test_bridge_contract_is_narrow_and_complete(self) -> None:
        bridge_source = (UI_ROOT / "bridge.js").read_text(encoding="utf-8")
        for method in ("invoke", "listen", "dispose"):
            self.assertIn(f'"{method}"', bridge_source)
        for command in ("read_bootstrap", "save_settings", "run_ready_check", "copy_history_entry", "clear_history"):
            self.assertIn(f'"{command}"', bridge_source)
        for event in ("lifecycle", "ready_check_changed", "settings_changed", "history_changed"):
            self.assertIn(f'"{event}"', bridge_source)

    def test_no_networking_analytics_or_remote_css(self) -> None:
        css_source = (UI_ROOT / "styles.css").read_text(encoding="utf-8").lower()
        javascript = "\n".join(path.read_text(encoding="utf-8") for path in UI_ROOT.glob("*.js"))
        self.assertNotRegex(css_source, r"url\s*\(\s*['\"]?https?://")
        for primitive in ("fetch(", "xmlhttprequest", "websocket", "sendbeacon", "google-analytics", "segment.com"):
            self.assertNotIn(primitive, javascript.lower())

    def test_no_competitor_terms_in_bundled_ui(self) -> None:
        for path in TEXT_ASSETS:
            source = path.read_text(encoding="utf-8").lower()
            for term in FORBIDDEN_TERMS:
                with self.subTest(path=path.name, term=term):
                    self.assertNotIn(term, source)

    def test_public_product_name_replaces_legacy_name(self) -> None:
        for path in tuple(UI_ROOT.glob("*.html")) + (UI_ROOT / "README.md",):
            source = path.read_text(encoding="utf-8")
            with self.subTest(path=path.name):
                self.assertIn("MorpheOS Voice", source)
                self.assertNotIn("OSWispa", source)


if __name__ == "__main__":
    unittest.main()
