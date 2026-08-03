"""Tokenization via Natasha."""

from typing import List
from pydantic import BaseModel
from natasha import Segmenter


class Token(BaseModel):
    text: str
    start: int
    end: int


class Tokenizer:
    """Tokenize text using Natasha."""

    def __init__(self):
        self.segmenter = Segmenter()

    def tokenize(self, text: str) -> List[Token]:
        """Tokenize text into tokens with byte offsets."""
        doc = self.segmenter(text)
        tokens = []
        for token in doc.tokens:
            tokens.append(
                Token(
                    text=token.value,
                    start=token.start,
                    end=token.stop,
                )
            )
        return tokens
