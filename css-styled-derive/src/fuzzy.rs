use strsim::levenshtein;

/// Return the closest match from `candidates` to `input`, if one is within a reasonable distance.
pub fn closest_match<'a>(input: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let threshold = (input.len() / 2).max(2);
    candidates
        .iter()
        .map(|c| (*c, levenshtein(input, c)))
        .filter(|(_, d)| *d <= threshold)
        .min_by_key(|(_, d)| *d)
        .map(|(c, _)| c)
}
