#[macro_use]

pub mod stemmer;

#[cfg(test)]
mod tests {
    use super::stemmer;

    fn vec_compare(va: &[String], vb: &[String]) -> bool {
        (va.len() == vb.len()) &&
            va.iter()
              .zip(vb)
              .all(|(a, b)| a == b)
    }

    #[test]
    fn stemmer_basic_test() {
        let stems = stemmer::generate_stems("sup-cat what hi", 3);
        println!("{:?}", stems);
        let expected_lits = vec!["supcat", "what", "hi", "supcat what", "what hi", "supcat what hi"];
        let expected: Vec<String> = expected_lits.iter().map(|x| x.to_string()).collect();
        assert!(vec_compare(&(expected[..]), &stems));
    }

    #[test]
    fn stemmer_less_than_k() {
        let stems = stemmer::generate_stems("sup-cat what hi", 9);
        println!("{:?}", stems);
        let expected_lits = vec!["supcat", "what", "hi", "supcat what", "what hi", "supcat what hi"];
        let expected: Vec<String> = expected_lits.iter().map(|x| x.to_string()).collect();
        assert!(vec_compare(&(expected[..]), &stems));
    }

    #[test]
    fn stemmer_repeated_whitespace() {
        let stems = stemmer::generate_stems("hello      there", 2);
        println!("{:?}", stems);
        let expected_lits = vec!["hello", "there", "hello there"];
        let expected: Vec<String> = expected_lits.iter().map(|x| x.to_string()).collect();
        assert!(vec_compare(&(expected[..]), &stems));
    }

    #[test]
    fn stemmer_norm_lower() {
        let stems = stemmer::generate_stems("HeLlO -TheRe-", 3);
        println!("{:?}", stems);
        let expected_lits = vec!["hello", "there", "hello there"];
        let expected: Vec<String> = expected_lits.iter().map(|x| x.to_string()).collect();
        assert!(vec_compare(&(expected[..]), &stems));
    }
}
