extern crate serde_json;
extern crate simd_json; 
extern crate searcher;

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::Value;

use searcher::indexer;

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
fn find_associations(search_set: &[String], preloaded_lines: &[Vec<String>]) -> HashMap<String, HashMap<String, String>> {
    // map[item]-> map[article]->title
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    for article_vec in preloaded_lines {
        let title = article_vec[0].to_string();
        // Iterate through items in search set
        for item in search_set.iter() {
            let item_key = item.to_string();
            // SELECTION CRITERIA - does title match item?
            if title.contains(&item_key) {
                let entry = association_dict.entry(item_key).or_insert_with(HashMap::new);
                // If so, go ahead and add articles->title
                for article in article_vec[1..article_vec.len()].iter() {
                    let article_string = article.as_str();
                    entry.insert(article_string.to_string(), title.to_string());
                }
            }
        }
    }
    return association_dict;
}

fn subfind_associations(associations: &HashMap<String, HashMap<String, String>>, preloaded_lines: &[Vec<String>]) -> HashMap<String, HashMap<String, String>> {
    // map[item]-> map[article]->title
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    for article_vec in preloaded_lines {
        let title = article_vec[0].to_string();
        // Iterate through items in search set
        for (term, subassociations) in associations.iter() {
            for (_, match_title) in subassociations.iter() {
                let title_match_key = match_title.to_string();
                // SELECTION CRITERIA - does title match item?
                if title.contains(&title_match_key) {
                    let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
                    // If so, go ahead and add articles->title
                    for article in article_vec[1..article_vec.len()].iter() {
                        let article_string = article.as_str();
                        entry.insert(article_string.to_string(), title.to_string());
                    }
                }
            }
        }
    }
    return association_dict;
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
    let now = Instant::now();
    let index = indexer::generate_fst_index(filename, 1).unwrap();
    println!("finished indexing in {}s", now.elapsed().as_secs());
    while true {
        let mut line = String::new();
        println!("Type search term, then enter>");
        let stdin = io::stdin();
        stdin.lock().read_line(&mut line).unwrap();
        println!("Searching: {}", &line);
        let results = indexer::search_fst_index(&line, &index, 1);
        println!("Results: {:?}", results);
    }
//    // Need a mapping from items to article
//    if let Ok(lines) = read_lines(filename) {
//        for line in lines {
//            if let Ok(entry) = line {
//                let mut mutable_bytes = entry.into_bytes();
//                let v: Value = simd_json::serde::from_slice(&mut mutable_bytes).unwrap();
//                let pair = v.as_array().unwrap();
//                let title = pair[0].as_str().unwrap();
//                let article_array = pair[1].as_array().unwrap();
//                let mut article_vec = vec![title.to_string()];
//                for article in article_array.iter() {
//                    let article_string = article.as_str().unwrap();
//                    article_vec.push(article_string.to_string());
//                }
//                preloaded_lines.push(article_vec);
//            }
//        }
//    }
//    println!("finished preloading in {}s", now.elapsed().as_secs());
//    let search_now = Instant::now();
//    let mut first_level = find_associations(&search_set, &preloaded_lines);
//    println!("finished first level in {}s", search_now.elapsed().as_secs());
//    for (term, map) in &first_level {
//        println!("Term: {}, {:?}", term, map);
//    }
//    let second_stage = Instant::now();
//    let mut association_dict = subfind_associations(&first_level, &preloaded_lines);
//    println!("finished second level in {}s", second_stage.elapsed().as_secs());
//    // Finally, we check if we got any good associations
//    let mut association_count_dict: HashMap<String, usize> = HashMap::new();
//    for item in search_set.iter() {
//        let item_key = item.to_string();
//        for (key, value) in association_dict.entry(item_key).or_insert_with(HashMap::new) {
//            let key_string = key.to_string();
//            association_count_dict.entry(key_string).and_modify(|e| {*e += 1}).or_insert(1);
//        }
//    }
//    for (assoc, count) in association_count_dict {
//        if count >= threshold {
//            for item in search_set.iter() {
//                let item_string = item.to_string();
//                let assoc_string = assoc.to_string();
//                println!("{}: {}", assoc, association_dict[&item_string].get(&assoc).unwrap_or(&"[NONE]".to_string()));
//            }
//        }
//    }
}
