#!/usr/bin/env python3
"""Synchronize generated wire types and reproducible Rust source dependencies."""
import argparse
import json
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[1]
parser = argparse.ArgumentParser()
parser.add_argument("--check", action="store_true")
args = parser.parse_args()
subprocess.run(["python3", str(root / "scripts/generate.py"), *(["--check"] if args.check else [])], check=True)
for repo in ("eos-platform", "eos-core-rs"):
    subprocess.run(["python3", str(root / "scripts/vendor.py"), "--destination", str(root.parent / repo / "vendor/eos-plugins"), *(["--check"] if args.check else [])], check=True)
source = root / "sdk/typescript/src/protocol.ts"
for target in (root.parent / "eos-app-src/frontend/src/plugin-platform-protocol.ts", root.parent / "eos-platform/core/web/admin/src/plugin-platform-protocol.ts"):
    if args.check:
        if target.read_bytes() != source.read_bytes():
            raise SystemExit(f"stale generated types: {target}")
    else:
        target.write_bytes(source.read_bytes())

# Go's existing RPC facade keeps its historical methods. Its new market extension
# is generated from the Core export, never maintained as a second string list.
core_schema = json.loads((root.parent / "eos-core-rs/crates/eos-core-protocol/schema.json").read_text())
market_methods = [row["name"] for row in core_schema["methods"] if row["name"].startswith("plugin/market/")]
lines = ["// Code generated from eos-core-protocol/schema.json; DO NOT EDIT.", "package jsonrpc", "const ("]
def go_name(method):
    return "Method" + "".join(part[:1].upper()+part[1:] for part in method.replace("/", "_").replace("-", "_").split("_"))
lines += [f"{go_name(method)} = {json.dumps(method)}" for method in market_methods]
lines += [")", "func pluginMarketMethods() []string { return []string{"]
lines += [go_name(method)+"," for method in market_methods]
lines += ["} }"]
wire = subprocess.run(["gofmt"], input="\n".join(lines)+"\n",text=True,capture_output=True,check=True).stdout
target = root.parent / "eos-cli/pkg/protocol/jsonrpc/plugin_market_methods_gen.go"
if args.check:
    if target.read_text()!=wire: raise SystemExit("stale Go market facade")
else: target.write_text(wire)

wire_source=root / "sdk/go/protocol.go"
wire_target=root.parent / "eos-cli/pkg/pluginprotocol/protocol_gen.go"
if args.check:
    if wire_target.read_bytes()!=wire_source.read_bytes():raise SystemExit("stale Go platform DTOs")
else:
    wire_target.parent.mkdir(parents=True,exist_ok=True);wire_target.write_bytes(wire_source.read_bytes())
