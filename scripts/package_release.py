"""Smoke-test and package a native canvas-native release build."""

import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import tomllib
import zipfile


ROOT = Path(__file__).resolve().parents[1]


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", help="Rust target triple already built in release mode")
    args = parser.parse_args()

    package = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]
    version = package["version"]
    name = package["name"]
    binary_name = name + (".exe" if "windows" in args.target else "")
    binary = ROOT / "target" / args.target / "release" / binary_name
    if not binary.is_file():
        parser.error(f"missing release binary: {binary}")

    if "linux-musl" in args.target:
        headers = subprocess.run(
            ["readelf", "-l", str(binary)], check=True, capture_output=True, text=True
        ).stdout
        if "INTERP" in headers:
            raise RuntimeError(f"{binary} is dynamically linked")

    with tempfile.TemporaryDirectory() as directory:
        scratch = Path(directory)
        canvas = scratch / "smoke.canvas"
        canvas.write_text(
            json.dumps(
                {
                    "nodes": [
                        {
                            "id": "a",
                            "type": "text",
                            "x": 0,
                            "y": 0,
                            "width": 240,
                            "height": 120,
                            "text": "Hello Canvas",
                        }
                    ],
                    "edges": [],
                }
            )
        )
        svg = scratch / "smoke.svg"
        png = scratch / "smoke.png"
        for output in (svg, png):
            subprocess.run([str(binary), str(canvas), str(output), "--agent"], check=True)
        if b"<svg" not in svg.read_bytes() or not png.read_bytes().startswith(b"\x89PNG\r\n\x1a\n"):
            raise RuntimeError("smoke render did not produce SVG and PNG")

    dist = ROOT / "dist"
    dist.mkdir(exist_ok=True)
    stem = f"{name}-v{version}-{args.target}"
    archive = dist / (stem + (".zip" if "windows" in args.target else ".tar.gz"))
    files = (
        (binary, binary_name),
        (ROOT / "LICENSE", "LICENSE"),
        (ROOT / "assets" / "Inter-LICENSE.txt", "Inter-LICENSE.txt"),
    )
    if "windows" in args.target:
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as output:
            for source, archived_name in files:
                output.write(source, archived_name)
    else:
        with tarfile.open(archive, "w:gz") as output:
            for source, archived_name in files:
                output.add(source, archived_name)

    with archive.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    checksum = archive.with_name(archive.name + ".sha256")
    checksum.write_text(f"{digest}  {archive.name}\n")
    print(f"{archive} ({archive.stat().st_size} bytes)")
    print(checksum)


if __name__ == "__main__":
    main()
