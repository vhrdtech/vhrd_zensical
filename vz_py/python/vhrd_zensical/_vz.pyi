"""The extension module: the ``vz`` command, built from the Rust workspace."""

def main(argv: list[str] | None = None) -> int:
    """Run ``vz`` with ``argv`` (program name first; ``sys.argv`` when omitted) and return its exit status."""
