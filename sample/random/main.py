import sys
import time
import random

if __name__ == '__main__':
    if random.random() > 0.5:
        s = random.randint(0, 10)
        print(f'sleeping for {s}s')
        time.sleep(s)

    sys.exit(1 if random.random() > 0.95 else 0)
