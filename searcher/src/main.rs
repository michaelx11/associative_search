extern crate serde_json;
extern crate simd_json; 
extern crate searcher;
extern crate fst;

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
    stages: Vec<QueryStage>,
    max_size: usize,
    association_dicts: Vec<HashMap<String, HashMap<String, String>>>
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
        let norm_results = indexer::search_fst_index(&term, &norm_index, 1, false);
        let table_results = indexer::search_fst_index(&term, &table_index, 1, false);
        for (article, title) in norm_results {
            entry.insert(article.to_string(), title.to_string());
        }
        for (article, title) in table_results {
            entry.insert(article.to_string(), title.to_string());
        }
    }
    return association_dict;
}

fn subfind_associations(associations: &HashMap<String, HashMap<String, String>>, norm_index: &indexer::FstIndex) -> HashMap<String, HashMap<String, String>> {
    // map[item]-> map[article]->title
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    // Iterate through items in search set
    for (term, subassociations) in associations.iter() {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        for (_, match_title) in subassociations.iter() {

            let title_match_key = match_title.to_string();
            let norm_results = indexer::search_fst_index(match_title, &norm_index, 0, true);
            for (article, title) in norm_results {
                entry.insert(article.to_string(), title.to_string());
            }
        }
    }
    return association_dict;
}

fn subfind_associations_map(associations: &HashMap<String, HashMap<String, String>>, norm_index: &indexer::InMemoryIndex) -> HashMap<String, HashMap<String, String>> {
    // map[item]-> map[article]->title
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    // Iterate through items in search set
    for (term, subassociations) in associations.iter() {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        for (_, match_title) in subassociations.iter() {

            let title_match_key = match_title.to_string();
            match (norm_index.index.get(match_title)) {
                Some(norm_results) => {
                    for (article) in norm_results {
                        entry.insert(article.to_string(), match_title.to_string());
                    }
                },
                None => {}
            }
        }
    }
    return association_dict;
}

fn sum_subentries(map_of_maps: &HashMap<String, HashMap<String, String>>) -> usize {
    let mut counter: usize = 0;
    for (_, submap) in map_of_maps {
        counter += submap.len();
    }
    return counter;
}

fn process_query(query: &mut Query, norm_index: &indexer::FstIndex, table_index: &indexer::FstIndex, inmem_index: &indexer::InMemoryIndex) -> String {
    let query_start = Instant::now();
    for stage in query.stages.iter() {
        let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
        if query.association_dicts.len() > 0 {
            let total_entries = sum_subentries(query.association_dicts.last().unwrap());
            if  total_entries > query.max_size {
                eprintln!("Aborting search as {} > maximum size {} for any association stage was exceeded.", total_entries, query.max_size);
                break;
            }
        }
        match stage {
            QueryStage::WikiAll => {
                eprintln!("WikiAll Stage");
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_associations(&query.query_terms[..], norm_index, table_index));
                    query.association_dicts.push(association_dict);
                } else {
                    eprintln!("Cannot do subfind on all wiki indexes, use WikiArticleRefs insead");
                }
            },
            QueryStage::WikiArticleRefs => {
                // TODO: fix this double index hack
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_associations(&query.query_terms[..], norm_index, norm_index));
                    query.association_dicts.push(association_dict);
                } else {
                    let latest_associations = &query.association_dicts.last().unwrap();
                    eprintln!("WikiArticleRefs subfind stage with {} associations", sum_subentries(latest_associations));
                    association_dict.extend(subfind_associations_map(latest_associations, inmem_index));
                    query.association_dicts.push(association_dict);
                }
            },
            QueryStage::Synonym => {
                // TODO: implement
            },
        }
        eprintln!("stage finished: {}s", query_start.elapsed().as_secs());
    }
    // Finally, we check if we got any good associations
    let mut association_count_dict: HashMap<String, usize> = HashMap::new();
    let last_association_dict = query.association_dicts.last().unwrap();
    for item in query.query_terms.iter() {
        match last_association_dict.get(item) {
            Some(entry) => {
                for (key, value) in entry {
                    let key_string = key.to_string();
                    association_count_dict.entry(key_string).and_modify(|e| {*e += 1}).or_insert(1);
                }
            }
            None => {}
        }
    }
    for (assoc, count) in association_count_dict {
        if count >= query.query_terms.len() {
            for item in query.query_terms.iter() {
                let item_string = item.to_string();
                let assoc_string = assoc.to_string();
                println!("{}: {}", assoc, last_association_dict[&item_string].get(&assoc).unwrap_or(&"[NONE]".to_string()));
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
    stages.push(QueryStage::WikiArticleRefs);
    let max_size: usize = 100000;
    let association_dicts: Vec<HashMap<String, HashMap<String, String>>> = Vec::new();
    return Query{query_terms, stages, max_size, association_dicts};
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
    let table_index = indexer::generate_fst_index(table_index_filename, 1, false).unwrap();
    let norm_index = indexer::generate_fst_index(norm_index_filename, 1, true).unwrap();
    let inmemory_index = indexer::generate_inmemory_index(norm_index_filename);
    println!("finished indexing in {}s", now.elapsed().as_secs());
    while true {
        let mut line = String::new();
        println!("Type search term, then enter>");
        let stdin = io::stdin();
        stdin.lock().read_line(&mut line).unwrap();
        println!("Searching: {}", &line);
        let mut query = parse_interactive_query(&line);
        let results = process_query(&mut query, &norm_index, &table_index, &inmemory_index);
        println!("Results: {:?}", results);
    }
}
