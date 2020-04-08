
use regex::Regex;

use std::collections::HashSet;


/**
 * Given a search string, generate 
 */
pub fn generate_stems(term: &str, k: usize, include_whole: bool) -> Vec<String> {
    lazy_static! {
        static ref nonalpha_re: Regex = Regex::new(r"[^\w\s]").unwrap(); 
    }
    // Non-alphanumeric characters are removed
    let alpha_only = nonalpha_re.replace_all(term, "");
    let alpha_only_lower = alpha_only.to_lowercase();
    // split by whitespace
    let split = alpha_only_lower.split_whitespace();
    let mut words: Vec<String> = Vec::new();
    for chunk in split {
        words.push(chunk.to_string());
    }
    let mut stems: HashSet<String> = HashSet::new();
    // Take combinations of chunks up to K
    for i in 1..k+1 {
        if i > words.len() {
            break;
        }
        for u in 0..words.len()-(i-1) {
            stems.insert(words[u..u+i].join(" "));
        }
    }
    if include_whole {
        stems.insert(words.join(" "));
    }
    return stems.into_iter().collect();
}
