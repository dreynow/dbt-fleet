"""dbt-fleet — governance scoring and trends for dbt projects.

This Python package is a launcher for the underlying Rust binary. See
``dbt_fleet._binary`` for the platform detection / download / exec logic.
"""

from importlib import metadata as _metadata

try:
    __version__ = _metadata.version("dbt-fleet")
except _metadata.PackageNotFoundError:
    # Source checkout without an installed dist (e.g. ``python -m dbt_fleet`` from
    # the repo). Fall back to the version we know we ship at the time of writing.
    __version__ = "0.1.2"

__all__ = ["__version__"]
