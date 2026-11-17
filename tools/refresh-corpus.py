#!/usr/bin/env python3
"""Refresh explicitly, or verify the pinned offline corpus without network I/O."""
import argparse
import hashlib
import json
from pathlib import Path
from urllib.request import urlopen

ROOT = Path(__file__).resolve().parents[1]
DESTINATION = ROOT / "tests/fixtures/corpus"
SOURCES = [
    ("commonmark.json", "https://spec.commonmark.org/0.31.2/spec.json", "d431b29d97b6f73e69d547109cf5081578fac931e72afe95639ebe766c1b2a20", "CC-BY-SA-4.0", None),
    ("COMMONMARK-LICENSE.txt", "https://raw.githubusercontent.com/commonmark/commonmark-spec/0.31.2/LICENSE", "3e4806ba6f20073e8ce40da5a0c4b59f7f44287965f538e195a4d734d833557b", "CC-BY-SA-4.0; BSD-2-Clause; MIT", None),
    ("regress-README.md", "https://github.com/ridiculousfish/regress/tree/v0.11.1", "ef3875d93b30214d789ee9ff1f35ad3e908f290092db703af17d6cdff1ae7458", "MIT OR Apache-2.0", "vendor/regress/README.md"),
    ("REGRESS-LICENSE-MIT.txt", "https://github.com/ridiculousfish/regress/tree/v0.11.1", "3f8a05fc8617c5ce569fa5a4f2301cd847fe00fa011bdaa8f353fd55a2691a54", "MIT", "vendor/regress/LICENSE-MIT"),
    ("REGRESS-LICENSE-APACHE.txt", "https://github.com/ridiculousfish/regress/tree/v0.11.1", "e36d55837170195cdf934aa6a48c0ad6efba1bde3f5b4a1b9259c171d5aeb647", "Apache-2.0", "vendor/regress/LICENSE-APACHE"),
    ("markdownlint-README.md", "https://www.npmjs.com/package/markdownlint/v/0.42.0", "2a37707f9fa0543e6cffbc3281dfad55678f1a16b361e4506a88e201b2a9b1bf", "MIT", "tools/parser-oracle/node_modules/markdownlint/README.md"),
    ("MARKDOWNLINT-LICENSE.txt", "https://www.npmjs.com/package/markdownlint/v/0.42.0", "cdc3c1fb00ee139442c915e98afac039223569a00527e9f7aefc54f92019aec4", "MIT", "tools/parser-oracle/node_modules/markdownlint/LICENSE"),
]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fetch", action="store_true", help="explicitly download/copy the pinned sources")
    options = parser.parse_args()
    manifest = {"commonmark_examples": 652, "attribution": "CommonMark specification copyright 2014-16 John MacFarlane; examples are unmodified CC-BY-SA 4.0 test data, not binary content.", "sources": []}
    for name, url, digest, license_name, local in SOURCES:
        destination = DESTINATION / name
        if options.fetch:
            data = (ROOT / local).read_bytes() if local else urlopen(url, timeout=30).read()
        else:
            data = destination.read_bytes()
        if hashlib.sha256(data).hexdigest() != digest:
            raise SystemExit(f"source hash mismatch: {name}")
        if options.fetch:
            DESTINATION.mkdir(parents=True, exist_ok=True)
            destination.write_bytes(data)
        manifest["sources"].append({"path": name, "url": url, "sha256": digest, "license": license_name, "bytes": len(data)})
    assert len(json.loads((DESTINATION / "commonmark.json").read_text(encoding="utf-8"))) == 652
    serialized = json.dumps(manifest, indent=2) + "\n"
    if options.fetch:
        (DESTINATION / "manifest.json").write_text(serialized, encoding="utf-8")
    elif (DESTINATION / "manifest.json").read_text(encoding="utf-8") != serialized:
        raise SystemExit("corpus manifest does not match pinned sources")
    print(f"PASS: {len(SOURCES)} pinned corpus/license files; 652 examples and two real documents")


if __name__ == "__main__":
    main()
