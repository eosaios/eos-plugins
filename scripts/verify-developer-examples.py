#!/usr/bin/env python3
"""Reproduce the developer guide's offline loop using synthetic, local data only."""
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args, cwd=ROOT, capture=False):
    result = subprocess.run(args, cwd=cwd, check=True, text=True,
                            stdout=subprocess.PIPE if capture else None)
    return result.stdout if capture else None


run("cargo", "build", "--locked", "-p", "eos-plugin-cli")
cli = ROOT / "target/debug" / ("eos-plugin.exe" if os.name == "nt" else "eos-plugin")
panel = ROOT / "examples/notes-panel"
npm = "npm.cmd" if os.name == "nt" else "npm"
run(npm, "ci", cwd=panel)
run(npm, "run", "build", cwd=panel)
run("node", "smoke.mjs", cwd=panel)
run("cargo", "build", "--locked", "--manifest-path", "examples/mcp-echo/Cargo.toml", "--release")
run("python3", "examples/mcp-echo/stage.py")
run("python3", "examples/mcp-echo/smoke.py")

sources = {
    "hello-skills": "examples/hello-skills",
    "notes-panel": "examples/notes-panel/package",
    "mcp-echo": "examples/mcp-echo/package",
    "wasm-ack": "examples/wasm-ack",
}
with tempfile.TemporaryDirectory(prefix="eos-plugin-developer-") as tmp:
    directory = Path(tmp)
    skills = json.loads((ROOT / "examples/hosts/skills.json").read_text())
    full = json.loads(json.dumps(skills))
    full["executors"] += [
        {"type": "webview/v1", "abi": "1.0.0", "profiles": []},
        {"type": "wasm-component/v1", "abi": "1.0.0", "profiles": []},
        {"type": "mcp-stdio/v1", "abi": "1.0.0", "profiles": ["trusted-process"]},
    ]
    full["capabilities"].append({"id": "eos.ui.panel", "version": "1.0.0"})
    full["uiSlots"] = [{"id": "plugin-panel", "kind": "panel", "version": "1.0.0"}]
    host = directory / "synthetic-host.json"
    host.write_text(json.dumps(full))
    for name, source in sources.items():
        package = directory / (name + ".eos-plugin")
        run(str(cli), "pack", str(ROOT / source), str(package))
        manifest = json.loads(run(str(cli), "validate", str(package), capture=True))
        assert manifest["plugin"]["id"] == "tutorial." + name
        result = json.loads(run(str(cli), "compatibility", str(package), str(host), capture=True))
        expected = "authorization-required" if name == "mcp-echo" else "compatible"
        assert result["status"] == expected and not result["issues"], result
        if name != "hello-skills":
            rejected = json.loads(run(str(cli), "compatibility", str(package),
                str(ROOT / "examples/hosts/skills.json"), capture=True))
            assert rejected["status"] == "incompatible" and rejected["issues"], rejected
        print(f"{name}: pack/validate + compatible/incompatible paths passed")
print("All offline developer examples passed; no market upload or real Agent acceptance was performed")
