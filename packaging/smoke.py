"""Verify an extracted portable archive in a fresh app/project root, without Rust on PATH."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import zipfile
from preview import safe_extract


def verify_checksums(root):
    manifest = root / "SHA256SUMS"
    expected = set()
    for line in manifest.read_text(encoding="utf-8").splitlines():
        digest, name = line.split("  ", 1)
        path = (root / name).resolve()
        if not path.is_relative_to(root.resolve()) or not path.is_file():
            raise ValueError("checksum path escapes package or is missing")
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError(f"checksum mismatch: {name}")
        expected.add(name)
    actual = {p.relative_to(root).as_posix() for p in root.rglob("*")
              if p.is_file() and p != manifest}
    if expected != actual:
        raise ValueError("checksum manifest does not match package files")


def extract(archive, root):
    if archive.name.endswith(".tar.gz"):
        safe_extract(archive, root)
    else:
        with zipfile.ZipFile(archive) as source:
            for item in source.infolist():
                path = (root / item.filename).resolve()
                if not path.is_relative_to(root.resolve()):
                    raise ValueError("ZIP path escapes package")
            source.extractall(root)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("archive", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        raise ValueError("fresh smoke directory required")
    app = args.output / "app"
    app.mkdir(parents=True)
    extract(args.archive, app)
    verify_checksums(app)
    info = json.loads((app / "BUILD-INFO.json").read_text(encoding="utf-8"))
    binary = app / ("pdx.exe" if os.name == "nt" else "pdx")
    if hashlib.sha256(binary.read_bytes()).hexdigest() != info["binary_sha256"]:
        raise ValueError("binary identity mismatch")
    if not (app / "LICENSE").is_file() or not (app / "source/Cargo.lock").is_file():
        raise ValueError("license/corresponding source missing")
    source_info = json.loads((app / "source/SOURCE-INFO.json").read_text(encoding="utf-8"))
    if source_info["commit"] != info["commit"]:
        raise ValueError("corresponding source commit mismatch")
    project = args.output / "確認 原稿"
    env = {**os.environ, "PATH": "", "PDX_STATE_ROOT": str(args.output.resolve() / "state")}

    def call(*arguments, code=0, machine=True):
        result = subprocess.run([str(binary.resolve()), *map(str, arguments)], cwd=args.output,
                                env=env, capture_output=True, timeout=30)
        if result.returncode != code:
            raise AssertionError((arguments, result.returncode, result.stdout, result.stderr))
        return json.loads(result.stdout) if machine else result.stdout.decode("utf-8").strip()

    assert call("--version", machine=False) == f"pdx {info['version']}"
    call("--help", machine=False)
    call("init", project.resolve(), "--json")
    before = {str(p): p.read_bytes() for p in project.rglob("*") if p.is_file()}
    call("check", project.resolve(), "--json")
    plan = call("plan", project.resolve(), "--json")
    assert plan["data"]["template"]["id"] == "default-report"
    assert plan["data"]["build_ready"] is False
    call("init", project.resolve(), "--json", code=4)
    error = call("build", project.resolve(), "--json", code=3)
    assert error["error"]["code"] == "missing_runtime"
    assert before == {str(p): p.read_bytes() for p in project.rglob("*") if p.is_file()}
    print(json.dumps({"schema_version": 1, "version": info["version"], "commit": info["commit"],
                      "archive": args.archive.name, "static_smoke": "passed", "path": "empty",
                      "project_preserved": True, "real_pdf": "not tested by this smoke"}))


if __name__ == "__main__":
    main()
