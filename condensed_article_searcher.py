import os
import json 
import re
import hashlib
from collections import defaultdict

article_to_parent = {}

markup_link_pattern = re.compile(r'\[\[([^|\]]{1,256})(\|[^\]]{1,256})?\]\]', re.I)
# can be 1., - + * for leading items
# also can have leading spaces
# but how much of the text do we take? all of it?
# take until newline seems like a reasonable strategy
list_item_pattern = re.compile(r'^\s*([*-+]+|\d+\.)\s*([^\n]+)$', re.I | re.M)
# Table markup parser, goal is to extract only the table items
# Do the easy thing and just look for |-
# can either be a single line (|| delimited) or a line for each item (| start)
table_row_pattern = re.compile(r'^\|-\n((\|\s+[^\n]+)+)$', re.I | re.M)

style_eliminator_pattern = re.compile(r'^\|\s+style=[^|]+\|(.+)$', re.I)

# NOTE: flag to control whether we generate "norm" indexes or full indexes
# where a norm index just uses list items and table items which are more reliable and structured
IS_NORM = False

def sha256digest(string):
    sha256 = hashlib.sha256()
    sha256.update(string.encode('utf-8'))
    return sha256.hexdigest()


title_dict = defaultdict(lambda: set())
item_dict = defaultdict(lambda: set())
longest_title = ''
longest_record = None
total_list_items = 0
def check_record(record, cc):
    global title_dict
    global item_dict
    global total_list_items
#    title_sha = sha256digest(record['title'])
    # Parse all [[subreferences]] contained in article
    if record['page'].startswith('"{{Not English'):
        # Skip it
        return
    title = json.loads(record['title']).strip().lower()
    page = json.loads(record['page'])
    # Hash title
    title_hash = sha256digest(title)
    if not title_hash.startswith(cc):
        return
#    print('title: {}'.format(title))
    if not IS_NORM:
        for match in re.finditer(markup_link_pattern, page):
            title_dict[match.group(1).strip().lower()].add(title)
    for match in re.finditer(list_item_pattern, page):
        # Check to see if the match would also contain markup link
        if markup_link_pattern.match(match.group(2)):
            continue
#        print('li: {}'.format(match.group(2).encode('utf-8')))
        item_dict[match.group(2)].add(title)
        total_list_items += 1
    for match in re.finditer(table_row_pattern, page):
        raw_row = match.group(1)
        # could start with '| style="'
        style_clause_match = style_eliminator_pattern.match(raw_row)
        if style_clause_match is not None:
#            import pdb; pdb.set_trace()
            raw_row = style_clause_match.group(1)
        # the raw_row is gonna start with (| and whitespace)
        trimmed_row = raw_row.lstrip('|')
        # split by ||
        components = trimmed_row.split('||')
        # ignore special stuff? |-
        # | Total || {{bartable|100||2||background:grey}}
        normalized_components = []
        for comp in components:
            norm = comp.strip().lower()
            if len(norm) == 0:
                continue
            item_dict[norm].add(title)
            normalized_components.append(norm)
#        print(normalized_components)


fields = ['title', 'categories', 'page']
count = 0
for cc in '0123456789abcdef':
    title_dict = defaultdict(lambda: set())
    item_dict = defaultdict(lambda: set())
    print("processing: {}".format(cc))
    with open('condensed.csv', 'r') as ff:
        i = 0
        record = {}
        for line in ff:
            record[fields[i]] = line.rstrip()
            i += 1
            if i == 3:
                check_record(record, cc)
                record = {}
                i = 0
                count += 1
            if count % 1000000 == 0 and i == 0:
                print('checked: {}'.format(count))
                print('total list items: {}'.format(total_list_items))
    index_folder = 'norm_table_indexes' if IS_NORM else 'table_indexes'
    if os.path.isdir(index_folder):
        os.mkdir(index_folder)
    with open('{}/{}.txt'.format(index_folder, cc), 'w') as index_file:
        for t_title in sorted(item_dict):
            index_file.write('{}\n'.format(json.dumps({'t': t_title, 'as': list(item_dict[t_title])})))

