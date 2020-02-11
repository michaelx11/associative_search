import json 
import re
import hashlib
from collections import defaultdict

article_to_parent = {}

markup_link_pattern = re.compile(r'\[\[([^|\]]{1,256})(\|[^\]]{1,256})?\]\]', re.I)



def sha256digest(string):
    sha256 = hashlib.sha256()
    sha256.update(string.encode('utf-8'))
    return sha256.hexdigest()


title_dict = defaultdict(lambda: set())
longest_title = ''
longest_record = None
def check_record(record, cc):
    global title_dict
    global longest_title
    global longest_record
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
    for match in re.finditer(markup_link_pattern, page):
        title_dict[match.group(1).strip().lower()].add(title)

#    if 'fred flintstone' in record['title'].lower():
#        print(record)


fields = ['title', 'categories', 'page']
count = 0
for cc in '0123456789abcdef':
    title_dict = defaultdict(lambda: set())
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
    with open('indexes/{}.txt'.format(cc), 'w') as index_file:
        for t_title in sorted(title_dict):
            index_file.write('{}\n'.format(json.dumps({'t': t_title, 'as': list(title_dict[t_title])})))

