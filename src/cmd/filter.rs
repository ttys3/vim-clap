use std::path::PathBuf;

use anyhow::Result;
use fuzzy_filter::{fuzzy_filter_and_rank, truncate_long_matched_lines, Algo};

use crate::icon::prepend_icon;

pub fn run(
    query: String,
    input: Option<PathBuf>,
    algo: Option<Algo>,
    number: Option<usize>,
    enable_icon: bool,
    winwidth: Option<usize>,
) -> Result<()> {
    let ranked = fuzzy_filter_and_rank(&query, input, algo.unwrap_or(Algo::Fzy))?;

    if let Some(number) = number {
        let total = ranked.len();
        let payload = ranked.into_iter().take(number);
        let winwidth = winwidth.unwrap_or(62);
        let (truncated_payload, truncated_map) =
            truncate_long_matched_lines(payload, winwidth, None);
        let mut lines = Vec::with_capacity(number);
        let mut indices = Vec::with_capacity(number);
        if enable_icon {
            for (text, _, idxs) in truncated_payload {
                let iconized = prepend_icon(&text);
                lines.push(iconized);
                indices.push(idxs);
            }
        } else {
            for (text, _, idxs) in truncated_payload {
                lines.push(text);
                indices.push(idxs);
            }
        }
        if truncated_map.is_empty() {
            println_json!(total, lines, indices);
        } else {
            println_json!(total, lines, indices, truncated_map);
        }
    } else {
        for (text, _, indices) in ranked.iter() {
            println_json!(text, indices);
        }
    }

    Ok(())
}

pub fn filter<'b>(
    lines: impl std::iter::Iterator<Item = &'b str>,
    query: &str,
    algo: Option<Algo>,
) -> Result<Vec<(String, f64, Vec<usize>)>> {
    let algo = algo.unwrap_or(Algo::Fzy);

    let scorer = |line: &str| match algo {
        Algo::Skim => fuzzy_indices(line, &query).map(|(score, indices)| (score as f64, indices)),
        Algo::Fzy => match_and_score_with_positions(&query, line),
    };

    let mut ranked = lines
        .filter_map(|line| {
            scorer(line).map(|(score, indices)| {
                let line_str: String = line.to_string();
                (line_str, score, indices)
            })
        })
        .collect::<Vec<_>>();

    ranked.par_sort_unstable_by(|(_, v1, _), (_, v2, _)| v2.partial_cmp(&v1).unwrap());

    Ok(ranked)
}
