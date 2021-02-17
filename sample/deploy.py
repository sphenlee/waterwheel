import requests
import uuid
import json
import yaml

WATERWHEEL_HOST = 'http://localhost:8080'

requests.post(WATERWHEEL_HOST + '/api/projects', json=json.load(open('project.json')))

job = yaml.safe_load(open('demo.yml'))
resp = requests.put(WATERWHEEL_HOST + '/api/jobs', json=job)

print(f'{resp.status_code}: {resp.text}')
