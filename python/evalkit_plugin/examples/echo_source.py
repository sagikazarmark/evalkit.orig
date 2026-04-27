from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from evalkit_plugin import source_plugin, run_plugin


@source_plugin("echo-source", capabilities=["structured-errors"])
def produce(input_text: str) -> str:
    return f"echo::{input_text}"


if __name__ == "__main__":
    run_plugin(produce)
