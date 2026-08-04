"""Sentence segmentation via razdel."""

from typing import List
from pydantic import BaseModel
import razdel


class Sentence(BaseModel):
    text: str
    start: int
    end: int


class SentenceSplitter:
    """Split text into sentences using razdel."""

    def split(self, text: str) -> List[Sentence]:
        """Split text into sentences with byte offsets."""
        sentences = []
        for sent in razdel.sentenize(text):
            sentences.append(
                Sentence(
                    text=sent.text,
                    start=sent.start,
                    end=sent.stop,
                )
            )
        return sentences
