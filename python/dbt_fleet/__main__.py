"""Allow ``python -m dbt_fleet ...`` as an alias for the ``dbt-fleet`` script."""

from dbt_fleet._binary import main

if __name__ == "__main__":
    raise SystemExit(main())
