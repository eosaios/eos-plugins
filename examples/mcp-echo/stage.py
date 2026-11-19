#!/usr/bin/env python3
"""Stage the current machine's native executable; never builds a portable binary."""
import os
from pathlib import Path
import shutil

root = Path(__file__).resolve().parent
source = root / "target/release" / ("echo-server.exe" if os.name == "nt" else "echo-server")
destination = root / "package/native/echo.exe"
destination.parent.mkdir(parents=True, exist_ok=True)
shutil.copy2(source, destination)
destination.chmod(0o755)
print(f"Staged {destination}; declare the actual build OS/architecture when uploading")
