import os
import json
from collections import deque

# Read from indexes directory (0-9a-f.txt)
# Write to a merged index.txt

hex_alpha = '0123456789abcdef'
index_files = []
# Global index format is JSON list per line, ["[article]", list[containing articles]]
global_index = deque()

# NOTE: flag to control whether we generate "norm" indexes or full indexes
# where a norm index just uses list items and table items which are more reliable and structured
IS_NORM = False

index_folder = 'norm_table_indexes' if IS_NORM else 'table_indexes'

if not os.path.isdir(index_folder):
    raise Exception('Missing index folder: {}'.format(index_folder))

# open all index files
for c in hex_alpha:
    index_files.append(open(os.path.join(index_folder, '{}.txt'.format(c)), 'r'))

current_set = [None] * len(index_files)

def parse_line(line):
    obj = json.loads(line)
    return (obj['t'], obj['as'])

for i, ff in enumerate(index_files):
    current_set[i] = parse_line(ff.next())

filename = 'norm_table_index.txt' if IS_NORM else 'big_table_index.txt'
final_index_file = open(filename, 'w')
current_item = None
counter = 0
while True:
    # find the minimum element
    min_index = 0
    min_title = None
    for i, record in enumerate(current_set):
        if record is None:
            continue
        if min_title == None or record[0] < min_title:
            min_title = record[0]
            min_index = i
    if min_title is None:
        # we're done!
        break
    # add that element to the global index if it's already the last element
    if current_item == None:
        current_item = current_set[min_index]
    elif current_item[0] == current_set[min_index][0]:
        current_item[1].extend(current_set[min_index][1])
        current_item[1].sort()
    elif current_item[0] < current_set[min_index][0]:
        # Write to a file
        final_index_file.write('{}\n'.format(json.dumps(current_item)))
        counter += 1
        if counter % 100000 == 0:
            print('Wrote: {} with record: {}'.format(counter, current_item))
        current_item = current_set[min_index]

    # Increment the lowest file
    try:
        current_set[min_index] = parse_line(index_files[min_index].next())
    except StopIteration:
        current_set[min_index] = None

final_index_file.close()
for ff in index_files:
    ff.close()
