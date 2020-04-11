extern crate serde_json;
extern crate simd_json;
extern crate searcher;
extern crate fst;

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::time::{Duration, Instant};

use serde_json::Value;

use searcher::{indexer, stemmer, synonym_index};

use searcher::indexer::Searchable;

enum QueryStage {
    WikiAllStem,
    WikiArticleStem,
    WikiArticleExact,
    Synonym
}

struct Query {
    query_terms: Vec<String>,
    stages: Vec<QueryStage>,
    max_size: usize,
    association_dicts: Vec<HashMap<String, HashMap<String, String>>>,
    // Purely for scoring, TODO: make this structured in some kind of sane way
    flavortext: Option<String>
}

#[derive(PartialEq, PartialOrd)]
struct ScorePair {
    score: f64,
    association: String
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
        let norm_results = norm_index.search(&term, 1, false);
        let table_results = table_index.search(&term, 1, false);
        for (article, title) in norm_results {
            entry.insert(article.to_string(), title.to_string());
        }
        for (article, title) in table_results {
            entry.insert(article.to_string(), title.to_string());
        }
    }
    return association_dict;
}

fn find_synonym_associations(search_set: &[String], index: &synonym_index::SynonymIndex) -> HashMap<String, HashMap<String, String>> {
    let mut association_dict: HashMap<String, HashMap<String, String>> = HashMap::new();
    for term in search_set {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        let synonym_results = synonym_index::search_synonym_index(&term, index);
        for (syn, _) in synonym_results {
            entry.insert(syn.to_string(), syn.to_string());
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
            let norm_results = norm_index.search(match_title, 0, true);
            println!("search term: {}, num results: {}", match_title, norm_results.len());
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
        for (_, prev_title) in subassociations.iter() {

            // search returns <result entry, what matched that entry's key>
            // since this is subfind we do 0 stemming and include the whole string
            for (matching_result, prev_title_stem) in norm_index.search(prev_title, 0, true) {
                entry.insert(matching_result.to_string(), prev_title_stem.to_string());
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

fn process_query(query: &mut Query, norm_index: &indexer::FstIndex, table_index: &indexer::FstIndex, inmem_index: &indexer::InMemoryIndex, syn_index: &synonym_index::SynonymIndex) -> String {
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
            QueryStage::WikiAllStem => {
                eprintln!("WikiAll Stage");
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_associations(&query.query_terms[..], norm_index, table_index));
                    query.association_dicts.push(association_dict);
                } else {
                    eprintln!("Cannot do subfind on all wiki indexes, use WikiArticleRefs insead");
                }
            },
            QueryStage::WikiArticleStem => {
                if query.association_dicts.len() == 0 {
                    // TODO: fix this double index hack
                    association_dict.extend(find_associations(&query.query_terms[..], norm_index, norm_index));
                    query.association_dicts.push(association_dict);
                } else {
                    let latest_associations = &query.association_dicts.last().unwrap();
                    eprintln!("WikiArticleStem subfind stage with {} associations", sum_subentries(latest_associations));
                    association_dict.extend(subfind_associations(latest_associations, norm_index));
                    query.association_dicts.push(association_dict);
                }
            },
            QueryStage::WikiArticleExact => {
                let latest_associations = &query.association_dicts.last().unwrap();
                eprintln!("WikiArticleExact subfind stage with {} associations", sum_subentries(latest_associations));
                association_dict.extend(subfind_associations_map(latest_associations, inmem_index));
                query.association_dicts.push(association_dict);
            },
            QueryStage::Synonym => {
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_synonym_associations(&query.query_terms[..], syn_index));
                    query.association_dicts.push(association_dict);
                } else {
                    eprintln!("Cannot do subfind on all wiki indexes, use WikiArticleRefs insead");
                }
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
    // Stem the flavortext
    let mut flavortext_set: HashSet<String> = HashSet::new();
    let mut use_flavortext_filter = false;
    match &query.flavortext {
        Some(flavortext) => {
            // max stem group 1 (word by word) and do not include the entire text (false)
            for stem in stemmer::generate_stems(&flavortext, 1, false) {
                flavortext_set.insert(stem.to_string());
            }
            use_flavortext_filter = true;
        },
        None => {
            use_flavortext_filter = false;
        }
    }
    // TODO: add scoring based on flavortext if it exists
    let mut scored_pairs: Vec<ScorePair> = Vec::new();
    for (assoc, count) in association_count_dict {
        // Score each association
        // Our scoring approach is a bit qualitative:
        // - Imagine we get 100k 5/5 matches (synonym expansion) with no thematic filter,
        // then count is completely useless.
        // - On the other hand, if we get 1 5/5 match and 5 4/5 matches, maybe we don't care
        // so much about theme. However, we may not need to quantify this because we're always
        // going to display a limited number of results and we can just display all of them.
        // - Problem is we'll get millions of 1/5 and 2/5 matches
        // - So maybe we just sort by count first, threshold, then apply thematic scoring
        // - That's bad again in the 100k 5/5 match case, it'll fill the threshold immediately
        // before thematic scoring occurs, but maybe that's okay because theme really doesn't
        // matter if it's 0/5, 1/5, 2/5 etc. There are just too many of those matches.
        // - Do both signals independently and use the one that provides more information? (higher
        // selectivity)
        // - For now, score is straight up (count) + ((# thematic)/(# words) in association)

        // Debate aside, we can safely ignore 0 or 1 matches
        if count <= 1 {
            continue;
        }

        let mut score: f64 = (count as f64) * 100.0;
        if use_flavortext_filter {
            let mut assoc_stems: Vec<String> = Vec::new();
            let mut thematic_stems: f64 = 0.0;
            for stem in stemmer::generate_stems(&assoc, 1, false) {
                match flavortext_set.get(&stem) {
                    Some(_) => {thematic_stems += 1.0},
                    None => {}
                }
                assoc_stems.push(stem);
            }
            score += thematic_stems;
//            let total_stems: usize = assoc_stems.len();
//            if total_stems == 0 {
//                score = 0.0;
//            } else {
//                score += thematic_stems / (total_stems as f64);
//            }
        }
        scored_pairs.push(ScorePair{score: score, association: assoc.to_string()});
    }
    println!("Total scored associations: {}", scored_pairs.len());
    // Need to sort f64s that don't implement Eq (damn you Rust), we no there are no NaNs
    scored_pairs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let ARBITRARY_THRESHOLD = 100;
    let mut num_displayed = 0;
    for score_pair in scored_pairs {
        let mut display_map: HashMap<String, String> = HashMap::new();
        for item in query.query_terms.iter() {
            let item_string = item.to_string();
            let last_match: String = match last_association_dict[item].get(&score_pair.association) {
                Some(v) => v.to_string(),
                _ => "[NONE]".to_string()
            };
            display_map.insert(item_string, last_match.to_string());
        }
        num_displayed += 1;
        println!("{}: {}: {:?}", score_pair.score, &score_pair.association, display_map);
        if num_displayed > ARBITRARY_THRESHOLD {
            println!("Terminating early at score: {}", score_pair.score);
            break;
        }
    }

    return "".to_string();
}

fn parse_interactive_query(query_terms_str: &str, query_stages_str: &str, flavortext_str: &str) -> Query {
    // Get query set, split by ","
    let mut query_terms: Vec<String> = Vec::new();
    for term in query_terms_str.split(",") {
        query_terms.push(term.to_string());
    }
    let mut stages: Vec<QueryStage> = Vec::new();
    for term in query_stages_str.split(",") {
        match term {
            "WikiAllStem" => stages.push(QueryStage::WikiAllStem),
            "WikiArticleStem" => stages.push(QueryStage::WikiArticleStem),
            "WikiArticleExact" => stages.push(QueryStage::WikiArticleExact),
            "Synonym" => stages.push(QueryStage::Synonym),
            _ => {}
        }
    }
    let max_size: usize = 100000;
    let association_dicts: Vec<HashMap<String, HashMap<String, String>>> = Vec::new();
    let mut flavortext: Option<String> = None;
    if flavortext_str.len() > 0 {
        flavortext = Some(flavortext_str.to_string());
    }
    return Query{query_terms, stages, max_size, association_dicts, flavortext};
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
    let synonym_index_filename = "moby_words.txt";
    let now = Instant::now();
    let syn_index = synonym_index::generate_synonym_index(synonym_index_filename);
    eprintln!("results: {:?}", synonym_index::search_synonym_index("pronouncement", &syn_index));
    let table_index = indexer::generate_fst_index(table_index_filename, 1, false).unwrap();
    let norm_index = indexer::generate_fst_index(norm_index_filename, 1, true).unwrap();
//    let norm_index = indexer::generate_inmemory_index(norm_index_filename, 1, true);
    let inmemory_index = indexer::generate_inmemory_index(norm_index_filename, 0, true);
    println!("finished indexing in {}s", now.elapsed().as_secs());
    while true {
        let mut search_terms_line = String::new();
        let mut query_stages_line = String::new();
        let mut flavortext = String::new();
        println!("Type comma-separated search terms, then enter>");
        let stdin = io::stdin();
        stdin.lock().read_line(&mut search_terms_line).unwrap();
        println!("Type comma-separate search stages [WikiAllStem or Synonym (both as first only), WikiArticleStem, WikiArticleExact]>");
        stdin.lock().read_line(&mut query_stages_line).unwrap();
        println!("Type any flavortext to filter by (single line):");
        stdin.lock().read_line(&mut flavortext).unwrap();
        search_terms_line = search_terms_line.trim().to_string();
        query_stages_line = query_stages_line.trim().to_string();
        flavortext = flavortext.trim().to_string();
        println!("Searching [{}] in stages [{}], with flavortext: [{}]", &search_terms_line, &query_stages_line, &flavortext);
        let mut query = parse_interactive_query(&search_terms_line, &query_stages_line, &flavortext);
        let results = process_query(&mut query, &norm_index, &table_index, &inmemory_index, &syn_index);
        println!("Results: {:?}", results);
    }
}
