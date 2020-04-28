extern crate fst;
extern crate memmap;
extern crate serde_json;
extern crate simd_json; 

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::{BufRead};
use std::path::Path;
use std::time::Instant;


// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub struct SynonymIndex {
    line_vecs: Vec<Vec<String>>,
    index: HashMap<String, Vec<usize>>
}

/**
 *
 */
pub fn generate_synonym_index(file_path: &str) -> SynonymIndex {

    let line_vecs: Vec<Vec<String>> = Vec::new();
    let index: HashMap<String, Vec<usize>> = HashMap::new();
    let mut synonym_index = SynonymIndex{line_vecs, index};
    let mut counter = 0;
    let process_start = Instant::now();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines {
            if let Ok(entry) = line {
                // Just split by ','
                let mut all_words: Vec<String> = Vec::new();
                for word in entry.split(",") {
                    all_words.push(word.to_ascii_lowercase());
                }
                let root_word = all_words.first().unwrap();
                for _word in &all_words[1..] {
                    let index_entry = synonym_index.index.entry(root_word.to_string()).or_insert_with(Vec::new);
                    index_entry.push(counter);
                }
                synonym_index.line_vecs.push(all_words);
                counter += 1;
                if counter % 1000000 == 0 {
                    println!("counter: {}", counter);
                }
            }
        }
    }
    println!("Finished: {} seconds", process_start.elapsed().as_secs());
    return synonym_index;
}


/**
 * Se
 */
pub fn search_synonym_index(term: &str, index: &SynonymIndex) -> HashMap<String, String> {

    let mut result_map: HashMap<String, String> = HashMap::new();

    // original is always a synonym
    result_map.insert(term.to_string(), term.to_string());

    match index.index.get(term) {
        Some(line_indexes) => {
            for line_num in line_indexes {
                for syn in &(index.line_vecs)[*line_num as usize] {
                    result_map.insert(syn.to_string(), term.to_string());
                }
            }
        },
        None => {}
    }
    return result_map;
}
