import requests
import os


def main():
    print('in main.py')

    host = os.environ['WATERWHEEL_SERVER_ADDR']
    jwt = os.environ['WATERWHEEL_JWT']

    headers = {
        'Authorization': 'Bearer ' + jwt
    }
    resp = requests.get(host + 'api/stash/test-key', headers=headers)
    resp.raise_for_status()

    secret = resp.text
    print(f'secret is "{secret}"')


if __name__ == '__main__':
    main()
