"""Extract the expected-bytes vectors from avalanchego's P-Chain tests.

avalanchego's serialization tests build a transaction or block, marshal it
and compare the result with a byte literal written out field by field. This
script copies those literals, unchanged, into a JSON file with their origin,
so this crate can be checked against the same bytes without running Go.

Usage: python -I tools/extract_vectors.py <avalanchego checkout> <out.json>
The checkout must be at a release tag; its commit is recorded.
avalanchego is BSD-3-Clause; see vectors/NOTICE.
"""
import json
import re
import subprocess
import sys
from pathlib import Path

START = re.compile(r"^\s*(\w+)\s*(?::=|=|:)\s*\[\]byte\{\s*$")
FUNC = re.compile(r"^func (Test\w+)\(")
NAME = re.compile(r'^\s*name:\s*"([^"]*)"')
HEXBYTE = re.compile(r"0x[0-9a-fA-F]{2}")


def wanted(var):
    return var.endswith("Bytes") or var == "bytes"


def extract(path):
    lines = path.read_text(encoding="utf8").splitlines()
    out, test, case, i = [], None, None, 0
    while i < len(lines):
        line = lines[i]
        m = FUNC.match(line)
        if m:
            test, case = m.group(1), None
        m = NAME.match(line)
        if m:
            case = m.group(1)
        m = START.match(line)
        if m and wanted(m.group(1)):
            var, first, depth, body = m.group(1), i + 1, 1, []
            i += 1
            while depth:
                code = lines[i].split("//", 1)[0]
                depth += code.count("{") - code.count("}")
                body.append(code if depth else code[: code.rfind("}")])
                i += 1
            text = " ".join(body)
            rest = HEXBYTE.sub("", text).replace(",", "").strip()
            if rest:
                raise SystemExit(f"{path.name}:{first}: non-literal content in {var}: {rest[:60]!r}")
            data = bytes(int(b, 16) for b in HEXBYTE.findall(text))
            out.append({"test": test, "case": case, "var": var, "file": path.name, "line": first, "hex": data.hex()})
            continue
        i += 1
    return out


def main(root, dest):
    root = Path(root)
    commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    tag = subprocess.run(["git", "-C", str(root), "describe", "--tags", "--exact-match", "--match", "v[0-9]*"], capture_output=True, text=True).stdout.strip()
    pkg = root / "vms" / "platformvm" / "platform"
    vectors = []
    for f in sorted(pkg.glob("*_test.go")):
        vectors.extend(extract(f))
    doc = {
        "source": "https://github.com/ava-labs/avalanchego",
        "tag": tag,
        "commit": commit,
        "path": "vms/platformvm/platform",
        "license": "BSD-3-Clause (see vectors/NOTICE)",
        "vectors": vectors,
    }
    Path(dest).write_text(json.dumps(doc, indent=1) + "\n", encoding="utf8")
    print(f"{len(vectors)} vectors from {tag or commit[:12]}")


main(*sys.argv[1:])
