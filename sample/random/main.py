import os
import sys
import time
import random

FAIL_PERCENTAGE = float(os.environ.get('FAIL_PERCENTAGE', '5'))
SUCCESS_RATE = 1.0 - FAIL_PERCENTAGE / 100.0

if __name__ == '__main__':
    if random.random() > 0.5:
        s = random.randint(0, 10)
        print(f'sleeping for {s}s')
        time.sleep(s)

    sys.exit(1 if random.random() > SUCCESS_RATE else 0)
