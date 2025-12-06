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

ranges:
    #!/usr/bin/env python3
    import sys
    # ch = chr("\u{0020}")
    # ch = chr(ord('\u0001f600'))
    # print(ch.encode("unicode_escape").decode())
    print(sys.argv[0])

    # Parse '\u{0020}' .. '\u{10FFFF}' - '\u{0020}' .. '\u{0022}' as:
    #   '\u{0020}' .. '\u{0022}'
    #   '\u{0022}' .. '\u{10FFFF}'

    # Parse '\u{0020}' .. '\u{10FFFF}' - '!' - '?' - '"' as:
    #   '\u{0020}' .. '!'
    #   '!' .. \u{0022}'
    #   \u{0023} .. '?'
    #   '?' .. '"'
    # '"' .. '\u{10FFFF}'

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
