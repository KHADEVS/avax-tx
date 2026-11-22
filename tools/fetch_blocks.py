"""Fetch P-Chain blocks from a public avalanchego API, both as raw hex and
as the node's own JSON decoding, to check this crate's decoder against the
node's field by field (tests/mainnet_blocks.rs).

Usage: python -I tools/fetch_blocks.py <out.jsonl> <height> [<height> ...]
       python -I tools/fetch_blocks.py <out.jsonl> --sample <n>
Appends one JSON object per line: {"height", "hex", "json"}.
"""
import hashlib
import json
import sys
import time
import urllib.request

API = "https://api.avax.network/ext/bc/P"


def call(method, params):
    body = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).encode()
    req = urllib.request.Request(API, body, {"content-type": "application/json", "user-agent": "avax-tx-conformance"})
    for attempt in range(5):
        try:
            with urllib.request.urlopen(req, timeout=30) as resp:
                out = json.load(resp)
            if "error" in out:
                raise RuntimeError(out["error"])
            return out["result"]
        except Exception as e:  # rate limits and timeouts: back off and retry
            if attempt == 4:
                raise
            time.sleep(2 * (attempt + 1))


def fetch(height):
    h = str(height)
    raw = call("platform.getBlockByHeight", {"height": h, "encoding": "hex"})["block"]
    decoded = call("platform.getBlockByHeight", {"height": h, "encoding": "json"})["block"]
    # formatting.Hex: the bytes, then the last 4 bytes of their SHA-256.
    data = bytes.fromhex(raw[2:])
    body, check = data[:-4], data[-4:]
    assert raw.startswith("0x") and hashlib.sha256(body).digest()[-4:] == check, f"checksum at {height}"
    return {"height": height, "hex": body.hex(), "json": decoded}


def main(out, *args):
    if args[0] == "--sample":
        top = int(call("platform.getHeight", {})["height"])
        n = int(args[1])
        heights = [1 + (top - 1) * i // (n - 1) for i in range(n)]
    else:
        heights = [int(a) for a in args]
    with open(out, "a", encoding="utf8") as f:
        for h in heights:
            f.write(json.dumps(fetch(h)) + "\n")
            time.sleep(0.15)
    print(f"{len(heights)} blocks appended to {out}")


main(*sys.argv[1:])
