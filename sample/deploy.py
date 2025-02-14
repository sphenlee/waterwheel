import requests
from requests.status_codes import codes
import json
import yaml
import pathlib
import os
import urllib3

# minikube uses a self-signed certificate; ignore the insecure warnings
urllib3.disable_warnings()

WATERWHEEL_HOST = os.environ.get('WATERWHEEL_ADDR', 'http://localhost:8080/').rstrip('/')

session = requests.session()
session.verify = None

resp = session.get(WATERWHEEL_HOST + '/api/status')
if resp.status_code == codes.unauthorized:
    print('login')
    resp = session.post(WATERWHEEL_HOST + '/login', data={'username': 'fry', 'password': 'fry'})
    resp.raise_for_status()
else:
    resp.raise_for_status()

print('create project')
project = json.load(open('project.json'))
resp = session.post(WATERWHEEL_HOST + '/api/projects', json=project)
print(resp.status_code, resp.text)
assert resp.status_code == codes.created

# obviously you wouldn't deploy secrets like this in a real environment
print('create secrets')
resp = session.put(WATERWHEEL_HOST + '/api/stash/test-key', data='test global stash')
assert resp.status_code == codes.created

resp = session.put(WATERWHEEL_HOST + f'/api/projects/{project["uuid"]}/stash/test-key',
                     data='test project stash')
assert resp.status_code == codes.created


for file in pathlib.Path('./jobs').iterdir():
    ext = file.suffix
    if ext == '.yml':
        content_type = "application/x-yaml"
    elif ext == '.json':
        content_type = "application/json"
    else:
        continue

    print(f'deploying {file}')
    resp = session.put(
        WATERWHEEL_HOST + '/api/jobs',
        data=open(file).read(),
        headers={'Content-Type': content_type},
    )
    if resp.status_code != codes.created:
        print(resp.text)
