#[macro_use]
extern crate lazy_static;

pub mod stemmer;
pub mod indexer;
pub mod synonym_index;

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
        let mut stems = stemmer::generate_stems("sup-cat what hi", 3, false);
        eprintln!("{:?}", stems);
        let expected_list = vec!["supcat", "what", "hi", "supcat what", "what hi", "supcat what hi"];
        let mut expected: Vec<String> = expected_list.iter().map(|x| x.to_string()).collect();
        stems.sort();
        expected.sort();
        assert!(vec_compare(&(expected[..]), &stems));
    }

    #[test]
    fn stemmer_less_than_k() {
        let mut stems = stemmer::generate_stems("sup-cat what hi", 9, false);
        eprintln!("{:?}", stems);
        let expected_list = vec!["supcat", "what", "hi", "supcat what", "what hi", "supcat what hi"];
        let mut expected: Vec<String> = expected_list.iter().map(|x| x.to_string()).collect();
        stems.sort();
        expected.sort();
        assert!(vec_compare(&(expected[..]), &stems));
    }

    #[test]
    fn stemmer_repeated_whitespace() {
        let mut stems = stemmer::generate_stems("hello      there", 2, false);
        eprintln!("{:?}", stems);
        let expected_list = vec!["hello", "there", "hello there"];
        let mut expected: Vec<String> = expected_list.iter().map(|x| x.to_string()).collect();
        stems.sort();
        expected.sort();
        assert!(vec_compare(&(expected[..]), &stems));
    }

    #[test]
    fn stemmer_norm_lower() {
        let mut stems = stemmer::generate_stems("HeLlO -TheRe-", 3, false);
        eprintln!("{:?}", stems);
        let expected_list = vec!["hello", "there", "hello there"];
        let mut expected: Vec<String> = expected_list.iter().map(|x| x.to_string()).collect();
        stems.sort();
        expected.sort();
        assert!(vec_compare(&(expected[..]), &stems));
    }
}
