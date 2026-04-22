from pathlib import Path
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "src"))

from evalkit_plugin import run_plugin, scorer_plugin


@scorer_plugin("exact-match-scorer", capabilities=["structured-errors"])
def score(
    input_text: str,
    output_text: str,
    reference_text: str | None,
    run_id: str | None,
    sample_id: str | None,
    trial_index: int,
    metadata: dict[str, object],
) -> dict[str, object]:
    return {
        "type": "binary",
        "value": reference_text is not None and output_text == reference_text,
    }


if __name__ == "__main__":
    run_plugin(score)
