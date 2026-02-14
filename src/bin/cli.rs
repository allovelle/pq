/*
pq: query json from the cli
examples:
$ pq
    print help message
$ stdout | pq
    colorize json only
! $ pq .
    invalid
! $ stdout | pq .
    live query & colorize
$ stdout | pq -q
    live query & colorize
$ stdout | pq --query
    live query & colorize
$ stdout | pq 'query'
    query & colorize
$ pq 'query'
    waits for stdin, err if not pipe
$ pq -f json.json 'query'

$ stdout | pq -f json.json 'query'
    what does this do? should it search multiple files? no. json doesn't do that


Ambiguous cases:

This becomes json lines:
pq json1 json2 json3
    pq json.jsonl
*/

// ! This is a valid call, and is most likely the biggest value add of pq:
// ! `bat json.json | pq .`
// ! ^^^ This enters the TUI for live-querying the resulting JSON stream.

fn main() {}
