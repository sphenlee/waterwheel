import requests
from requests.status_codes import codes
import json
import yaml
import os
import os.path as p

WATERWHEEL_HOST = 'http://localhost:8080'

print('create project')
project = json.load(open('project.json'))
resp = requests.post(WATERWHEEL_HOST + '/api/projects', json=project)
assert resp.status_code == codes.created

# obviously you wouldn't deploy secrets like this in a real environment
print('create secrets')
resp = requests.put(WATERWHEEL_HOST + '/api/stash/test-key', data='test global stash')
assert resp.status_code == codes.created

resp = requests.put(WATERWHEEL_HOST + f'/api/projects/{project["uuid"]}/stash/test-key',
                     data='test project stash')
assert resp.status_code == codes.created


for file in os.listdir('./jobs'):
    ext = p.splitext(file)[1]
    if ext == '.yml':
        print(f'deploying {file}')
        job = yaml.safe_load(open(file))
    elif ext == '.json':
        print(f'deploying {file}')
        job = json.load(open(file))
    else:
        continue

    resp = requests.put(WATERWHEEL_HOST + '/api/jobs', json=job)
    if resp.status_code != codes.created:
        print(resp.text)
