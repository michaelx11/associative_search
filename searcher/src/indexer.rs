extern crate fst;
extern crate memmap;
extern crate serde_json;
extern crate simd_json; 

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;
use std::time::{Duration, Instant};
use memmap::Mmap;

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

pub struct FstIndex {
    // vector of fst_value -> orig_lines
    fst_values: Vec<Vec<u64>>,
    // Byte offsets of each line in original index file
    line_starts: Vec<u64>,
    // Original association file path
    association_file: String,
    // FST file path
    fst_file: String,
    // max grouping threshold
    max_group: usize
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
pub fn generate_fst_index(file_path: &str, max_group: usize) -> Option<FstIndex> {

    let fst_file = format!("{}_{}.{}", "fst", file_path, "fst");
    let mut wtr = io::BufWriter::new(File::create(&fst_file).unwrap());
    let mut build = MapBuilder::new(wtr).unwrap();

    let mut chunk_vec: Vec<StemChunk> = Vec::new();
    let fst_values: Vec<Vec<u64>> = Vec::new();
    let line_starts: Vec<u64> = Vec::new();
    let association_file = file_path.to_string();

    let mut result_index = FstIndex{
        fst_values,
        line_starts,
        association_file,
        fst_file,
        max_group
    };
    let mut counter: u64 = 0;
    let mut byte_counter: u64 = 0;
    let mut fst_value_counter: u64 = 0;
    let process_start = Instant::now();
    if let Ok(lines) = read_lines(file_path) {
        for line in lines {
            if let Ok(entry) = line {
                let mut mutable_bytes = entry.into_bytes();

                result_index.line_starts.push(byte_counter);
                byte_counter += (mutable_bytes.len() + 1) as u64; // + 1 for newline
                let v: Value = simd_json::serde::from_slice(&mut mutable_bytes).unwrap();
                let pair = v.as_array().unwrap();
                let title = pair[0].as_str().unwrap();
                let article_array = pair[1].as_array().unwrap();
                // Generate stems from title
                let stems = stemmer::generate_stems(&title, max_group);
                if stems.len() > 0 {
                    // For each stem, insert into 
                    for stem in stems {
                        let stem_string = stem.to_string();
                        chunk_vec.push(StemChunk{
                            stem: stem_string,
                            index: counter
                        });
                    }
                }
                // Always increment counter otherwise
                counter += 1;
                if counter % 1000000 == 0 {
                    println!("counter: {}", counter);
                }
            } else {
                println!("Error reading line!");
                return None;
            }
        }
    }
    // Sentinel value so we can query ranges by i, i+1
    result_index.line_starts.push(byte_counter);

    println!("Finished gathering stemmed chunks in: {} seconds", process_start.elapsed().as_secs());
    let sort_start = Instant::now();
    println!("Sorting now");
    chunk_vec.sort();
    println!("Finished sorting in: {} seconds", sort_start.elapsed().as_secs());

    let mut merged_chunk_indices: VecDeque<Vec<u64>> = VecDeque::new();
    let mut merged_chunk_stems: VecDeque<String> = VecDeque::new();

    // There needs to be at least one value
    let last_but_first = chunk_vec.pop().unwrap();
    let mut current_indices: Vec<u64> = vec![last_but_first.index];
    let mut current_stem: String = last_but_first.stem.to_string();

    let merge_start = Instant::now();
    println!("Merging indentical chunks");
    // Start at the back of the chunk_vec and merge identical values
    while chunk_vec.len() > 0 {
        // Pop the last one
        let popped = chunk_vec.pop().unwrap();
        if popped.stem == current_stem {
            current_indices.push(popped.index);
        } else {
            merged_chunk_stems.push_front(current_stem.to_string());
            merged_chunk_indices.push_front(current_indices);
            current_stem = popped.stem.to_string();
            current_indices = vec![popped.index];
        }
    }
    merged_chunk_stems.push_front(current_stem.to_string());
    merged_chunk_indices.push_front(current_indices);

    // Drain merged_chunk_indices into fst_values
    result_index.fst_values.extend(merged_chunk_indices);
    println!("Finished merge: {} seconds", merge_start.elapsed().as_secs());


    println!("Building fst");
    let fst_start = Instant::now();
    let mut merged_counter = 0;
    for stem in merged_chunk_stems {
        build.insert(stem, merged_counter).unwrap();
        merged_counter += 1;
    }
    println!("Finished building fst: {} seconds", fst_start.elapsed().as_secs());
    build.finish().unwrap();
    println!("Finished writing fst: {} seconds (cumulative)", fst_start.elapsed().as_secs());
    return Some(result_index);
}


pub fn search_fst_index(term: &str, index: &FstIndex, max_group: usize) -> Vec<String> {
    let mmap = unsafe { Mmap::map(&File::open(&(index.fst_file)).unwrap()).unwrap() };
    let map = Map::new(mmap).unwrap();

    let association_file_map = unsafe { Mmap::map(&File::open(&(index.association_file)).unwrap()).unwrap() };

    let mut result: Vec<String> = Vec::new();
    let stems = stemmer::generate_stems(&term, max_group);
    for stem in stems {
        match map.get(&stem) {
            Some(fst_value_index) => {
                for orig_file_line in &(index.fst_values)[fst_value_index as usize] {
                    let line_num: usize = *orig_file_line as usize;
                    // Get byte offset from line_offsets
                    let start_offset = (index.line_starts)[line_num] as usize;
                    let end_offset = (index.line_starts)[line_num + 1] as usize;

                    let mut byte_vec: Vec<u8> = association_file_map[start_offset..end_offset].iter().cloned().collect();
                    let v: Value = simd_json::serde::from_slice(&mut byte_vec[..]).unwrap();
                    let pair = v.as_array().unwrap();
                    let title = pair[0].as_str().unwrap(); // unused but might be good for filtering
                    let article_array = pair[1].as_array().unwrap();
                    for article in article_array {
                        result.push(article.to_string());
                    }
                }
            },
            None => {}
        }
    }
    return result;
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

