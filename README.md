The inspiration here is to create a tool that can handle the following puzzle types:

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
- 

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

%u0e42%u0e23%u0e07%u0e40%u0e23%u0e35%u0e22%u0e19%u0e1a%u0e49%u0e32%u0e19%u0e1e%u0e23%u0e49%u0e32%u0e27 %u0e15%u0e33%u0e1a%u0e25%u0e22%u0e21 %u0e2d%u0e33%u0e40%u0e20%u0e2d%u0e17%u0e48%u0e32%u0e27%u0e31%u0e07%u0e1c%u0e32 %u0e08%u0e31%u0e07%u0e2b%u0e27%u0e31%u0e14%u0e19%u0e48%u0e32%u0e19
