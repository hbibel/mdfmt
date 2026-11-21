#!/usr/bin/env python3
"""Build a curated host-native ZIP and checksum without publishing anything."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import stat
import subprocess
import sys
import tempfile
import time
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[1]
SUPPLEMENTS = {
    ("fast-glob", "1.1.2"): ("licenses/fast-glob-1.1.2.txt", "https://github.com/oxc-project/fast-glob/blob/cd1157ebe949c07c2e08ccf7b96f753dfc08e7bd/LICENSE"),
    ("nodejs-built-in-modules", "1.0.0"): ("licenses/nodejs-built-in-modules-1.0.0.txt", "https://github.com/oxc-project/nodejs-built-in-modules/blob/9117efa38fe088325e8a1036a8ddea7f3546c1ac/LICENSE"),
}
LOCAL_NOTICES = {
    ".": ("LICENSE",),
    "vendor/regress": ("LICENSE-APACHE", "LICENSE-MIT"),
    "vendor/oxc_resolver": ("LICENSE",),
}
SOURCE_TREES = ("src", "vendor/regress/src", "vendor/oxc_resolver/src")
SOURCE_FILES = (
    "Cargo.toml", "Cargo.lock", "mise.toml", "vendor/regress/Cargo.toml",
    "vendor/oxc_resolver/Cargo.toml", "docs/compatibility/rules.json",
)
OPTIONAL_BUILD_INPUTS = (
    "build.rs", "vendor/regress/build.rs", "vendor/oxc_resolver/build.rs",
    ".cargo/config", ".cargo/config.toml",
)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_input(path):
    if any(part == ".env" or part.startswith(".env.") for part in path.parts):
        raise ValueError("environment files cannot be release inputs")
    resolved = path.resolve()
    if any(part == ".env" or part.startswith(".env.") for part in resolved.parts):
        raise ValueError("environment files cannot be release inputs")
    return path.read_bytes()


def windows_job_api():
    import ctypes
    from ctypes import wintypes
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel.CreateJobObjectW.argtypes = [wintypes.LPVOID, wintypes.LPCWSTR]
    kernel.CreateJobObjectW.restype = wintypes.HANDLE
    kernel.OpenJobObjectW.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.LPCWSTR]
    kernel.OpenJobObjectW.restype = wintypes.HANDLE
    kernel.SetInformationJobObject.argtypes = [wintypes.HANDLE, ctypes.c_int, wintypes.LPVOID, wintypes.DWORD]
    kernel.SetInformationJobObject.restype = wintypes.BOOL
    kernel.AssignProcessToJobObject.argtypes = [wintypes.HANDLE, wintypes.HANDLE]
    kernel.AssignProcessToJobObject.restype = wintypes.BOOL
    kernel.GetCurrentProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    kernel.CloseHandle.restype = wintypes.BOOL
    return ctypes, kernel


def create_windows_job():
    ctypes, kernel = windows_job_api()
    from ctypes import wintypes

    class BasicLimits(ctypes.Structure):
        _fields_ = [
            ("process_time", ctypes.c_longlong), ("job_time", ctypes.c_longlong),
            ("flags", wintypes.DWORD), ("min_working_set", ctypes.c_size_t),
            ("max_working_set", ctypes.c_size_t), ("active_processes", wintypes.DWORD),
            ("affinity", ctypes.c_size_t), ("priority", wintypes.DWORD),
            ("scheduling", wintypes.DWORD),
        ]

    class ExtendedLimits(ctypes.Structure):
        _fields_ = [
            ("basic", BasicLimits), ("io_counters", ctypes.c_ulonglong * 6),
            ("process_memory", ctypes.c_size_t), ("job_memory", ctypes.c_size_t),
            ("peak_process_memory", ctypes.c_size_t), ("peak_job_memory", ctypes.c_size_t),
        ]

    name = "Local\\mdfmt-build-" + uuid.uuid4().hex
    handle = kernel.CreateJobObjectW(None, name)
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    limits = ExtendedLimits()
    limits.basic.flags = 0x2000  # JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
    if not kernel.SetInformationJobObject(handle, 9, ctypes.byref(limits), ctypes.sizeof(limits)):
        error = ctypes.WinError(ctypes.get_last_error())
        kernel.CloseHandle(handle)
        raise error
    return kernel, handle, name


def contained_windows_command(name, arguments):
    ctypes, kernel = windows_job_api()
    handle = kernel.OpenJobObjectW(1, False, name)  # JOB_OBJECT_ASSIGN_PROCESS
    if not handle:
        raise ctypes.WinError(ctypes.get_last_error())
    try:
        if not kernel.AssignProcessToJobObject(handle, kernel.GetCurrentProcess()):
            raise ctypes.WinError(ctypes.get_last_error())
    finally:
        kernel.CloseHandle(handle)
    return subprocess.call(arguments)


def run(arguments, *, environment=None, timeout=600):
    arguments = list(map(str, arguments))
    job = create_windows_job() if os.name == "nt" else None
    launched = [sys.executable, str(Path(__file__).resolve()), "--contained-command", job[2], *arguments] if job else arguments
    process = None
    # Build logs are files, so inherited handles cannot prolong timeout cleanup.
    with tempfile.TemporaryFile() as stdout, tempfile.TemporaryFile() as stderr:
        try:
            process = subprocess.Popen(launched, cwd=ROOT, env=environment, stdin=subprocess.DEVNULL,
                                       stdout=stdout, stderr=stderr, start_new_session=not job)
            process.wait(timeout=timeout)
        except subprocess.TimeoutExpired:
            if job:
                process.kill()
            else:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired as error:
                raise RuntimeError("build process cleanup exceeded one second") from error
            raise
        finally:
            if job:
                job[0].CloseHandle(job[1])
            elif process:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        stdout.seek(0)
        stderr.seek(0)
        output = stdout.read().decode("utf-8")
        error = stderr.read().decode("utf-8")
    if process.returncode:
        raise RuntimeError(f"command failed ({process.returncode}): {' '.join(arguments)}\n{error}")
    return output.strip()


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
        if package.get("source") is None:
            relative = directory.relative_to(ROOT.resolve()).as_posix()
            if relative not in LOCAL_NOTICES:
                raise ValueError(f"unreviewed local package notices: {relative}")
            files = {directory / name for name in LOCAL_NOTICES[relative]}
        else:
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
    paths = sorted(path for tree in SOURCE_TREES for path in (ROOT / tree).rglob("*") if path.is_file())
    paths += [ROOT / name for name in SOURCE_FILES]
    paths += [ROOT / name for name in OPTIONAL_BUILD_INPUTS if (ROOT / name).is_file()]
    for path in paths:
        checksum.update(path.relative_to(ROOT).as_posix().encode() + b"\0" + read_input(path))
    return {
        "git_commit": run(["git", "rev-parse", "--verify", "HEAD"], timeout=30),
        "inputs_dirty": bool(run(["git", "status", "--porcelain", "--untracked-files=all", "--",
                                 *SOURCE_TREES, *SOURCE_FILES, *OPTIONAL_BUILD_INPUTS,
                                 "README.md", "LICENSE", "tools/release.py", "licenses"], timeout=30)),
        "production_inputs_sha256": checksum.hexdigest(),
        "production_inputs": sorted(path.relative_to(ROOT).as_posix() for path in paths),
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
    if os.name == "nt" and sys.argv[1:2] == ["--contained-command"]:
        sys.exit(contained_windows_command(sys.argv[2], sys.argv[3:]))
    else:
        main()
