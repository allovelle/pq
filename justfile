stream:
    #!/usr/bin/env python3
    import time, random, sys
    buffer = open('json1.json').read()
    offset, length = 0, random.randint(1, 8)
    while buffer[offset:offset + length]:
        try:
            # time.sleep(random.random() * 0.5)
            sliced = buffer[offset:offset + length]
            for ch in sliced:
                print(ch, end='')
                time.sleep(0.01)
                sys.stdout.flush()
        except KeyboardInterrupt:
            break
        except BrokenPipeError:
            print('\n', 'Consumer closed pipe unexpectedly')
            break
        except Exception as e:
            print('error', e, '\n')
        offset += length
        length = random.randint(1, 8)

globe:
    #!/usr/bin/env python3
    import itertools, time, sys
    for c in itertools.cycle("🌍🌎🌏"):
        sys.stdout.write("\r"+c)
        sys.stdout.flush()
        try:
            time.sleep(0.2)
        except KeyboardInterrupt:
            break

stream_ou:
    #!/usr/bin/env python3
    import os, time

    while True:
        os.write(1, b"Hi")  # raw fd write
        time.sleep(0.01)    # 10 ms

stream_in:
    #!/usr/bin/env python3
    import sys, time

    while True:
        string = sys.stdin.read(1)
        print(string, end='')
        if string.endswith('\0'):
            break
