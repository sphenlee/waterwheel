import sys
import time
import random

if __name__ == '__main__':
    if random.random() > 0.5:
        time.sleep(random.randint(0, 10))

    sys.exit(1 if random.random() > 0.9 else 0)
