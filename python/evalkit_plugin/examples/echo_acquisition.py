from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from evalkit_plugin import acquisition_plugin, run_plugin


@acquisition_plugin("echo-acquisition", capabilities=["structured-errors"])
def acquire(input_text: str) -> str:
    return f"echo::{input_text}"


if __name__ == "__main__":
    run_plugin(acquire)
