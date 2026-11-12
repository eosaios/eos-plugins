#!/usr/bin/env python3
"""Check a staged server with the same empty environment and cwd as the host."""
import json
from pathlib import Path
import subprocess

binary = Path(__file__).resolve().parent / "package/native/echo.exe"
requests = [
    {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2024-11-05", "capabilities": {},
        "clientInfo": {"name": "tutorial-smoke", "version": "0.1.0"}}},
    {"jsonrpc": "2.0", "method": "notifications/initialized"},
    {"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}},
    {"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
        "name": "echo", "arguments": {"text": "hello"}}},
    {"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
        "name": "echo", "arguments": {"text": 42}}},
]
result = subprocess.run([str(binary)], input="".join(json.dumps(r)+"\n" for r in requests),
    text=True, capture_output=True, check=True, timeout=10, cwd=binary.parent, env={})
rows = [json.loads(line) for line in result.stdout.splitlines()]
assert [r["id"] for r in rows] == [1, 2, 3, 4], rows
assert rows[0]["result"]["protocolVersion"] == "2024-11-05", rows
assert rows[1]["result"]["tools"][0]["name"] == "echo", rows
assert rows[2]["result"] == {"content": [{"type": "text", "text": "echo:hello"}]}, rows
assert rows[3]["result"].get("isError") is True, rows
print("MCP initialize/tools/list/tools/call and invalid-input checks passed")
