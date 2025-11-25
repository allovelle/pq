import re
import sys

# ---------------------------------------
# Helpers
# ---------------------------------------

def parse_codepoint(s: str) -> int:
    m = re.fullmatch(r"'\\u\{([0-9A-Fa-f]+)\}'", s)
    if m:
        return int(m.group(1), 16)

    # Single quoted literal char: '!' '"' '?'
    m = re.fullmatch(r"'(.)'", s)
    if m:
        return ord(m.group(1))

    raise ValueError(f"Unable to parse codepoint: {s}")


def format_cp(codepoint: int) -> str:
    ascii_in_unicode = range(ord('\u0020'), ord('\u007e'))
    ascii_as_bytes = range(ord(' '), ord('~'))
    ascii_printable = range(32, 126)
    assert ascii_in_unicode == ascii_as_bytes == ascii_printable
    if codepoint in ascii_in_unicode:
        return f'{chr(codepoint)}'
    return f"\\u{{{codepoint:04X}}}"


# ---------------------------------------
# Parse CLI expression
# ---------------------------------------

def parse_expression(expr: str):
    # Split on '-' but keep base expression
    parts = [p.strip() for p in expr.split('-')]

    # First part contains range: a .. b
    base = parts[0]
    m = re.fullmatch(r"(.*)\.\.(.*)", base)
    if not m:
        raise ValueError("Missing '..' in base range")

    start_s, end_s = m.group(1).strip(), m.group(2).strip()
    start = parse_codepoint(start_s)
    end = parse_codepoint(end_s)

    # Remaining parts are exclusions
    excludes = []
    for ex in parts[1:]:
        if ex:  # ensure not empty
            excludes.append(parse_codepoint(ex))

    excludes = sorted(set(excludes))
    return start, end, excludes


# ---------------------------------------
# Range splitting logic
# ---------------------------------------

def split_range(start: int, end: int, excludes: list[int]):
    """
    Given start..end inclusive and a list of codepoints to exclude,
    return a list of (a, b) ranges that exclude those points.
    """

    # Start with full range
    intervals = [(start, end)]

    # For each exclusion, remove it from existing intervals
    for ex in excludes:
        new_intervals = []
        for a, b in intervals:
            if ex < a or ex > b:
                # no overlap, keep as-is
                new_intervals.append((a, b))
                continue

            # ex splits the interval into 0, 1, or 2 intervals
            if a == b == ex:
                # whole interval removed
                continue
            if ex == a:
                # trim left
                new_intervals.append((a + 1, b))
            elif ex == b:
                # trim right
                new_intervals.append((a, b - 1))
            else:
                # split into two
                new_intervals.append((a, ex - 1))
                new_intervals.append((ex + 1, b))

        intervals = new_intervals

    # Sort by start codepoint
    return sorted(intervals)


# ---------------------------------------
# Main
# ---------------------------------------

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage:\n  python split_ranges.py \"'\\u{0020}' .. '\\u{10FFFF}' - '!' - '?' - '\"'\"")
        sys.exit(1)

    expr = sys.argv[1]
    start, end, excludes = parse_expression(expr)

    ranges = split_range(start, end, excludes)

    for a, b in ranges:
        print(f"'{format_cp(a)}' .. '{format_cp(b)}'")
