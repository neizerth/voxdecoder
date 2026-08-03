//! Shared word / non-word tokenizer used by every stage that needs to walk
//! individual words without disturbing surrounding punctuation/whitespace.

pub struct Tok<'a> {
    pub text: &'a str,
    pub is_word: bool,
}

pub fn is_wordish(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-' || c == '+'
}

pub fn tokenize(input: &str) -> Vec<Tok<'_>> {
    let mut toks = Vec::new();
    let mut rest = input;
    while !rest.is_empty() {
        let mut chars = rest.char_indices();
        let (_, first) = chars.next().expect("non-empty");
        let word = is_wordish(first);
        let end = chars
            .find(|(_, c)| is_wordish(*c) != word)
            .map_or(rest.len(), |(i, _)| i);
        toks.push(Tok {
            text: &rest[..end],
            is_word: word,
        });
        rest = &rest[end..];
    }
    toks
}
