stream:
    #!/usr/bin/env python3
    import time, random, sys
    buffer = open('json3.json').read()
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
