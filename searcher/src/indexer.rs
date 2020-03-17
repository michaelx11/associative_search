extern crate fst;
extern crate serde_json;
extern crate simd_json; 

use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::time::{Duration, Instant};

use fst::{IntoStreamer, Streamer, Map, MapBuilder};

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
    // FST file
    fst_file: String,
    // K threshold
    max_group: usize,
    // Original Vector
    orig_vec: Vec<Vec<String>>
}

#[derive(Eq, Ord, PartialEq, PartialOrd)]
pub struct StemChunk {
    stem: String,
    index: u64
}

/**
 * Populates an index from a file with format:
 * - ["text", ["a", "bunch", "of", "article", "titles", "containing", "text"]]
 *
 */
pub fn generate_stemmed_index(file_path: &str, max_group: usize) -> StemmedIndex {

    let fst_file = format!("{}.{}", file_path, "fst");
    let mut wtr = io::BufWriter::new(File::create(&fst_file).unwrap());
    let mut build = MapBuilder::new(wtr).unwrap();
    let orig_vec: Vec<Vec<String>> = Vec::new();
    let mut chunk_vec: Vec<StemChunk> = Vec::new();
    let mut result_index = StemmedIndex{
        fst_file,
        max_group,
        orig_vec
    };
    let mut counter: u64 = 0;
    let process_start = Instant::now();
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
                    let stem_string = stem.to_string();
                    chunk_vec.push(StemChunk{
                        stem: stem_string,
                        index: counter
                    });
                }
                counter += 1;
                if counter % 1000000 == 0 {
                    println!("counter: {}", counter);
                }
            }
        }
    }
    println!("Finished gathering stemmed chunks in: {} seconds", process_start.elapsed().as_secs());
    let sort_start = Instant::now();
    println!("Sorting now");
    chunk_vec.sort();
    println!("Finished sorting in: {} seconds", sort_start.elapsed().as_secs());

    println!("Building fst");
    let fst_start = Instant::now();
    for stem_chunk in chunk_vec {
        build.insert(stem_chunk.stem, stem_chunk.index).unwrap();
    }
    println!("Finished building fst: {} seconds", fst_start.elapsed().as_secs());
    build.finish().unwrap();
    println!("Finished writing fst: {} seconds (cumulative)", fst_start.elapsed().as_secs());
    return result_index;
}
