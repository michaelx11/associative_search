# Associative Search

The goal is to build a tool that can find associations between elements of a set. For example, finding out that all the words in the set belong to movie titles from the 1960's.

Why can't we just use Google? Google will work great if all the elements show up together on some page e.g. "List of Movies for 1960's", but any indirect associations are not so easily recovered.

Example: What do "drugstore" and "urban" have in common? Turns out "drugstore cowboy" and "urban cowboy" are both drama films from the 1980's.

The basic approach is:

1) Index wikipedia (thank you [Nutrimatic](https://nutrimatic.org/) for inspiration and [WikiExtractor](https://github.com/attardi/wikiextractor) for an excellent tool)
2) Try to find all articles referencing each original term, gathering a set of matching articles for each term.
3) Look for any common articles shared by a majority of original terms after previous expansion.
4) If found, return otherwise repeat recursively until desired depth or computation limit.

This search is not terribly efficient and has an aggressive branching factor; however, even sometimes two or three level deep search is enough. Additionally, more selective search modes are provided. Instead of all wiki articles referencing a term we can return just articles that reference a term directly as a link.

# Run Instructions

Warning: This application takes ~20 GB of disk space and ~30 GB of memory to run. It's best run in a cloud environment (GCE n1-highmem-4 or n1-highmem-8)

1. Install rust: https://www.rust-lang.org/tools/install
2a. Download and unzip two files (13GB) to top-level (TODO: add instructions to reconstruct from wikipedia dump directly)
- https://storage.googleapis.com/michaelx_wikipedia_dumps/big_table_index.txt.tar.gz
- https://storage.googleapis.com/michaelx_wikipedia_dumps/big_norm_index.txt.tar.gz
2b. To build indexes from scratch, download the [latest English wikimedia dump](https://dumps.wikimedia.org/enwiki/latest/enwiki-latest-pages-articles.xml.bz2)
- Run the modified WikiExtractor.py code on it to generate a "condensed.csv" file. This is not a CSV, it's just bad naming.
- Create a `table_indexes` folder and then run `condensed_article_searcher.py`
- Finally run `index_merge.py` to end up with a `big_table_index.txt`
- The final `big_table_index.txt` is used by the Rust `searcher` application
3. cd searcher && cargo build --release
4. from repo top-level: `./searcher/target/release/searcher [port number e.g. 7777]`
- NOTE 1: the server will create *.fst and *.map files taking about 5GB of disk space
- NOTE 2: the server uses 29.6 GB of memory by default, you can reduce this by going into searcher/src/main.rs and removing indexes and stuff in a hacky way

# Random Musings

1. A list of pairs, each item in a pair is part of a movie title or something similar
2. All the items in the list appear in Sci-Fi Movies
3. Brick and Mortar Mortar and Pestle
4. Alabama -> Heart of Dixie and Character Flaw -> Feet of Clay (this is much harder and may require an association score on format or something)
could also just show lists of synonyms in place of the actual text

To do this, we need to do the hard work of creating and curating an index of:

- novels (not done)
- movies (not done)
- tv shows and episodes (not done)
- animal species (not done)
- us cities (not done)
- sports teams (not done)
- presidents (not done)
- congressmen (not done)
- world cities (not done)
- vehicle models (not done)
- fast food brand (not done)
- songs (not done)
- artists (not done)

(Questionable value for work)
Another aspect of this is to pre-curate visual examples of certain things that are relatively
small that we could search through.

In Scope:

Things that don't change frequently:
- flags of the world
- state flags
- brand logos
- product images

Not in scope (anything that reverse image search would do well on):
- animal species

Actually instead of building all these indexes by hand, we should just support the following API:

- list of items as text lines -> MAP -> to wikipedia article (with some fuzziness) -> FILTER -> fields for extraction -> POST PROCESSING

1. Identify lists in Wikipedia
2. Use manually curated (carefully time-dated lists)
3. Pull from other online list sources (IMDB, etc)

Where possible, if metadata exists within a list also categorize as sublists

How to efficiently represent lists? Tags on individual items or list tables or what?

Anyways: The key operation here is to perform an association search:

For each item in the puzzle -> PERFORM DERIVATIONS -> feed group into single level list containment search with some threshold or feed group into list of lists containment search either via
manually inputted list, list full text search or specifically specifying an object as a list of lists.

Return any groups that contain subitems above a certain threshold number or percentage.

DERIVED LISTS: for a given article set do an extraction on the article contents to create a derived list from certain attributes (tagged: sci-fi for example). Only
if the given list doesn't already exist.

Synonym association search (same logic but with a word graph) -> MAP back into WIKIPEDIA articles

custom DSL?

[orig list] -> EXPAND (python code) -> MAP -> 

(or write this in golang)?

Transforms (expansions? extensions?):

Raw association of concepts can usually be determined via a google search with some set of them. To get more value, we need to
perform various transforms that would be tedious to do by hand (or require actual knowledge).

- Synonym Transform
- Homonym Transform
- Partial Match Transform
- Indirection Transform

The basic idea is that each transform would expand the set of matching entities for a given original entity and
as a result would increase the complexity of the search.

Logically everything becomes very simple once we have:
1. entities (nodes of some sort with associated data)
2. categories (any set of nodes)

The search flow just becomes:
1. Find matching entities given original input via some search criteria - this builds a multimap from { original entity -> list [ entities ] }
2. Build category sets for each original entity { original entity -> { category -> list [entities] }}
3. Look for set intersections of minimum cardinality in the categories belonging to original entities

The tricky bit is that we want to derive some categories too (e.g. implicit category for every entity of all entities contained in its page?).
Forward mapping is easy to extract into a large file via regex search for references (i.e. [[something]] or [[something|visible description]])
Inverting the mapping may not be so easy unless we can keep it in memory lol. We can build a partial mapping for each article aka keep
inverted mapping in memory as long as the "parent article"'s sha256 starts with 'a'.

Then do we make the index recursive and complete? - let's check how much data that takes

Article Titles need to be unambiguous under any normalization that we do (e.g. lowercase, whitespace stripping, etc) - aka no collisions

Should we apply broad categories to the recursive completion? (these 5 things are rocks, these are all animals, etc) - seems nice to have but
requires external category lists that are super broad. Actually associations that are that broad probably could be determined by a human more
effectively.

EXAMPLES:
list of word association puzzles in mysteryhunt history: https://devjoe.appspot.com/huntindex/keyword/wordassociation

carmen san diego (phonetic, partial match, synonym) - http://web.mit.edu/puzzle/www/1999/puzzles/4Detective/Warrants4/w4.4/w4.4.html

All words are part of phrases that contain one of TOP,LEFT: http://web.mit.edu/puzzle/www/00/set2/7/Puzzle.html - looks like we actually want the nutrimatic index (or some kind of "phrase" database - exactly the nutrimatic index)

Excellent example of pure association: http://web.mit.edu/puzzle/www/2012/puzzles/phantom_of_the_operator/set_theory/ - lots of queries though so needs to be quick with initial results at least
- consider adding associative relationships if those are pre-known? the goal being to basically solve set_theory automatically

More notes:

We're extracting (or attempting to with regex) all of the ordered/unordered list items and table entries for each article. This produces a much noisier data set.
Any depth-based search should use clean data as soon as possible to limit the branch factor.

(random aside) For synonym lookup: Wordnet: https://wordnet.princeton.edu/download/current-version

Wikipedia has lists with sublist breakdowns: (u'list of films: d', [u'drugstore girl', u'domestic disturbance']) -> these would be captured by depth >1 searches but it's a bit annoying. Can we do anything?

Algorithm:

Given a set of words:
- Choose initial transformations
- Choose initial acceptance criteria (default: contains word, alternatives could be hamming distance, anagram, phonetic distance, etc)
- Choose depth and recursive criteria
- Choose exit criteria (association found for all elements, for half the elements, etc)

Code:
1. Perform initial transformation on [original_set] to derive { original_word: [working_set] }
2. For each working set, do a scan through initial data sets 
3. Do a scan through wikipedia article reference association
4. Do iteratively until desired depth is reached OR exit criteria reached OR processing limit reached

Pre-processing:
- takes wikimedia raw dump as only argument, generates finished merged index with 2G memory and only takes up ~100G disk space total (wikipedia uncompressed is like ~70G)
