# vz_cli

The `vz` command: the book kit's steps (code index, assets, build stamp) as subcommands that register themselves.

The binary is a thin `main` over the library's `run`, which the Python console script `vz` calls too (`vz_py`). Layer `app`.
