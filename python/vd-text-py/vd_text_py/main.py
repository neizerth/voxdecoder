"""Entry point for vd-text-py sidecar. File-based IPC (vd-pipeline-compatible)."""

import sys
import json
import argparse
from pathlib import Path
from typing import Any, Dict

from .tokenizer import Tokenizer
from .sentence_splitter import SentenceSplitter
from .morphology import Morphology


def load_input(input_path: Path) -> str:
    """Load text from input file."""
    with open(input_path, "r", encoding="utf-8") as f:
        return f.read()


def save_output(output_path: Path, data: Dict[str, Any]) -> None:
    """Save JSON results to output file."""
    output_path.parent.mkdir(parents=True, exist_ok=True)
    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(data, f, ensure_ascii=False, indent=2)


def tokenize_operation(text: str) -> Dict[str, Any]:
    """Tokenize text."""
    tokenizer = Tokenizer()
    tokens = tokenizer.tokenize(text)
    return {
        "operation": "tokenize",
        "tokens": [t.model_dump() for t in tokens],
    }


def sentence_split_operation(text: str) -> Dict[str, Any]:
    """Split text into sentences."""
    splitter = SentenceSplitter()
    sentences = splitter.split(text)
    return {
        "operation": "sentence_split",
        "sentences": [s.model_dump() for s in sentences],
    }


def morph_operation(text: str) -> Dict[str, Any]:
    """Analyze text morphologically (word-by-word)."""
    tokenizer = Tokenizer()
    tokens = tokenizer.tokenize(text)

    morphology = Morphology()
    analyses = []
    for token in tokens:
        analysis = morphology.analyze(token.text)
        analyses.append(analysis.model_dump())

    return {
        "operation": "morph",
        "analyses": analyses,
    }


def main() -> None:
    """Main entry point for vd-text-py sidecar."""
    parser = argparse.ArgumentParser(
        description="VoxDecoder linguistic infrastructure sidecar (Natasha/razdel)"
    )
    parser.add_argument("-i", "--input", required=True, help="Input text file path")
    parser.add_argument("-o", "--output", required=True, help="Output JSON file path")
    parser.add_argument(
        "-op", "--operation", default="tokenize", help="Operation: tokenize, sentence_split, morph"
    )

    args = parser.parse_args()

    try:
        input_path = Path(args.input)
        output_path = Path(args.output)
        operation = args.operation

        # Load input
        text = load_input(input_path)

        # Perform operation
        if operation == "tokenize":
            result = tokenize_operation(text)
        elif operation == "sentence_split":
            result = sentence_split_operation(text)
        elif operation == "morph":
            result = morph_operation(text)
        else:
            raise ValueError(f"Unknown operation: {operation}")

        # Save output
        save_output(output_path, result)
        sys.exit(0)

    except Exception as e:
        error_result = {
            "operation": args.operation,
            "error": str(e),
        }
        save_output(Path(args.output), error_result)
        sys.exit(1)


if __name__ == "__main__":
    main()
