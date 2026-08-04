"""VoxDecoder linguistic infrastructure sidecar (Natasha/razdel backend)."""

__version__ = "0.1.0"

from .tokenizer import Tokenizer
from .sentence_splitter import SentenceSplitter
from .morphology import Morphology

__all__ = [
    "Tokenizer",
    "SentenceSplitter",
    "Morphology",
]
