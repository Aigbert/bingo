//! The one ranking for dock lists, the `/` palette and the `@` mentions: a
//! prefix beats a run inside the text, which beats letters in order; a tie
//! keeps the order the owner listed its items in.

/// How well a query matched, and the characters of the text it matched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fuzzy {
    pub score: i64,
    pub hits: Vec<usize>,
}

/// Match `query` against `text` without regard to case. `hits` are character
/// indices into `text`; an empty query matches everything with no hits.
pub fn fuzzy(query: &str, text: &str) -> Option<Fuzzy> {
    let query = folded(query);
    if query.is_empty() {
        return Some(Fuzzy {
            score: 0,
            hits: Vec::new(),
        });
    }
    let text = folded(text);
    match contained(&text, &query) {
        Some(0) => Some(Fuzzy {
            score: 300 - count(text.len()),
            hits: (0..query.len()).collect(),
        }),
        Some(at) => Some(Fuzzy {
            score: 200 - count(at),
            hits: (at..at + query.len()).collect(),
        }),
        None => subsequence(&text, &query),
    }
}

/// The items that match, best first; equal scores keep the items' order.
pub fn ranked<'a, T>(
    query: &str,
    items: impl IntoIterator<Item = &'a T>,
    text: impl Fn(&T) -> &str,
) -> Vec<(&'a T, Fuzzy)>
where
    T: 'a,
{
    let mut found = items
        .into_iter()
        .filter_map(|item| fuzzy(query, text(item)).map(|fuzzy| (item, fuzzy)))
        .collect::<Vec<_>>();
    found.sort_by_key(|(_, fuzzy)| std::cmp::Reverse(fuzzy.score));
    found
}

fn folded(text: &str) -> Vec<char> {
    text.chars()
        .map(|c| c.to_lowercase().next().unwrap_or(c))
        .collect()
}

fn contained(text: &[char], query: &[char]) -> Option<usize> {
    text.windows(query.len()).position(|run| run == query)
}

fn subsequence(text: &[char], query: &[char]) -> Option<Fuzzy> {
    let mut wanted = query.iter().peekable();
    let mut hits = Vec::new();
    for (at, c) in text.iter().enumerate() {
        if wanted.peek() == Some(&c) {
            wanted.next();
            hits.push(at);
        }
    }
    if wanted.peek().is_some() {
        return None;
    }
    let spread = hits.last().zip(hits.first()).map_or(0, |(l, f)| l - f);
    Some(Fuzzy {
        score: 100 - count(spread),
        hits,
    })
}

fn count(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_prefix_scores_by_length_and_hits_the_head() {
        let found = fuzzy("st", "status").unwrap();
        assert_eq!(found.score, 294);
        assert_eq!(found.hits, vec![0, 1]);
    }
    #[test]
    fn a_contained_run_scores_by_where_it_starts() {
        let found = fuzzy("queue", "src/session/queue.rs").unwrap();
        assert_eq!(found.score, 188);
        assert_eq!(found.hits, (12..17).collect::<Vec<_>>());
    }
    #[test]
    fn letters_in_order_score_by_their_spread() {
        let found = fuzzy("re", "resume").unwrap();
        assert_eq!(found.score, 294);
        let found = fuzzy("mdl", "model").unwrap();
        assert_eq!(found.score, 96);
        assert_eq!(found.hits, vec![0, 2, 4]);
        assert_eq!(fuzzy("zz", "model"), None);
    }
    #[test]
    fn case_is_ignored_and_an_empty_query_matches_with_no_hits() {
        assert_eq!(fuzzy("MO", "model").unwrap().hits, vec![0, 1]);
        assert_eq!(
            fuzzy("", "model"),
            Some(Fuzzy {
                score: 0,
                hits: vec![]
            })
        );
    }
    #[test]
    fn ranking_puts_prefix_then_contains_then_subsequence_and_keeps_ties_in_order() {
        let items = [
            "experience",
            "rename",
            "resume",
            "rewind",
            "review",
            "provider",
        ];
        let found = ranked("re", &items, |item| item)
            .into_iter()
            .map(|(item, _)| *item)
            .collect::<Vec<_>>();
        assert_eq!(
            found,
            [
                "rename",
                "resume",
                "rewind",
                "review",
                "experience",
                "provider"
            ]
        );
    }
}
