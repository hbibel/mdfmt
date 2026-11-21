#!/usr/bin/env python3
"""Build a curated host-native ZIP and checksum without publishing anything."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import time
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SUPPLEMENTS = {
    ("fast-glob", "1.1.2"): ("licenses/fast-glob-1.1.2.txt", "https://github.com/oxc-project/fast-glob/blob/cd1157ebe949c07c2e08ccf7b96f753dfc08e7bd/LICENSE"),
    ("nodejs-built-in-modules", "1.0.0"): ("licenses/nodejs-built-in-modules-1.0.0.txt", "https://github.com/oxc-project/nodejs-built-in-modules/blob/9117efa38fe088325e8a1036a8ddea7f3546c1ac/LICENSE"),
}


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_input(path):
    if any(part == ".env" or part.startswith(".env.") for part in path.parts):
        raise ValueError("environment files cannot be release inputs")
    resolved = path.resolve()
    if any(part == ".env" or part.startswith(".env.") for part in resolved.parts):
        raise ValueError("environment files cannot be release inputs")
    return path.read_bytes()


def run(arguments, *, environment=None, timeout=600):
    result = subprocess.run(arguments, cwd=ROOT, env=environment, capture_output=True,
                            text=True, encoding="utf-8", timeout=timeout)
    if result.returncode:
        raise RuntimeError(f"command failed ({result.returncode}): {' '.join(map(str, arguments))}\n{result.stderr}")
    return result.stdout.strip()


def tool(*arguments, environment=None, timeout=600):
    return run(["mise", "exec", "--", *arguments], environment=environment, timeout=timeout)


def notices(metadata):
    packages = {package["id"]: package for package in metadata["packages"]}
    nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
    pending = [metadata["resolve"]["root"]]
    selected = set()
    while pending:
        identifier = pending.pop()
        if identifier in selected:
            continue
        selected.add(identifier)
        for dependency in nodes[identifier]["deps"]:
            if any(kind["kind"] != "dev" for kind in dependency["dep_kinds"]):
                pending.append(dependency["pkg"])
    inventory = []
    sections = []
    for package in sorted((packages[key] for key in selected), key=lambda item: (item["name"], item["version"])):
        directory = Path(package["manifest_path"]).parent.resolve()
        files = {child for child in directory.iterdir()
                 if child.name.lower().startswith(("license", "copying", "notice", "copyright")) and child.is_file()}
        if package.get("license_file"):
            files.add(directory / package["license_file"])
        copied = []
        heading = f"{package['name']} {package['version']} ({package.get('license') or 'license-file'})"
        for path in sorted(files):
            if not path.resolve().is_relative_to(directory):
                raise ValueError(f"notice outside package directory: {heading}")
            data = read_input(path)
            copied.append({"file": path.name, "sha256": digest(data)})
            sections.append(f"{heading}\n{path.name}\n\n{data.decode('utf-8')}\n")
        if not copied:
            supplement = SUPPLEMENTS.get((package["name"], package["version"]))
            if not supplement:
                raise ValueError(f"missing packaged license/notice: {heading}")
            local, source = supplement
            data = read_input(ROOT / local)
            copied.append({"file": Path(local).name, "sha256": digest(data), "source": source})
            sections.append(f"{heading}\nSource: {source}\n\n{data.decode('utf-8')}\n")
        inventory.append({"name": package["name"], "version": package["version"], "license": package.get("license"), "notices": copied})
    for name, relative in [("RE2JS 2.8.6", "src/config/RE2JS-LICENSE.txt"), ("Unicode data", "src/md060/UNICODE-LICENSE.txt")]:
        sections.append(f"{name}\n\n{read_input(ROOT / relative).decode('utf-8')}\n")
    sections.append("Rust standard-library notices are retained separately in RUST-LIBRARY-NOTICES.html.\n")
    return "\n".join(sections).encode(), inventory


def source_identity():
    checksum = hashlib.sha256()
    paths = [ROOT / "Cargo.toml", ROOT / "Cargo.lock", *sorted((ROOT / "src").rglob("*.rs"))]
    for path in paths:
        checksum.update(path.relative_to(ROOT).as_posix().encode() + b"\0" + read_input(path))
    return {
        "git_commit": run(["git", "rev-parse", "--verify", "HEAD"], timeout=30),
        "inputs_dirty": bool(run(["git", "status", "--porcelain", "--", "Cargo.toml", "Cargo.lock", "src", "README.md", "LICENSE", "tools/release.py", "licenses"], timeout=30)),
        "rust_and_manifest_sha256": checksum.hexdigest(),
    }


def package(output, build_directory):
    if os.environ.get("RUSTFLAGS") or os.environ.get("CARGO_ENCODED_RUSTFLAGS"):
        raise ValueError("fixed release recipe requires unset RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS")
    build_directory = build_directory.resolve()
    if build_directory == ROOT:
        raise ValueError("build directory cannot be the project root")
    rustc = tool("rustc", "--version")
    verbose = tool("rustc", "-vV")
    host = next(line.removeprefix("host: ") for line in verbose.splitlines() if line.startswith("host: "))
    metadata = json.loads(tool("cargo", "metadata", "--locked", "--format-version", "1", "--filter-platform", host))
    root_package = next(item for item in metadata["packages"] if item["id"] == metadata["resolve"]["root"])
    version = root_package["version"]
    combined_notices, inventory = notices(metadata)
    sysroot = Path(tool("rustc", "--print", "sysroot"))
    rust_notices = read_input(sysroot / "share/doc/rust/COPYRIGHT-library.html")
    cargo_home = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo")).resolve()
    remappings = [(ROOT, "mdfmt"), (cargo_home, "cargo-home"), (build_directory, "mdfmt-build"), (sysroot, "rust-toolchain")]
    flags = [f"--remap-path-prefix={original}={replacement}" for original, replacement in remappings] + ["-C", "strip=debuginfo"]
    environment = {**os.environ, "CARGO_TARGET_DIR": str(build_directory), "CARGO_ENCODED_RUSTFLAGS": "\x1f".join(flags)}
    tool("cargo", "build", "--release", "--locked", "--target", host, environment=environment, timeout=1200)
    executable = "mdfmt.exe" if os.name == "nt" else "mdfmt"
    binary = build_directory / host / "release" / executable
    payload = {
        executable: read_input(binary), "README.md": read_input(ROOT / "README.md"),
        "LICENSE": read_input(ROOT / "LICENSE"), "Cargo.lock": read_input(ROOT / "Cargo.lock"),
        "THIRD-PARTY-NOTICES.txt": combined_notices, "RUST-LIBRARY-NOTICES.html": rust_notices,
    }
    epoch = int(os.environ.get("SOURCE_DATE_EPOCH") or run(["git", "show", "-s", "--format=%ct", "HEAD"], timeout=30))
    timestamp = time.gmtime(max(epoch, 315532800))[:6]
    if timestamp[0] > 2107:
        raise ValueError("SOURCE_DATE_EPOCH is outside ZIP's supported date range")
    info = {
        "version": version, "host": host, "rustc": rustc, "source": source_identity(),
        "source_date_epoch": epoch, "cargo_lock_sha256": digest(payload["Cargo.lock"]),
        "release_recipe_sha256": digest(read_input(Path(__file__))),
        "recipe": "locked host-native release; project/cargo/build/sysroot remapped; debuginfo stripped",
        "dependencies": inventory, "payload_sha256": {name: digest(data) for name, data in sorted(payload.items())},
    }
    payload["BUILD-INFO.json"] = (json.dumps(info, indent=2, sort_keys=True) + "\n").encode()
    forbidden = {str(original).encode() for original, _ in remappings} | {original.as_posix().encode() for original, _ in remappings}
    for name, data in payload.items():
        if any(prefix in data for prefix in forbidden):
            raise ValueError(f"machine-specific build/project path in payload: {name}")
    output.mkdir(parents=True, exist_ok=True)
    stem = f"mdfmt-{version}-{host}"
    archive_path = output / f"{stem}.zip"
    with tempfile.NamedTemporaryFile(prefix=".mdfmt-release-", dir=output, delete=False) as temporary:
        temporary_path = Path(temporary.name)
    try:
        with zipfile.ZipFile(temporary_path, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as archive:
            for name, data in sorted(payload.items()):
                member = zipfile.ZipInfo(f"{stem}/{name}", timestamp)
                member.create_system = 3
                member.external_attr = (stat.S_IFREG | (0o755 if name == executable else 0o644)) << 16
                member.compress_type = zipfile.ZIP_DEFLATED
                archive.writestr(member, data, compresslevel=9)
        checksum = digest(temporary_path.read_bytes())
        temporary_path.replace(archive_path)
    finally:
        temporary_path.unlink(missing_ok=True)
    sidecar = archive_path.with_suffix(".zip.sha256")
    with tempfile.NamedTemporaryFile(prefix=".mdfmt-checksum-", dir=output, delete=False) as temporary:
        checksum_path = Path(temporary.name)
        temporary.write(f"{checksum}  {archive_path.name}\n".encode())
    try:
        checksum_path.replace(sidecar)
    finally:
        checksum_path.unlink(missing_ok=True)
    print(f"PASS: {archive_path.name}; {len(inventory)} locked dependency notices; SHA-256 {checksum}")
    return archive_path


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "dist")
    parser.add_argument("--build-dir", type=Path, default=ROOT / "target/native-release")
    options = parser.parse_args()
    try:
        package(options.output, options.build_dir)
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        parser.exit(1, f"release failed: {error}\n")


if __name__ == "__main__":
    main()
