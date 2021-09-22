import requests
from requests.status_codes import codes
import json
import yaml
import pathlib

WATERWHEEL_HOST = 'http://localhost:8080'

session = requests.session()

print('create project')
project = json.load(open('project.json'))
resp = session.post(WATERWHEEL_HOST + '/api/projects', json=project)
print(resp.status_code, resp.text)
assert resp.status_code == codes.created

# obviously you wouldn't deploy secrets like this in a real environment
print('create secrets')
resp = session.put(WATERWHEEL_HOST + '/api/stash/test-key', data='test global stash')
assert resp.status_code == codes.created

resp = requests.put(WATERWHEEL_HOST + f'/api/projects/{project["uuid"]}/stash/test-key',
                     data='test project stash')
assert resp.status_code == codes.created


for file in pathlib.Path('./jobs').iterdir():
    ext = file.suffix
    if ext == '.yml':
        print(f'deploying {file}')
        job = yaml.safe_load(file.open())
    elif ext == '.json':
        print(f'deploying {file}')
        job = json.load(file.open())
    else:
        continue

    resp = session.put(WATERWHEEL_HOST + '/api/jobs', json=job)
    if resp.status_code != codes.created:
        print(resp.text)
