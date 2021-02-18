import requests
import os

WW_SERVER_ADDR = os.environ['WATERWHEEL_SERVER_ADDR']
WW_JWT = os.environ['WATERWHEEL_JWT']
WW_PROJECT = os.environ['WATERWHEEL_PROJECT_ID']
WW_JOB = os.environ['WATERWHEEL_JOB_ID']
WW_TASK_NAME = os.environ['WATERWHEEL_TASK_NAME']
WW_TRIGGER = os.environ['WATERWHEEL_TRIGGER_DATETIME']


def put_secret(key, value):
    headers = {
        'Authorization': 'Bearer ' + WW_JWT
    }
    resp = requests.put(f'{WW_SERVER_ADDR}api/jobs/{WW_JOB}/stash/{WW_TRIGGER}/{key}', headers=headers, data=value)
    resp.raise_for_status()


def get_secret(path):
    headers = {
        'Authorization': 'Bearer ' + WW_JWT
    }
    resp = requests.get(f'{WW_SERVER_ADDR}api/{path}', headers=headers)
    resp.raise_for_status()
    return resp.text


def step0():
    print('step 0!')

    put_secret('current-time', WW_TRIGGER)

def step1():
    print('step 1!')

    secret = get_secret('stash/test-key')
    print(f'global secret is "{secret}"')

    secret = get_secret(f'projects/{WW_PROJECT}/stash/test-key')
    print(f'project secret is "{secret}"')

    secret = get_secret(f'jobs/{WW_JOB}/stash/{WW_TRIGGER}/current-time')
    print(f'job secret is "{secret}"')
    assert secret == WW_TRIGGER




if __name__ == '__main__':
    globals()[WW_TASK_NAME]()
