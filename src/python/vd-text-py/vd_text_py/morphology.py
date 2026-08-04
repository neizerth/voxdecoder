"""Morphological analysis via Natasha."""

from typing import List, Optional
from pydantic import BaseModel
from natasha import MorphVocab


class Gram(BaseModel):
    """A single morphological tag."""
    tag: str


class Morph(BaseModel):
    """Morphological analysis for a word."""
    text: str
    grammemes: List[str]
    normalized: str
    pos: Optional[str] = None


class Morphology:
    """Morphological analysis using Natasha."""

    def __init__(self):
        self.vocab = MorphVocab()

    def analyze(self, word: str) -> Morph:
        """Analyze a word morphologically."""
        parse = self.vocab.parse(word)

        grammemes = []
        if parse.grammemes:
            grammemes = list(parse.grammemes)

        pos = None
        if parse.pos:
            pos = parse.pos

        return Morph(
            text=word,
            grammemes=grammemes,
            normalized=parse.normal_form,
            pos=pos,
        )

    def analyze_batch(self, words: List[str]) -> List[Morph]:
        """Analyze multiple words."""
        return [self.analyze(word) for word in words]
