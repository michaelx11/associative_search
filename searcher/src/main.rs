extern crate serde_json;
extern crate simd_json;
extern crate searcher;
extern crate fst;
extern crate httparse;

use std::thread;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::{self, BufRead, Read, Write};
use std::path::Path;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::net::TcpStream;
use std::net::TcpListener;

use serde_json::{json, Value};

use searcher::{indexer, stemmer, synonym_index};

use searcher::indexer::Searchable;

#[derive(Debug)]
enum QueryStage {
    WikiAllStem,
    WikiArticleStem,
    WikiArticleExact,
    Synonym,
    Homophone
}

struct Query {
    query_terms: Vec<String>,
    stages: Vec<QueryStage>,
    max_size: usize,
    association_dicts: Vec<AssociationDict>,
    // Purely for scoring, TODO: make this structured in some kind of sane way
    flavortext: Option<String>
}

#[derive(PartialEq, PartialOrd)]
struct ScorePair {
    score: f64,
    association: String
}

// This struct stores 1) original search term 2) the match
// e.g. book -> book of job
// this is to help us retrace our steps through association phases
#[derive(Debug)]
struct SearchMatch {
    search_term: String,
    search_match: String
}

type AssociationDict = HashMap<String, HashMap<String, SearchMatch>>;

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn find_associations(search_set: &[String], norm_index: &Arc<impl Searchable>, table_index: &Arc<impl Searchable>) -> AssociationDict {
    let mut association_dict: AssociationDict = HashMap::new();
    for term in search_set {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        let norm_results = norm_index.search(&term, 1, false);
        let table_results = table_index.search(&term, 1, false);
        for (search_child, search_match) in norm_results {
            entry.insert(search_child.to_string(), SearchMatch{search_term: term.to_string(), search_match: search_match.to_string()});
        }
        for (search_child, search_match) in table_results {
            entry.insert(search_child.to_string(), SearchMatch{search_term: term.to_string(), search_match: search_match.to_string()});
        }
    }
    return association_dict;
}

fn find_synonym_associations(search_set: &[String], index: &Arc<synonym_index::SynonymIndex>) -> AssociationDict {
    let mut association_dict: AssociationDict = HashMap::new();
    for term in search_set {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        let synonym_results = synonym_index::search_synonym_index(&term, index);
        for (syn, _) in synonym_results {
            // Need to map syn -> syn otherwise if we use 'term' we'll only get the last entry
            entry.insert(syn.to_string(), SearchMatch{search_term: term.to_string(), search_match: term.to_string()});
        }
    }
    return association_dict;
}

fn subfind_associations(associations: &AssociationDict, norm_index: &Arc<impl Searchable>) -> AssociationDict {
    // map[item]-> map[article]->(title found in the article)
    let mut association_dict: AssociationDict = HashMap::new();
    // Iterate through items in search set
    for (term, subassociations) in associations.iter() {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        for (orig_search_child, orig_search_match) in subassociations.iter() {

            let norm_results = norm_index.search(orig_search_child, 0, true);
            for (search_child, search_match) in norm_results {
                entry.insert(search_child.to_string(), SearchMatch{search_term: orig_search_child.to_string(), search_match: search_match.to_string()});
            }
        }
    }
    return association_dict;
}

fn subfind_synonyms(associations: &AssociationDict, index: &Arc<synonym_index::SynonymIndex>) -> AssociationDict {
    // map[item]-> map[article]->(title found in the article)
    let mut association_dict: AssociationDict = HashMap::new();
    // Iterate through items in search set
    for (term, subassociations) in associations.iter() {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        for (orig_search_child, orig_search_match) in subassociations.iter() {

            let synonym_results = synonym_index::search_synonym_index(&orig_search_child, index);
            for (search_child, search_match) in synonym_results {
                entry.insert(search_child.to_string(), SearchMatch{search_term: orig_search_child.to_string(), search_match: search_match.to_string()});
            }
        }
    }
    return association_dict;
}

fn subfind_associations_map(associations: &AssociationDict, norm_index: &Arc<impl Searchable>) -> AssociationDict {
    // map[item]-> map[article]->title
    let mut association_dict: AssociationDict = HashMap::new();
    // Iterate through items in search set
    for (term, subassociations) in associations.iter() {
        let entry = association_dict.entry(term.to_string()).or_insert_with(HashMap::new);
        for (orig_search_child, orig_search_match) in subassociations.iter() {

            // search returns <result entry, what matched that entry's key>
            // since this is subfind we do 0 stemming and include the whole string
            for (search_child, search_match) in norm_index.search(orig_search_child, 0, true) {
                entry.insert(search_child.to_string(), SearchMatch{search_term: orig_search_child.to_string(), search_match: search_match.to_string()});
            }
        }
    }
    return association_dict;
}

fn sum_subentries(map_of_maps: &AssociationDict) -> usize {
    let mut counter: usize = 0;
    for (_, submap) in map_of_maps {
        counter += submap.len();
    }
    return counter;
}

fn construct_chains(query: &Query, scored_pairs: Vec<ScorePair>) -> Vec<HashMap<String, Vec<String>>> {
    let mut all_results: Vec<HashMap<String, Vec<String>>> = Vec::new();
    let ARBITRARY_THRESHOLD = 100;
    let mut num_processed = 0;
    let last_association_dict = query.association_dicts.last().unwrap();
    for score_pair in scored_pairs {
        let mut match_chains: HashMap<String, Vec<String>> = HashMap::new();
        for item in query.query_terms.iter() {
            // last search match -> last search term -> previous search match -> previous search term
            let mut chain: Vec<String> = Vec::new();
            let item_string = item.to_string();
            let mut current_association = &score_pair.association;
            match last_association_dict[item].get(&score_pair.association) {
                Some(v) => {
                    // Start iterative construction
                    let mut current_match = v;
                    let num_stages = query.association_dicts.len();
                    for stage_num in (0..num_stages).rev() {
                        current_match = &query.association_dicts.get(stage_num).unwrap()[item][current_association];
                        chain.push(current_association.to_string());
                        chain.push(current_match.search_match.to_string());
                        chain.push(current_match.search_term.to_string());
                        chain.push(format!("{:?}", query.stages[stage_num]));
                        current_association = &current_match.search_term;
                    }
                },
                _ => {}
            };
            match_chains.insert(item_string, chain.iter().rev().cloned().collect());
        }
        num_processed += 1;
        println!("{}: {}: {:?}", score_pair.score, &score_pair.association, match_chains);
        all_results.push(match_chains);
        if num_processed > ARBITRARY_THRESHOLD {
            eprintln!("Terminating early at score: {}", score_pair.score);
            break;
        }
    }
    return all_results;
}

fn process_query(mut query_raw: Query,
                 norm_index: Arc<impl Searchable>,
                 table_index: Arc<impl Searchable>,
                 syn_index: Arc<synonym_index::SynonymIndex>,
                 homophone_index: Arc<synonym_index::SynonymIndex>) -> String {
    let query_start = Instant::now();
    let mut query = &mut query_raw;
    for stage in query.stages.iter() {
        let mut association_dict: AssociationDict = HashMap::new();
        if query.association_dicts.len() > 0 {
            let total_entries = sum_subentries(query.association_dicts.last().unwrap());
            if  total_entries > query.max_size {
                eprintln!("Aborting search as {} > maximum size {} for any association stage was exceeded.", total_entries, query.max_size);
                return format!("{{\"error\": \"maximum working size {} exceeded max {} for stage: {:?} (#{})\"}}", total_entries, query.max_size, stage, query.association_dicts.len());
            }
        }
        println!("stage: {:?}", stage);
        match stage {
            QueryStage::WikiAllStem => {
                eprintln!("WikiAll Stage");
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_associations(&query.query_terms[..], &norm_index, &table_index));
                    query.association_dicts.push(association_dict);
                } else {
                    eprintln!("Cannot do subfind on all wiki indexes, use WikiArticleRefs insead");
                }
            },
            QueryStage::WikiArticleStem => {
                if query.association_dicts.len() == 0 {
                    // TODO: fix this double index hack
                    association_dict.extend(find_associations(&query.query_terms[..], &norm_index, &norm_index));
                    query.association_dicts.push(association_dict);
                } else {
                    let latest_associations = &query.association_dicts.last().unwrap();
                    eprintln!("WikiArticleStem subfind stage with {} associations", sum_subentries(latest_associations));
                    association_dict.extend(subfind_associations(latest_associations, &norm_index));
                    query.association_dicts.push(association_dict);
                }
            },
            QueryStage::WikiArticleExact => {
                let latest_associations = &query.association_dicts.last().unwrap();
                eprintln!("WikiArticleExact subfind stage with {} associations", sum_subentries(latest_associations));
                association_dict.extend(subfind_associations_map(latest_associations, &norm_index));
                query.association_dicts.push(association_dict);
            },
            QueryStage::Synonym => {
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_synonym_associations(&query.query_terms[..], &syn_index));
                    query.association_dicts.push(association_dict);
                } else {
                    let latest_associations = &query.association_dicts.last().unwrap();
                    eprintln!("Synonym subfind stage with {} associations", sum_subentries(latest_associations));
                    association_dict.extend(subfind_synonyms(latest_associations, &syn_index));
                    query.association_dicts.push(association_dict);
                }
            },
            QueryStage::Homophone => {
                if query.association_dicts.len() == 0 {
                    association_dict.extend(find_synonym_associations(&query.query_terms[..], &homophone_index));
                    println!("homophone associations: {:?}", &association_dict);
                    query.association_dicts.push(association_dict);
                } else {
                    let latest_associations = &query.association_dicts.last().unwrap();
                    eprintln!("Synonym subfind stage with {} associations", sum_subentries(latest_associations));
                    association_dict.extend(subfind_synonyms(latest_associations, &homophone_index));
                    query.association_dicts.push(association_dict);
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
    // Need to sort f64s that don't implement Eq (damn you Rust), we no there are no NaNs
    scored_pairs.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    println!("Total scored associations: {}", scored_pairs.len());
    return json!(construct_chains(&query, scored_pairs)).to_string();
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
            "Homophone" => stages.push(QueryStage::Homophone),
            _ => {}
        }
    }
    let max_size: usize = 100000;
    let association_dicts: Vec<AssociationDict> = Vec::new();
    let mut flavortext: Option<String> = None;
    if flavortext_str.len() > 0 {
        flavortext = Some(flavortext_str.to_string());
    }
    println!("num stages: {}", stages.len());
    return Query{query_terms, stages, max_size, association_dicts, flavortext};
}

fn parse_http_query(body: &mut [u8]) -> Query {
    println!("body: {:?}", body);
    let v: Value = simd_json::serde::from_slice(body).unwrap();
    println!("{:?}", v);
    let object = v.as_object().unwrap();
    // Parse query stages array
    let query_stages_array = object.get("stages").unwrap().as_array().unwrap();
    // Parse terms
    let query_terms_array = object.get("terms").unwrap().as_array().unwrap();
    // Parse flavortext
    let flavortext_value = object.get("flavortext");
    println!("query_stages: {:?}", query_stages_array);
    println!("query_terms: {:?}", query_terms_array);
    println!("flavortext: {:?}", flavortext_value);

    // Get query set, split by ","
    let mut query_terms: Vec<String> = Vec::new();
    for term_value in query_terms_array {
        query_terms.push(term_value.as_str().unwrap().to_string());
    }
    let mut stages: Vec<QueryStage> = Vec::new();
    for stage_value in query_stages_array {
        let stage_str = stage_value.as_str().unwrap();
        match stage_str {
            "WikiAllStem" => stages.push(QueryStage::WikiAllStem),
            "WikiArticleStem" => stages.push(QueryStage::WikiArticleStem),
            "WikiArticleExact" => stages.push(QueryStage::WikiArticleExact),
            "Synonym" => stages.push(QueryStage::Synonym),
            "Homophone" => stages.push(QueryStage::Homophone),
            _ => {}
        }
    }
    let max_size: usize = 100000;
    let association_dicts: Vec<AssociationDict> = Vec::new();
    let mut flavortext: Option<String> = None;
    match flavortext_value {
        Some(flavortext_json_value) => {
            flavortext = Some(flavortext_json_value.to_string());
        },
        None => {}
    }
    return Query{query_terms, stages, max_size, association_dicts, flavortext};
}

fn handle_connection(mut stream: TcpStream,
                     norm_index: Arc<impl Searchable>,
                     table_index: Arc<impl Searchable>,
                     syn_index: Arc<synonym_index::SynonymIndex>,
                     homophone_index: Arc<synonym_index::SynonymIndex>) {
    let mut buffer = [0; 256 * 1024];
    stream.read(&mut buffer).unwrap();

    let mut headers = [httparse::EMPTY_HEADER; 16];
    let mut req = httparse::Request::new(&mut headers);
    let res = req.parse(&buffer).unwrap();
    if !res.is_partial() {
        let response = "HTTP/1.1 200 OK\r\n\r\n";
        println!("Method: {:?}", req.method.unwrap_or("missing"));
        println!("Path: {:?}", req.path.unwrap_or("no path"));
        let start_body = res.unwrap();
        let mut end_body = start_body;
        for i in start_body..buffer.len() {
            if buffer[i] != 0 {
                end_body += 1;
            } else {
                break;
            }
        }
        let body: &mut [u8] = &mut buffer[res.unwrap()..end_body];
        let query = parse_http_query(body);
        let res = process_query(query, norm_index, table_index, syn_index, homophone_index);
        stream.write(format!("{}{}", response, res).as_bytes()).unwrap();
        stream.flush().unwrap();
    } else {
        let response = "HTTP/1.1 413 Payload Too Large\r\n\r\n";
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}

fn main() {
    // first arg: filename, remaining args go into search set
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: ./searcher [filename] [search set size] [item1] [item2] ...");
        return;
    }
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();
    let filename = &args[1];
    let threshold = args[2].parse::<usize>().unwrap();
    // Search set is a list of search items
    let search_set = &args[3..args.len()];
    eprintln!("filename: {:?}, threshold: {:?}, search set: {:?}", filename, threshold, search_set);
    let table_index_filename = "big_table_index.txt";
    let norm_index_filename = "big_norm_index.txt";
    let synonym_index_filename = "moby_words.txt";
    let homophone_index_filename = "homophone_list.txt";
    let now = Instant::now();
    let syn_index = Arc::new(synonym_index::generate_synonym_index(synonym_index_filename));
    let homophone_index = Arc::new(synonym_index::generate_synonym_index(homophone_index_filename));
    let table_index = Arc::new(indexer::generate_fst_index(table_index_filename, 1, false).unwrap());
    let norm_index = Arc::new(indexer::generate_inmemory_index(norm_index_filename, 1, true));
    println!("finished indexing in {}s", now.elapsed().as_secs());
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        let thread_table_index = table_index.clone();
        let thread_norm_index = norm_index.clone();
        let thread_syn_index = syn_index.clone();
        let thread_homophone_index = homophone_index.clone();
        thread::spawn(|| {
            handle_connection(stream, thread_norm_index, thread_table_index, thread_syn_index, thread_homophone_index);
        });
        println!("Connection established!");
    }
}
