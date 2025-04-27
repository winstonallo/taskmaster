import time
import sys

i = 0

while True:
    print(f"--stdout--", flush=True)
    print(f"!!stderr!!", flush=True, file=sys.stderr)
    time.sleep(1)