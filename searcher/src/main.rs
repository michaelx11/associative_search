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

enum QueryStage {
    WikiAll,
    WikiArticleRefs,
    Synonym
}

struct Query {
    query_terms: Vec<String>,
    stages: Vec<QueryStage>
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
fn find_associations(search_set: &[String], norm_index: &indexer::FstIndex, table_index: &indexer::FstIndex) -> HashMap<String, HashMap<String, String>> {
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    for term in search_set {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        let norm_results = indexer::search_fst_index(&term, &norm_index, 1);
        let table_results = indexer::search_fst_index(&term, &table_index, 1);
        for (article, title) in norm_results {
            entry.insert(article.to_string(), title.to_string());
        }
        for (article, title) in table_results {
            entry.insert(article.to_string(), title.to_string());
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

fn process_query(query: &Query, norm_index: &indexer::FstIndex, table_index: &indexer::FstIndex) -> String {
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    for stage in query.stages.iter() {
        match stage {
            QueryStage::WikiAll => {
                association_dict.extend(find_associations(&query.query_terms[..], norm_index, table_index));
            },
            QueryStage::WikiArticleRefs => {
                // TODO: fix this hack
                association_dict.extend(find_associations(&query.query_terms[..], norm_index, norm_index));
            },
            QueryStage::Synonym => {
                // TODO: implement
            },
        }
    }
    // Finally, we check if we got any good associations
    let mut association_count_dict: HashMap<String, usize> = HashMap::new();
    for item in query.query_terms.iter() {
        let item_key = item.to_string();
        for (key, value) in association_dict.entry(item_key).or_insert_with(HashMap::new) {
            let key_string = key.to_string();
            association_count_dict.entry(key_string).and_modify(|e| {*e += 1}).or_insert(1);
        }
    }
    for (assoc, count) in association_count_dict {
        if count >= query.query_terms.len() {
            for item in query.query_terms.iter() {
                let item_string = item.to_string();
                let assoc_string = assoc.to_string();
                println!("{}: {}", assoc, association_dict[&item_string].get(&assoc).unwrap_or(&"[NONE]".to_string()));
            }
        }
    }
    return "".to_string();
}

fn parse_interactive_query(query: &str) -> Query {
    // Get query set, split by ","
    let mut query_terms: Vec<String> = Vec::new();
    for term in query.split(",") {
        query_terms.push(term.to_string());
    }
    let mut stages: Vec<QueryStage> = Vec::new();
    stages.push(QueryStage::WikiAll);
    return Query{query_terms, stages};
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
    let table_index_filename = "big_table_index.txt";
    let norm_index_filename = "big_norm_index.txt";
    let now = Instant::now();
    let table_index = indexer::generate_fst_index(table_index_filename, 1).unwrap();
    let norm_index = indexer::generate_fst_index(norm_index_filename, 1).unwrap();
    println!("finished indexing in {}s", now.elapsed().as_secs());
    while true {
        let mut line = String::new();
        println!("Type search term, then enter>");
        let stdin = io::stdin();
        stdin.lock().read_line(&mut line).unwrap();
        println!("Searching: {}", &line);
        let query = parse_interactive_query(&line);
        let results = process_query(&query, &norm_index, &table_index);
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
