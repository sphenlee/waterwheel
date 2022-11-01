import os
import sys
import time
import random

FAIL_PERCENTAGE = float(os.environ.get('FAIL_PERCENTAGE', '5'))
SUCCESS_RATE = 1.0 - FAIL_PERCENTAGE / 100.0

if __name__ == '__main__':
    print('starting main.py')

    if True or random.random() > 0.5:
        s = random.randint(30, 60)
        print(f'sleeping for {s}s')
        for i in range(s):
            time.sleep(1)
            print('zzz...')

    print('finished - exiting')

    sys.exit(1 if random.random() > SUCCESS_RATE else 0)
