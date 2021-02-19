import requests
from requests.status_codes import codes
import json
import yaml
import os
import os.path as p

WATERWHEEL_HOST = 'http://localhost:8080'

print('create project')
resp = requests.post(WATERWHEEL_HOST + '/api/projects', json=json.load(open('project.json')))
assert resp.status_code == codes.created

for file in os.listdir('.'):
    if p.splitext(file)[1] == '.yml':
        print(f'deploying {file}')
        job = yaml.safe_load(open(file))
        resp = requests.put(WATERWHEEL_HOST + '/api/jobs', json=job)
        if resp.status_code != codes.created:
            print(resp.text)
