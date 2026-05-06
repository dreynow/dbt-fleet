"""Tests for the binary resolver. Covers platform mapping, cache layout,
checksum parsing, and the DBT_FLEET_BINARY override path. Network-touching
paths (download_) are mocked.
"""

from __future__ import annotations

import hashlib
import os
import platform
import tarfile
import tempfile
import zipfile
from pathlib import Path
from unittest import mock

import pytest

from dbt_fleet import _binary


# ---------------------------------------------------------------------------
# Platform mapping
# ---------------------------------------------------------------------------


@pytest.mark.parametrize(
    ("system", "machine", "expected_target"),
    [
        ("Linux", "x86_64", "x86_64-unknown-linux-gnu"),
        ("Linux", "aarch64", "aarch64-unknown-linux-gnu"),
        ("Darwin", "x86_64", "x86_64-apple-darwin"),
        ("Darwin", "arm64", "aarch64-apple-darwin"),
        ("Windows", "AMD64", "x86_64-pc-windows-msvc"),
    ],
)
def test_resolve_target_supported_platforms(system, machine, expected_target):
    with mock.patch("platform.system", return_value=system), mock.patch(
        "platform.machine", return_value=machine
    ):
        target, archive_ext = _binary._resolve_target()
    assert target == expected_target
    assert archive_ext in ("tar.gz", "zip")


def test_resolve_target_rejects_unsupported_platform():
    with mock.patch("platform.system", return_value="Plan9"), mock.patch(
        "platform.machine", return_value="risc-v"
    ):
        with pytest.raises(_binary.BinaryError) as excinfo:
            _binary._resolve_target()
    assert "Unsupported platform" in str(excinfo.value)
    assert "Plan9/risc-v" in str(excinfo.value)


# ---------------------------------------------------------------------------
# Cache paths
# ---------------------------------------------------------------------------


def test_cache_dir_respects_env_override(monkeypatch, tmp_path):
    monkeypatch.setenv("DBT_FLEET_CACHE_DIR", str(tmp_path / "custom"))
    assert _binary._cache_root() == tmp_path / "custom"


def test_cached_binary_path_is_versioned(monkeypatch, tmp_path):
    monkeypatch.setenv("DBT_FLEET_CACHE_DIR", str(tmp_path))
    a = _binary._cached_binary_path("0.1.0")
    b = _binary._cached_binary_path("0.1.1")
    assert a != b
    assert a.parent.name == "v0.1.0"
    assert b.parent.name == "v0.1.1"


# ---------------------------------------------------------------------------
# Checksum parsing
# ---------------------------------------------------------------------------


def test_read_expected_sha_two_column_format():
    text = "abcd1234  dbt-fleet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz\n"
    sha = _binary._read_expected_sha(text, "dbt-fleet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz")
    assert sha == "abcd1234"


def test_read_expected_sha_handles_star_prefix():
    """BSD-style sha output uses ``*`` to mark binary mode."""
    text = "abcd1234 *dbt-fleet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz\n"
    sha = _binary._read_expected_sha(text, "dbt-fleet-v0.1.0-x86_64-unknown-linux-gnu.tar.gz")
    assert sha == "abcd1234"


def test_read_expected_sha_handles_directory_prefix():
    """Our release.yml runs ``sha256sum dist/<file>`` so the recorded path
    has a ``dist/`` prefix. Matching by basename keeps verification working
    against existing releases."""
    text = "ab12cd34  dist/dbt-fleet-v0.1.1-x86_64-unknown-linux-gnu.tar.gz\n"
    sha = _binary._read_expected_sha(
        text, "dbt-fleet-v0.1.1-x86_64-unknown-linux-gnu.tar.gz"
    )
    assert sha == "ab12cd34"


def test_read_expected_sha_single_column_fallback():
    text = "0123456789abcdef" * 4 + "\n"
    sha = _binary._read_expected_sha(text, "anything")
    assert sha == "0123456789abcdef" * 4


def test_read_expected_sha_raises_on_no_match():
    with pytest.raises(_binary.BinaryError):
        _binary._read_expected_sha("garbage line\n", "nope.tar.gz")


# ---------------------------------------------------------------------------
# Override flag
# ---------------------------------------------------------------------------


def test_dbt_fleet_binary_env_var_short_circuits_download(tmp_path, monkeypatch):
    fake_binary = tmp_path / "dbt-fleet"
    fake_binary.write_text("#!/bin/sh\necho fake\n")
    fake_binary.chmod(0o755)
    monkeypatch.setenv("DBT_FLEET_BINARY", str(fake_binary))
    monkeypatch.setenv("DBT_FLEET_CACHE_DIR", str(tmp_path / "cache"))

    # Should never touch the network.
    with mock.patch.object(_binary, "_download") as mock_download:
        path = _binary.resolve_binary("0.1.1")
    assert path == fake_binary
    mock_download.assert_not_called()


def test_dbt_fleet_binary_env_var_rejects_missing_path(monkeypatch, tmp_path):
    monkeypatch.setenv("DBT_FLEET_BINARY", str(tmp_path / "no-such-file"))
    with pytest.raises(_binary.BinaryError):
        _binary.resolve_binary("0.1.1")


# ---------------------------------------------------------------------------
# Archive extraction
# ---------------------------------------------------------------------------


def test_extract_tar_gz_finds_binary(tmp_path):
    # Build a tar.gz that mirrors the release layout.
    inner = tmp_path / "dbt-fleet-v0.1.1-x86_64-unknown-linux-gnu"
    inner.mkdir()
    bin_path = inner / "dbt-fleet"
    bin_path.write_text("#!/bin/sh\necho stub\n")
    bin_path.chmod(0o755)

    archive = tmp_path / "asset.tar.gz"
    with tarfile.open(archive, "w:gz") as t:
        t.add(inner, arcname=inner.name)

    found = _binary._extract(archive, tmp_path / "extracted")
    assert found.name == "dbt-fleet"
    assert found.read_text() == "#!/bin/sh\necho stub\n"


def test_extract_rejects_path_traversal(tmp_path):
    """Refuse archives that try to write outside the extraction root."""
    archive = tmp_path / "evil.tar.gz"
    payload = tmp_path / "payload"
    payload.write_text("evil")
    with tarfile.open(archive, "w:gz") as t:
        info = tarfile.TarInfo(name="../../../etc/escape")
        info.size = len(b"evil")
        import io

        t.addfile(info, io.BytesIO(b"evil"))
    with pytest.raises(_binary.BinaryError):
        _binary._extract(archive, tmp_path / "extracted")


# ---------------------------------------------------------------------------
# SHA verification
# ---------------------------------------------------------------------------


def test_verify_sha256_matches(tmp_path):
    f = tmp_path / "data.bin"
    f.write_bytes(b"hello dbt-fleet")
    expected = hashlib.sha256(f.read_bytes()).hexdigest()
    _binary._verify_sha256(f, expected)  # should not raise


def test_verify_sha256_mismatch(tmp_path):
    f = tmp_path / "data.bin"
    f.write_bytes(b"hello dbt-fleet")
    with pytest.raises(_binary.BinaryError):
        _binary._verify_sha256(f, "0" * 64)
