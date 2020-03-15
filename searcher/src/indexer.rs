extern crate serde_json;
extern crate simd_json; 

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use serde_json::Value;

use super::stemmer;

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub struct StemmedIndex {
    // HashMap
    index_map: HashMap<String, Vec<u32>>,
    // K threshold
    max_group: usize,
    // Original Vector
    orig_vec: Vec<Vec<String>>
}

/**
 * Populates an index from a file with format:
 * - ["text", ["a", "bunch", "of", "article", "titles", "containing", "text"]]
 *
 */
pub fn generate_stemmed_index(file_path: &str, max_group: usize) -> StemmedIndex {
    let index_map: HashMap<String, Vec<u32>> = HashMap::new();
    let orig_vec: Vec<Vec<String>> = Vec::new();
    let mut result_index = StemmedIndex{
        index_map,
        max_group,
        orig_vec
    };
    let mut counter: u32 = 0;
    if let Ok(lines) = read_lines(file_path) {
        for line in lines {
            if let Ok(entry) = line {
                let mut mutable_bytes = entry.into_bytes();
                let v: Value = simd_json::serde::from_slice(&mut mutable_bytes).unwrap();
                let pair = v.as_array().unwrap();
                let title = pair[0].as_str().unwrap();
                let article_array = pair[1].as_array().unwrap();
                // Generate stems from title
                let stems = stemmer::generate_stems(&title, max_group);
                if stems.len() == 0 {
                    continue;
                }

                let mut article_vec = vec![title.to_string()];
                for article in article_array.iter() {
                    let article_string = article.as_str().unwrap();
                    article_vec.push(article_string.to_string());
                }
                result_index.orig_vec.push(article_vec);
                // For each stem, insert into 
                for stem in stems {
                    let entry = result_index.index_map.entry(stem.to_string()).or_insert_with(Vec::new);
                    entry.push(counter);
                }
                counter += 1;
                if counter % 100000 == 0 {
                    println!("counter: {}", counter);
                }
            }
        }
    }
    return result_index;
}
