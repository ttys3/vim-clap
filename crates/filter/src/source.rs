use std::marker::PhantomData;

use super::*;
use matcher::{fzy, skim, substring};
use std::io::BufRead;
use std::path::PathBuf;
#[cfg(feature = "enable_dyn")]
use subprocess::Exec;

use matcher::MatchItem;

/// Source is anything that can produce an iterator of String.
#[derive(Debug)]
pub enum Source<'a, SourceItem: From<String> + MatchItem<'a>, I: Iterator<Item = SourceItem>> {
    Stdin,
    #[cfg(feature = "enable_dyn")]
    Exec(Exec),
    File(PathBuf),
    List(I),
    Phantom(PhantomData<&'a SourceItem>),
}

impl<'a, SourceItem: From<String> + MatchItem<'a>> From<Vec<String>>
    for Source<'a, SourceItem, std::vec::IntoIter<SourceItem>>
{
    // TODO: fix collect?
    fn from(source_list: Vec<String>) -> Self {
        Self::List(
            source_list
                .into_iter()
                .map(SourceItem::from)
                .collect::<Vec<_>>()
                .into_iter(),
        )
    }
}

impl<'a, SourceItem: From<String> + MatchItem<'a> + Send, I: Iterator<Item = SourceItem>>
    From<PathBuf> for Source<'a, SourceItem, I>
{
    fn from(fpath: PathBuf) -> Self {
        Self::File(fpath)
    }
}

#[cfg(feature = "enable_dyn")]
impl<'a, SourceItem: From<String> + MatchItem<'a>, I: Iterator<Item = SourceItem>> From<Exec>
    for Source<'a, SourceItem, I>
{
    fn from(exec: Exec) -> Self {
        Self::Exec(exec)
    }
}

impl<'a, SourceItem: From<String> + MatchItem<'a> + Send, I: Iterator<Item = SourceItem>>
    Source<'a, SourceItem, I>
{
    /// Returns the complete filtered results after applying the specified
    /// matcher algo on each item in the input stream.
    ///
    /// This is kind of synchronous filtering, can be used for multi-staged processing.
    pub fn filter(self, algo: Algo, query: &str) -> Result<Vec<FilterResult<SourceItem>>> {
        let fuzzy_algo = match algo {
            Algo::Skim => skim::fuzzy_indices,
            Algo::Fzy => fzy::fuzzy_indices,
            Algo::SubString => substring::substr_indices,
        };

        let apply_match = |source_item: SourceItem| {
            source_item
                .do_match(query, fuzzy_algo)
                .map(|(score, indices)| (source_item, score, indices))
        };

        let filtered = match self {
            Self::Stdin => std::io::stdin()
                .lock()
                .lines()
                .filter_map(|lines_iter| {
                    lines_iter
                        .ok()
                        .map(|s| SourceItem::from(s))
                        .and_then(|source_item| apply_match(source_item))
                })
                .collect::<Vec<_>>(),
            #[cfg(feature = "enable_dyn")]
            Self::Exec(exec_cmd) => std::io::BufReader::new(exec_cmd.stream_stdout()?)
                .lines()
                .filter_map(|lines_iter| {
                    lines_iter
                        .ok()
                        .map(|s| SourceItem::from(s))
                        .and_then(|line| apply_match(line))
                })
                .collect::<Vec<_>>(),
            Self::File(fpath) => std::fs::read_to_string(fpath)?
                .par_lines()
                .map(|s| SourceItem::from(s.to_string()))
                .filter_map(|line| apply_match(line))
                .collect::<Vec<_>>(),
            Self::List(list) => list.filter_map(apply_match).collect::<Vec<_>>(),
            _ => unreachable!(),
        };

        Ok(filtered)
    }
}
