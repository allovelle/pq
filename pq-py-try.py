from decimal import Decimal
import ijson
import itertools
from collections import defaultdict


# ------------------------------------------------------------
# Row Generator (streaming)
# ------------------------------------------------------------


def stream_rows(json_stream):
    """
    Incrementally parse JSON and yield rows representing every
    object, array, key, and scalar value.
    """
    id_gen = itertools.count(1)
    stack = []  # stack of (id, kind, key/index)
    key_pending = None

    for prefix, event, value in ijson.parse(json_stream):
        if isinstance(value, Decimal):
            value = float(value)

        # Start of object
        if event == "start_map":
            row_id = next(id_gen)
            parent = stack[-1][0] if stack else None
            key, index = _extract_key_index(stack, key_pending)
            path = _make_path(stack, key_pending)
            yield {
                "id": row_id,
                "parent": parent,
                "kind": "object",
                "key": key,
                "index": index,
                "value": None,
                "path": path,
            }
            stack.append((row_id, "object", key_pending))
            key_pending = None

        # End of object
        elif event == "end_map":
            stack.pop()

        # Start of array
        elif event == "start_array":
            row_id = next(id_gen)
            parent = stack[-1][0] if stack else None
            key, index = _extract_key_index(stack, key_pending)
            path = _make_path(stack, key_pending)
            yield {
                "id": row_id,
                "parent": parent,
                "kind": "array",
                "key": key,
                "index": index,
                "value": None,
                "path": path,
            }
            stack.append((row_id, "array", key_pending))
            key_pending = None

        # End of array
        elif event == "end_array":
            stack.pop()

        # Key inside object
        elif event == "map_key":
            key_pending = value
            row_id = next(id_gen)
            parent = stack[-1][0] if stack else None
            path = _make_path(stack, key_pending)
            yield {
                "id": row_id,
                "parent": parent,
                "kind": "key",
                "key": value,
                "index": None,
                "value": None,
                "path": path,
            }

        # Scalar value
        elif event in ("string", "number", "boolean", "null"):
            row_id = next(id_gen)
            parent = stack[-1][0] if stack else None
            key, index = _extract_key_index(stack, key_pending)
            path = _make_path(stack, key_pending)
            yield {
                "id": row_id,
                "parent": parent,
                "kind": "value",
                "key": key,
                "index": index,
                "value": value,
                "path": path,
            }
            key_pending = None


def _extract_key_index(stack, key_pending):
    """Determine key/index for the next row."""
    if not stack:
        return None, None
    parent_id, parent_kind, _ = stack[-1]
    if parent_kind == "object":
        return key_pending, None
    if parent_kind == "array":
        # Count how many children this array already has
        return None, None  # index assigned later by reconstructor
    return None, None


def _make_path(stack, key_pending):
    """Construct a dotted path for debugging/transforming."""
    parts = []
    for row_id, kind, key in stack:
        if key is not None:
            parts.append(str(key))
    if key_pending is not None:
        parts.append(str(key_pending))
    return ".".join(parts)


# ------------------------------------------------------------
# Transform Pipeline
# ------------------------------------------------------------


def apply_pipeline(rows, pipeline):
    """
    pipeline = [fn1, fn2, ...]
    Each fn: row -> row or None
    """
    for row in rows:
        r = row
        for fn in pipeline:
            if r is None:
                break
            r = fn(r)
        if r is not None:
            yield r


# ------------------------------------------------------------
# Reconstruct JSON from rows
# ------------------------------------------------------------


def reconstruct(rows):
    """
    Rebuild the JSON structure from transformed rows.
    """
    by_parent = defaultdict(list)
    root_id = None
    row_map = {}

    for r in rows:
        row_map[r["id"]] = r
        if r["parent"] is None:
            root_id = r["id"]
        else:
            by_parent[r["parent"]].append(r)

    def build(node_id):
        r = row_map[node_id]
        if r["kind"] == "object":
            obj = {}
            for child in by_parent[node_id]:
                if child["kind"] == "key":
                    # key rows don't hold values; skip
                    continue
                if child["kind"] in ("object", "array"):
                    obj[child["key"]] = build(child["id"])
                elif child["kind"] == "value":
                    obj[child["key"]] = child["value"]
            return obj

        if r["kind"] == "array":
            arr = []
            for child in by_parent[node_id]:
                if child["kind"] in ("object", "array"):
                    arr.append(build(child["id"]))
                elif child["kind"] == "value":
                    arr.append(child["value"])
            return arr

        if r["kind"] == "value":
            return r["value"]

        raise ValueError("Unexpected row kind:", r["kind"])

    return build(root_id)


if __name__ == "__main__":
    import sys
    import json
    from io import StringIO

    # Example transform: uppercase all string values
    def uppercase_values(row):
        if row["kind"] == "value" and isinstance(row["value"], str):
            row = dict(row)
            row["value"] = row["value"].upper()
        return row

    if sys.stdin.isatty() and len(sys.argv) == 1:
        data = """
        {
        "user": {
            "name": "Alice",
            "roles": ["admin", "editor"]
        }
        }
        """
        json_stream = StringIO(data)
    else:
        json_stream = sys.stdin

    rows = stream_rows(json_stream)
    rows2 = apply_pipeline(rows, [uppercase_values])
    result = reconstruct(rows2)

    print(result, "!!!!!!!!!!")
    print(json.dumps(result, indent=2))
