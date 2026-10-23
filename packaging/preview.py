"""Build portable preview archives from a clean, committed checkout; never publish."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parent.parent


def run(*args):
    result = subprocess.run(args, cwd=ROOT, capture_output=True, timeout=180)
    if result.returncode:
        raise RuntimeError(f"{args[0]} failed: {result.stderr.decode('utf-8', errors='replace')}")
    return result.stdout.decode("utf-8").strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def checksums(root):
    files = sorted(p for p in root.rglob("*") if p.is_file() and p != root / "SHA256SUMS")
    lines = [f"{sha(p)}  {p.relative_to(root).as_posix()}" for p in files]
    (root / "SHA256SUMS").write_text("\n".join(lines) + "\n", encoding="utf-8")


def zip_tree(root, destination):
    with zipfile.ZipFile(destination, "x", compression=zipfile.ZIP_DEFLATED) as archive:
        for path in sorted(root.rglob("*")):
            if path.is_file():
                archive.write(path, path.relative_to(root).as_posix())


def safe_extract(archive, destination):
    with tarfile.open(archive) as source:
        for member in source.getmembers():
            path = Path(member.name)
            if member.issym() or member.islnk() or not (member.isfile() or member.isdir()):
                raise ValueError("source archive contains a link or special file")
            if path.is_absolute() or ".." in path.parts:
                raise ValueError("unsafe source archive path")
        source.extractall(destination, filter="data")


def license_inventory(destination, target):
    metadata = json.loads(run("cargo", "metadata", "--format-version", "1", "--locked", "--offline", "--filter-platform", target))
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    pending = [p["id"] for p in metadata["packages"] if p["name"] == "pdx"]
    seen = set()
    while pending:
        item = pending.pop()
        if item in seen:
            continue
        seen.add(item)
        pending.extend(dep["pkg"] for dep in nodes[item]["deps"]
                       if any(kind["kind"] in (None, "build") for kind in dep["dep_kinds"]))
    overrides = json.loads((ROOT / "packaging/license-overrides.json").read_text(encoding="utf-8"))
    notices = ["Locked build dependencies for this target (including build-time tools).",
               "License expressions are upstream metadata, not a legal opinion.",
               "Dependency source: https://crates.io/crates/<name>/<version>", ""]
    for package in sorted(metadata["packages"], key=lambda p: (p["name"], p["version"])):
        if package["id"] not in seen or not package["source"]:
            continue
        name = f"{package['name']}-{package['version']}"
        base = Path(package["manifest_path"]).parent
        files = [p for p in base.iterdir() if p.is_file() and re.match(r"^(LICENSE|LICENCE|COPYING|COPYRIGHT|NOTICE)", p.name, re.I)]
        if package.get("license_file"):
            extra = (base / package["license_file"]).resolve()
            if not extra.is_relative_to(base.resolve()):
                raise ValueError(f"license_file escapes crate: {name}")
            if extra not in files:
                files.append(extra)
        if not files:
            override = overrides[name]
            vcs = json.loads((base / ".cargo_vcs_info.json").read_text(encoding="utf-8"))
            if vcs["git"]["sha1"] != override["commit"]:
                raise ValueError(f"license override source mismatch: {name}")
            files = [ROOT / "packaging/extra-licenses" / override["file"]]
            if sha(files[0]) != override["sha256"]:
                raise ValueError(f"license override digest mismatch: {name}")
        target_dir = destination / "licenses/crates" / name
        target_dir.mkdir(parents=True)
        for path in files:
            if path.is_symlink() or path.stat().st_size > 1024 * 1024:
                raise ValueError(f"unsafe license file: {name}")
            shutil.copyfile(path, target_dir / path.name)
        notices.append(f"{name}: {package['license']} | https://crates.io/crates/{package['name']}/{package['version']}")
    sysroot = Path(run("rustc", "--print", "sysroot"))
    rust_docs = sysroot / "share/doc/rust"
    if not (rust_docs / "licenses").is_dir():
        raise ValueError("Rust toolchain license texts missing")
    shutil.copytree(rust_docs / "licenses", destination / "licenses/rust")
    for name in ("COPYRIGHT.html", "COPYRIGHT-library.html"):
        shutil.copyfile(rust_docs / name, destination / "licenses/rust" / name)
    (destination / "THIRD-PARTY-NOTICES.txt").write_text("\n".join(notices) + "\n", encoding="utf-8")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", choices=["x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu"], required=True)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--source-asset", action="store_true")
    args = parser.parse_args()
    if run("git", "status", "--porcelain", "--untracked-files=all"):
        raise ValueError("package only a clean committed checkout")
    if args.output.exists():
        raise ValueError("fresh output directory required")
    args.output.mkdir(parents=True)
    assets = args.output / "assets"
    assets.mkdir()
    stage = args.output / "stage"
    stage.mkdir()
    source = stage / "source"
    source.mkdir()
    commit = run("git", "rev-parse", "HEAD")
    run("git", "archive", "--format=tar", f"--output={args.output / 'source.tar'}", commit)
    safe_extract(args.output / "source.tar", source)
    metadata = json.loads(run("cargo", "metadata", "--format-version", "1", "--no-deps", "--locked", "--offline"))
    version = next(p["version"] for p in metadata["packages"] if p["name"] == "pdx")
    if not re.fullmatch(r"3\.0\.0-preview\.[1-9][0-9]*", version):
        raise ValueError("this packager publishes only the reviewed v3 preview series")
    binary_name = "pdx.exe" if "windows" in args.target else "pdx"
    binary = stage / binary_name
    shutil.copyfile(args.binary, binary)
    binary.chmod(0o755)
    actual_version = run(str(binary.resolve()), "--version")
    if actual_version != f"pdx {version}":
        raise ValueError(f"binary/source version mismatch: {actual_version}")
    source_info = {"schema_version": 1, "version": version, "commit": commit,
                   "repository": "https://github.com/Xpotato1024/Pandocker-X",
                   "tree": run("git", "rev-parse", "HEAD^{tree}"), "cargo_lock_sha256": sha(source / "Cargo.lock")}
    write_json(source / "SOURCE-INFO.json", source_info)
    checksums(source)
    if args.source_asset:
        zip_tree(source, assets / f"pandocker-x-v{version}-source.zip")
    shutil.copyfile(source / "LICENSE", stage / "LICENSE")
    shutil.copyfile(source / "packaging/README-FIRST.ja.md", stage / "README-FIRST.ja.md")
    license_inventory(stage, args.target)
    write_json(stage / "BUILD-INFO.json", {**source_info, "target": args.target,
               "binary_sha256": sha(binary), "rustc": run("rustc", "--version"), "runtime_image_included": False})
    checksums(stage)
    suffix = "windows-x86_64-msvc.zip" if "windows" in args.target else "linux-x86_64-gnu.tar.gz"
    package = assets / f"pandocker-x-v{version}-{suffix}"
    if package.suffix == ".zip":
        zip_tree(stage, package)
    else:
        with tarfile.open(package, "x:gz") as archive:
            archive.add(stage, arcname=".")
    (assets / f"SHA256SUMS-{args.target}.txt").write_text(
        "".join(f"{sha(p)}  {p.name}\n" for p in sorted(assets.iterdir()) if p.is_file()), encoding="utf-8")
    print(json.dumps({"version": version, "commit": commit, "archive": str(package)}))


if __name__ == "__main__":
    main()
