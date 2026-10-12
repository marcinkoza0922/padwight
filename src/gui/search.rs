//! Fuzzy matching for the search boxes. Every word typed must find a word of the text, either as
//! part of it, a typo away from it, or as its first letter and some of the ones after it ("sens"
//! and "snstvty" both find "sensitivity"). Words are matched one by one, so order doesn't matter.

/// How well `query` matches `haystack`: `None` if some word of it finds nothing, else a score where
/// lower is better. An empty query matches everything with 0.
pub(super) fn fuzzy(query: &str, haystack: &str) -> Option<u32> {
    let haystack = haystack.to_lowercase();
    let tokens: Vec<&str> = haystack.split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).collect();
    query.split_whitespace().try_fold(0, |total, word| Some(total + word_score(&word.to_lowercase(), &haystack, &tokens)?))
}

/// Whether `fuzzy` finds the query in `haystack`.
pub(super) fn matches_query(query: &str, haystack: &str) -> bool {
    fuzzy(query, haystack).is_some()
}

fn word_score(word: &str, haystack: &str, tokens: &[&str]) -> Option<u32> {
    if haystack.contains(word) {
        return Some(0);
    }
    let len = word.chars().count();
    if len >= 4 {
        let allowed = if len >= 7 { 2 } else { 1 };
        // Against the whole word and its start, so a half-typed word with a slip still counts.
        let near = tokens.iter().any(|t| {
            let start: String = t.chars().take(len).collect();
            edit_distance(word, t, allowed) <= allowed || edit_distance(word, &start, allowed) <= allowed
        });
        if near {
            return Some(1);
        }
    }
    if len >= 3 && tokens.iter().any(|t| t.chars().next() == word.chars().next() && is_subsequence(word, t)) {
        return Some(2);
    }
    None
}

fn is_subsequence(word: &str, token: &str) -> bool {
    let mut rest = token.chars();
    word.chars().all(|c| rest.any(|t| t == c))
}

/// Levenshtein distance between `a` and `b`, or something above `limit` once it is certain to be.
fn edit_distance(a: &str, b: &str, limit: usize) -> usize {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    if a.len().abs_diff(b.len()) > limit {
        return limit + 1;
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, &cb) in b.iter().enumerate() {
            cur.push((prev[j] + usize::from(ca != cb)).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_parts_typos_and_abbreviations() {
        let text = "Gyro sensitivity and Dead zone";
        assert_eq!(fuzzy("", text), Some(0));
        assert_eq!(fuzzy("SENS", text), Some(0));
        assert_eq!(fuzzy("zone dead", text), Some(0), "order doesn't matter");
        assert_eq!(fuzzy("sensitivty", text), Some(1), "a typo");
        assert_eq!(fuzzy("snstvty", text), Some(2), "abbreviated");
        assert!(fuzzy("gyro rumble", text).is_none(), "every word must find something");
        assert!(fuzzy("xyz", text).is_none());
    }

    #[test]
    fn short_words_are_not_fuzzy() {
        assert!(fuzzy("gxo", "gyro").is_none());
        assert!(fuzzy("zq", "zone quick").is_none());
    }
}
