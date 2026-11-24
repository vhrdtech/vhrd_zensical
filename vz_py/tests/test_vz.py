"""The package and its extension module, as an installed wheel presents them."""

import subprocess
import sys
import tomllib
from pathlib import Path

import vhrd_zensical
from vhrd_zensical import _vz

ROOT = Path(__file__).resolve().parents[2]


def cargo_version() -> str:
    with (ROOT / "Cargo.toml").open("rb") as f:
        return tomllib.load(f)["workspace"]["package"]["version"]


def test_package_version_is_the_cargo_version():
    assert vhrd_zensical.__version__ == cargo_version()


def test_main_runs_vz():
    assert _vz.main(["vz", "--version"]) == 0
    assert _vz.main(["vz", "no-such-command"]) == 2


def test_console_script_prints_the_cargo_version():
    vz = Path(sys.executable).parent / "vz"
    out = subprocess.run([vz, "--version"], capture_output=True, text=True, check=True)
    assert out.stdout.startswith(f"vz {cargo_version()} (")
