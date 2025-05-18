import signal
import time
import os

signal_names = {}
for name in dir(signal):
    if name.startswith('SIG') and not name.startswith('SIG_'):
        try:
            signal_names[getattr(signal, name)] = name
        except (ValueError, AttributeError):
            pass

def signal_handler(signum, frame):
    signal_name = signal_names.get(signum, f"Unknown signal {signum}")
    print(f"SIGNAL RECEIVED: {signum})", flush=True)

for name, signum in [(name, getattr(signal, name)) for name in dir(signal) if name.startswith('SIG') and not name.startswith('SIG_')]:
    try:
        signal.signal(signum, signal_handler)
    except (OSError, ValueError, TypeError) as e: ...

print(f"\nPID: {os.getpid()}", flush=True)

while True:
    time.sleep(1)