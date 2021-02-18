import requests
import json
import yaml
import os
import os.path as p

WATERWHEEL_HOST = 'http://localhost:8080'

requests.post(WATERWHEEL_HOST + '/api/projects', json=json.load(open('project.json')))

for file in os.listdir('.'):
    if p.splitext(file)[1] == '.yml':
        print(f'deploying {file}')
        job = yaml.safe_load(open(file))
        resp = requests.put(WATERWHEEL_HOST + '/api/jobs', json=job)

        print(f'{resp.status_code}: {resp.text}')
