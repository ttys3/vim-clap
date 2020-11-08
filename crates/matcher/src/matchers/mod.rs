mod file_name;
mod grep;
mod tag_name;

pub use self::file_name::FileNameMatcher;
pub use self::grep::GrepMatcher;
pub use self::tag_name::TagNameMatcher;

pub trait MatchItem<'a> {
    /// Returns the text for displaying.
    fn display_text(&self) -> &'a str;

    // Currently we only take care of matching one piece.
    /// Returns the text for matching and the offset (in byte) of it begins.
    fn match_text(&self) -> Option<(&'a str, usize)>;

    fn do_match(
        &self,
        query: &str,
        fuzzy_algo: impl FnOnce(&str, &str) -> crate::MatcherResult,
    ) -> crate::MatcherResult {
        self.match_text().and_then(|match_info| match match_info {
            (text, 0) => fuzzy_algo(text, query),
            (text, offset) => fuzzy_algo(text, query)
                .map(|(score, indices)| (score, indices.into_iter().map(|x| x + offset).collect())),
        })
    }
}
