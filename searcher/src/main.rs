extern crate serde_json;
extern crate simd_json; 

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use serde_json::Value;

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn main() {
    // first arg: filename, remaining args go into search set
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: ./searcher [filename] [search set size] [item1] [item2] ...");
        return;
    }
    let filename = &args[1];
    let threshold = args[2].parse::<usize>().unwrap();
    // Search set is a list of search items
    let search_set = &args[3..args.len()];
    eprintln!("filename: {:?}, threshold: {:?}, search set: {:?}", filename, threshold, search_set);
    // map[item]-> map[article]->title
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    // Need a mapping from items to article
    if let Ok(lines) = read_lines(filename) {
        for line in lines {
            if let Ok(entry) = line {
                let mut mutable_bytes = entry.into_bytes();
                let v: Value = simd_json::serde::from_slice(&mut mutable_bytes).unwrap();
                let pair = v.as_array().unwrap();
                let title = pair[0].as_str().unwrap();
                let article_array = pair[1].as_array().unwrap();
                // Iterate through items in search set
                for item in search_set.iter() {
                    let item_key = item.to_string();
                    // SELECTION CRITERIA - does title match item?
                    if title.contains(&item_key) {
                        let entry = association_dict.entry(item_key).or_insert_with(HashMap::new);
                        // If so, go ahead and add articles->title
                        for article in article_array.iter() {
                            let article_string = article.as_str().unwrap();
                            entry.insert(article_string.to_string(), title.to_string());
                        }
                    }
                }
            }
        }
    }
    // Finally, we check if we got any good associations
    let mut association_count_dict: HashMap<String, usize> = HashMap::new();
    for item in search_set.iter() {
        let item_key = item.to_string();
        for (key, value) in association_dict.entry(item_key).or_insert_with(HashMap::new) {
            let key_string = key.to_string();
            association_count_dict.entry(key_string).and_modify(|e| {*e += 1}).or_insert(1);
        }
    }
    for (assoc, count) in association_count_dict {
        if count >= threshold {
            for item in search_set.iter() {
                let item_string = item.to_string();
                let assoc_string = assoc.to_string();
                println!("{}: {}", assoc, association_dict[&item_string].get(&assoc).unwrap_or(&"[NONE]".to_string()));
            }
        }
    }
//    let mut d = br#"{"some": ["key", "value", 2]}"#.to_vec();
//    let v: simd_json::BorrowedValue = simd_json::to_borrowed_value(&mut d).unwrap();
//    println!("Hello, world!");
}
