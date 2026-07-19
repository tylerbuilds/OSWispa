#!/usr/bin/env python3
"""Validate the legacy-URL hand-off to MorpheOS Voice."""

from __future__ import annotations

import sys
from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urlsplit


ROOT = Path(__file__).resolve().parents[1]
WEBSITE = ROOT / "website"


class SiteParser(HTMLParser):
    def __init__(self, path: Path) -> None:
        super().__init__(convert_charrefs=True)
        self.path = path
        self.ids: set[str] = set()
        self.duplicate_ids: set[str] = set()
        self.references: list[tuple[str, str]] = []
        self.h1_count = 0
        self.missing_alt: list[str] = []
        self.unsafe_blank_targets: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        values = {name: value or "" for name, value in attrs}
        element_id = values.get("id")
        if element_id:
            if element_id in self.ids:
                self.duplicate_ids.add(element_id)
            self.ids.add(element_id)

        if tag == "h1":
            self.h1_count += 1

        if tag == "a" and values.get("href"):
            self.references.append(("href", values["href"]))
            if values.get("target") == "_blank":
                rel = set(values.get("rel", "").split())
                if "noopener" not in rel:
                    self.unsafe_blank_targets.append(values["href"])

        if tag in {"img", "script", "source"} and values.get("src"):
            self.references.append(("src", values["src"]))

        if tag == "link" and values.get("href"):
            self.references.append(("href", values["href"]))

        if tag == "img" and "alt" not in values:
            self.missing_alt.append(values.get("src", "<unknown>"))


def parse_html(path: Path) -> SiteParser:
    parser = SiteParser(path)
    parser.feed(path.read_text(encoding="utf-8"))
    return parser


def local_target(source: Path, reference: str) -> tuple[Path, str] | None:
    parsed = urlsplit(reference)
    if parsed.scheme or parsed.netloc or reference.startswith("//"):
        return None

    path_text = unquote(parsed.path)
    if path_text.startswith("/"):
        return None

    target = source if not path_text else (source.parent / path_text).resolve()
    if path_text.endswith("/"):
        target /= "index.html"
    return target, unquote(parsed.fragment)


def main() -> int:
    errors: list[str] = []
    html_files = sorted(WEBSITE.glob("*.html"))
    parsers = {path.resolve(): parse_html(path) for path in html_files}

    for path, parser in parsers.items():
        relative = path.relative_to(ROOT)
        if parser.h1_count != 1:
            errors.append(f"{relative}: expected exactly one h1, found {parser.h1_count}")
        if parser.duplicate_ids:
            errors.append(
                f"{relative}: duplicate ids: {', '.join(sorted(parser.duplicate_ids))}"
            )
        for source in parser.missing_alt:
            errors.append(f"{relative}: image is missing alt text: {source}")
        for reference in parser.unsafe_blank_targets:
            errors.append(f"{relative}: target=_blank link needs rel=noopener: {reference}")

        for kind, reference in parser.references:
            target_info = local_target(path, reference)
            if target_info is None:
                continue
            target, fragment = target_info
            if not target.exists():
                errors.append(f"{relative}: broken local {kind}: {reference}")
                continue
            if fragment and target.suffix.lower() == ".html":
                target_parser = parsers.get(target.resolve()) or parse_html(target)
                if fragment not in target_parser.ids:
                    errors.append(f"{relative}: missing fragment target: {reference}")

    index = (WEBSITE / "index.html").read_text(encoding="utf-8")

    claim_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(WEBSITE.iterdir())
        if path.is_file() and path.suffix.lower() in {".html", ".md"}
    )
    website_text = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted(WEBSITE.iterdir())
        if path.is_file() and path.suffix.lower() in {".html", ".css", ".js", ".md"}
    )
    forbidden_claims = {
        "0.7s": "unsupported latency claim",
        "Updated March 13, 2026": "stale release date",
        "turns your voice into text instantly": "unsupported instant-output claim",
        "Everything runs on your machine": "claim contradicts optional remote mode",
        "No network dependency": "claim ignores the first model download",
        "sudo systemctl enable ydotool": "incorrect system-level ydotool guidance",
    }
    forbidden_resources = {
        "googletagmanager.com": "third-party analytics loader",
        "google-analytics.com": "third-party analytics loader",
        "fonts.googleapis.com": "remote font loader",
        "fonts.gstatic.com": "remote font loader",
        "formsubmit.co": "third-party form handler",
        "oswispa.tylerbuilds.com": "retired product domain",
    }
    folded_claims = claim_text.casefold()
    for phrase, reason in forbidden_claims.items():
        if phrase.casefold() in folded_claims:
            errors.append(f"website: remove {reason}: {phrase!r}")
    folded_website = website_text.casefold()
    for phrase, reason in forbidden_resources.items():
        if phrase.casefold() in folded_website:
            errors.append(f"website: remove {reason}: {phrase!r}")

    required = [
        "OSWispa is now MorpheOS Voice.",
        "MorpheOS Voice",
        "https://morpheos.net/voice/",
        "Visit MorpheOS Voice",
        "Legacy OSWispa address",
    ]
    for phrase in required:
        if phrase.casefold() not in index.casefold():
            errors.append(f"website/index.html: missing transition marker {phrase!r}")

    if 'name="robots" content="noindex,follow"' not in index:
        errors.append("website/index.html: transition page must be noindex,follow")
    if "releases/latest/download" in index:
        errors.append("website/index.html: legacy landing page must not offer direct downloads")

    if errors:
        print("Website validation failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"Website transition validation passed for {len(html_files)} HTML pages.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
