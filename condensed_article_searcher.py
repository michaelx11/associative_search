import json 
import re
import hashlib
from collections import defaultdict

article_to_parent = {}

markup_link_pattern = re.compile(r'\[\[([^|\]]{1,128})(\|[^\]]{1,128})?\]\]', re.I)



def sha256digest(string):
    sha256 = hashlib.sha256()
    sha256.update(string)
    return sha256.hexdigest()


title_dict = defaultdict(lambda: set())
longest_title = ''
longest_record = None
def check_record(record):
    global title_dict
    global longest_title
    global longest_record
#    title_sha = sha256digest(record['title'])
    # Parse all [[subreferences]] contained in article
    if len(record['title']) > len(longest_title):
        longest_title = record['title']
        longest_record = record
    if record['page'].startswith('"{{Not English'):
        # Skip it
        return
    # Hash title
    title_hash = sha256digest(record['title'])
    if not title_hash.startswith('a'):
        return
    for match in re.finditer(markup_link_pattern, record['page']):
        title_dict[match.group(1)].add(record['title'])

#    if 'fred flintstone' in record['title'].lower():
#        print(record)


fields = ['title', 'categories', 'page']
count = 0
with open('condensed.csv', 'r') as ff:
    i = 0
    record = {}
    for line in ff:
        record[fields[i]] = line.rstrip()
        i += 1
        if i == 3:
            check_record(record)
            record = {}
            i = 0
            count += 1
        if count % 100000 == 0 and i == 0:
            print('checked: {}'.format(count))
print('longest title', longest_title)
print('record:', longest_record)
