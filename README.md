# pq

pique-rs

> See [QUERY.txt](QUERY.txt) for an in-depth look at the querying capabilities
> of pique

# TODO

I'm building a pipeline where I need to read N filenames from the CLI input as
    well as read from stdin for a total of N + 1 file streams. Please treat each
    filename the same as stdio. I don't want to tell where BufReaders came from.
I need each file to be processed one at a time, so it's ok to wait while the
    pipeline finishes before starting the next one. There may be a way to
    schedule the launch of all of them and then they block.
I need one file stream to run in it's own async fn and use non-buffered IO for
    a specific reason. I need incremental bytes coming across the line.
I need a pipelined second async fn to read bytes incrementally off of the file
    buf reader. These bytes need to be convered to `char` and if there are
    fragmented bytes at the end (the last char would be corrupted), please store
    them in a [u8; 4] array to be processed first before next chunk.
I need this to be zero copy where possible and use as few heap allocations as
    is possible.
Please think long and hard, I need this to work.
