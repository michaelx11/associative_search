import json

from collections import defaultdict

#search_set = ['manning', 'brady', 'dalton']
#search_set = ['sun tzu', 'hannibal', 'rommel', 'patton']
#search_set = ['sun tzu', 'hannibal', 'rommel', 'patton']
search_set = ['tom bowler','red devil','agate','cats eye']
#search_set = ['drugstore', 'urban']
#search_set = ['okinawa', 'tinian']
association_dict = defaultdict(lambda: {})

cc = 0
with open('big_table_index.txt', 'r') as ff:
    for line in ff:
        title, associated_articles = json.loads(line.strip())
        for item in search_set:
            # TODO: fix this search as there are a ton of false positives
            if item in title:
                for thingy in associated_articles:
                    association_dict[item][thingy] = title
        cc += 1
        if cc % 1000000 == 0:
            print('processed: {}'.format(cc))
                    
            # Look for thingies in common
            association_count_dict = defaultdict(int)
            
            for item in search_set:
                for assoc in association_dict[item]:
                    association_count_dict[assoc] += 1
            
            for k, count in association_count_dict.iteritems():
                if count >= len(search_set):
                    print(k, [association_dict[item].get(k) for item in search_set])
