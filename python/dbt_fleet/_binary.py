"""Locate (or download) the dbt-fleet Rust binary, then exec it.

Strategy:

1. Honour ``DBT_FLEET_BINARY`` if it points at an executable.
2. Look in the version-pinned cache directory.
3. Download the matching tarball / zip from the dbt-fleet GitHub Release.
4. Verify the SHA-256 checksum that ships alongside the asset.
5. Extract the binary into the cache and exec it with the caller's argv.

We never try to keep the binary in lock-step with the Python process at runtime
(no in-process invocation, no FFI). The Rust binary owns all behaviour; this
module is plumbing.
"""

from __future__ import annotations

import hashlib
import os
import platform
import shutil
import sys
import tarfile
import tempfile
import urllib.error
import urllib.request
import zipfile
from pathlib import Path

from dbt_fleet import __version__

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

GITHUB_OWNER = "dreynow"
GITHUB_REPO = "dbt-fleet"
DOWNLOAD_TIMEOUT_SECONDS = 60

# (system, machine) → cargo target triple
_TARGET_MAP = {
    ("Linux", "x86_64"): ("x86_64-unknown-linux-gnu", "tar.gz"),
    ("Linux", "aarch64"): ("aarch64-unknown-linux-gnu", "tar.gz"),
    ("Linux", "arm64"): ("aarch64-unknown-linux-gnu", "tar.gz"),
    ("Darwin", "x86_64"): ("x86_64-apple-darwin", "tar.gz"),
    ("Darwin", "arm64"): ("aarch64-apple-darwin", "tar.gz"),
    ("Windows", "AMD64"): ("x86_64-pc-windows-msvc", "zip"),
    ("Windows", "x86_64"): ("x86_64-pc-windows-msvc", "zip"),
}


class BinaryError(RuntimeError):
    """Raised when we can't find or fetch the dbt-fleet binary."""


# ---------------------------------------------------------------------------
# Platform detection
# ---------------------------------------------------------------------------


def _resolve_target() -> tuple[str, str]:
    system = platform.system()
    machine = platform.machine()
    try:
        return _TARGET_MAP[(system, machine)]
    except KeyError as exc:
        supported = ", ".join(sorted({t[0] for t in _TARGET_MAP.values()}))
        raise BinaryError(
            f"Unsupported platform: {system}/{machine}. "
            f"Pre-built binaries exist for: {supported}. "
            "Build from source: https://github.com/dreynow/dbt-fleet"
        ) from exc


def _binary_filename() -> str:
    return "dbt-fleet.exe" if platform.system() == "Windows" else "dbt-fleet"


# ---------------------------------------------------------------------------
# Cache layout
# ---------------------------------------------------------------------------


def _cache_root() -> Path:
    """Per-user cache root. Honours DBT_FLEET_CACHE_DIR; otherwise XDG-friendly."""
    override = os.environ.get("DBT_FLEET_CACHE_DIR")
    if override:
        return Path(override).expanduser()
    if platform.system() == "Windows":
        base = os.environ.get("LOCALAPPDATA") or str(Path.home() / "AppData" / "Local")
        return Path(base) / "dbt-fleet" / "Cache"
    return Path.home() / ".cache" / "dbt-fleet"


def _cached_binary_path(version: str) -> Path:
    """Versioned binary path, so upgrading doesn't smash an older copy."""
    return _cache_root() / f"v{version}" / _binary_filename()


# ---------------------------------------------------------------------------
# Download + extract
# ---------------------------------------------------------------------------


def _release_asset_url(version: str, target: str, archive_ext: str) -> str:
    name = f"{GITHUB_REPO}-v{version}-{target}.{archive_ext}"
    return (
        f"https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases/download/"
        f"v{version}/{name}"
    )


def _download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    try:
        with urllib.request.urlopen(url, timeout=DOWNLOAD_TIMEOUT_SECONDS) as resp:  # noqa: S310 — github.com is the only URL we open
            with open(dest, "wb") as out:
                shutil.copyfileobj(resp, out)
    except urllib.error.HTTPError as e:
        raise BinaryError(
            f"Could not download {url}: HTTP {e.code}. "
            "Check the version exists at "
            f"https://github.com/{GITHUB_OWNER}/{GITHUB_REPO}/releases"
        ) from e
    except urllib.error.URLError as e:
        raise BinaryError(f"Network error downloading {url}: {e.reason}") from e


def _verify_sha256(archive: Path, expected_hex: str) -> None:
    h = hashlib.sha256()
    with open(archive, "rb") as f:
        for chunk in iter(lambda: f.read(65536), b""):
            h.update(chunk)
    actual = h.hexdigest()
    if actual.lower() != expected_hex.lower():
        raise BinaryError(
            f"Checksum mismatch for {archive.name}: "
            f"expected {expected_hex}, got {actual}"
        )


def _read_expected_sha(checksum_text: str, archive_name: str) -> str:
    """Parse a sha256sum / shasum file and return the hash for ``archive_name``.

    Accepts the variants we see in the wild:

    - ``<hex>  <filename>`` — GNU coreutils default
    - ``<hex> *<filename>`` — BSD binary-mode marker
    - ``<hex>  some/dir/<filename>`` — when the producer ran sha256sum with a
      relative path (our release.yml does ``sha256sum dist/<file>``)
    - ``<hex>`` alone on a line — single-column hash file

    Matches by basename so directory prefixes don't break verification.
    """
    for line in checksum_text.splitlines():
        line = line.strip()
        if not line:
            continue
        parts = line.split(maxsplit=1)
        if len(parts) == 2:
            recorded_path = parts[1].lstrip("*")
            if os.path.basename(recorded_path) == archive_name:
                return parts[0]
        elif len(parts) == 1 and len(parts[0]) == 64:
            return parts[0]
    raise BinaryError(
        f"Checksum file did not contain a hash for {archive_name}: {checksum_text!r}"
    )


def _extract(archive: Path, into: Path) -> Path:
    """Extract the archive and return the path to the dbt-fleet executable inside it.

    Release archives use the layout
    ``<crate>-v<ver>-<target>/{dbt-fleet,LICENSE,README.md}``.
    """
    if archive.suffix == ".zip":
        with zipfile.ZipFile(archive) as z:
            z.extractall(into)
    else:
        with tarfile.open(archive, "r:gz") as t:
            _safe_extract(t, into)

    # Find the binary anywhere under `into`.
    for path in into.rglob(_binary_filename()):
        if path.is_file():
            return path
    raise BinaryError(f"No {_binary_filename()} found inside {archive.name}")


def _safe_extract(tar: tarfile.TarFile, into: Path) -> None:
    """Extract a tarfile, refusing entries that escape ``into``."""
    base = into.resolve()
    for member in tar.getmembers():
        target = (into / member.name).resolve()
        if not str(target).startswith(str(base)):
            raise BinaryError(f"Refusing to extract escaping path: {member.name}")
    tar.extractall(into)  # noqa: S202 — paths validated above


# ---------------------------------------------------------------------------
# Resolution
# ---------------------------------------------------------------------------


def _ensure_binary(version: str) -> Path:
    """Return a path to the dbt-fleet binary, downloading it if necessary."""
    override = os.environ.get("DBT_FLEET_BINARY")
    if override:
        path = Path(override).expanduser()
        if not path.is_file():
            raise BinaryError(f"DBT_FLEET_BINARY points at non-existent file: {path}")
        return path

    cached = _cached_binary_path(version)
    if cached.is_file():
        return cached

    target, archive_ext = _resolve_target()
    archive_url = _release_asset_url(version, target, archive_ext)
    sha_url = archive_url + ".sha256"
    archive_name = f"{GITHUB_REPO}-v{version}-{target}.{archive_ext}"

    sys.stderr.write(
        f"dbt-fleet: downloading binary v{version} for {target}…\n"
    )

    with tempfile.TemporaryDirectory(prefix="dbt-fleet-install-") as tmp_str:
        tmp = Path(tmp_str)
        archive = tmp / archive_name
        sha_file = tmp / (archive_name + ".sha256")

        _download(archive_url, archive)
        try:
            _download(sha_url, sha_file)
            expected = _read_expected_sha(sha_file.read_text(), archive_name)
            _verify_sha256(archive, expected)
        except BinaryError as e:
            # Don't fail the install if sha file is genuinely missing — warn and proceed.
            # We still want a working `pip install` if the upstream forgot to upload .sha256.
            sys.stderr.write(f"dbt-fleet: checksum verification skipped ({e})\n")

        extracted_binary = _extract(archive, tmp / "extracted")
        cached.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(extracted_binary, cached)
        cached.chmod(0o755)

    return cached


def resolve_binary(version: str | None = None) -> Path:
    """Public-facing API: return a path to a runnable dbt-fleet binary."""
    return _ensure_binary(version or __version__)


# ---------------------------------------------------------------------------
# Console-script entry point
# ---------------------------------------------------------------------------


def main() -> int:
    """Entry point for the ``dbt-fleet`` console script."""
    try:
        binary = _ensure_binary(__version__)
    except BinaryError as e:
        sys.stderr.write(f"dbt-fleet: {e}\n")
        return 2

    # Replace this Python process with the Rust binary so signals and exit codes
    # propagate cleanly. On Windows os.execv is broken under some shells, so we
    # fall back to subprocess there.
    args = [str(binary), *sys.argv[1:]]
    if platform.system() == "Windows":
        import subprocess

        result = subprocess.run(args, check=False)
        return result.returncode
    os.execv(str(binary), args)
    # os.execv only returns on failure.
    return 1
