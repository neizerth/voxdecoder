//! Tokenize free text into candidate forms.

pub fn tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|c: char| {
        !(c.is_alphanumeric() || c == '_' || c == '-' || c == '+' || c == '/' || c == '.')
    })
    .filter(|t| !t.is_empty())
    .map(str::to_string)
}
