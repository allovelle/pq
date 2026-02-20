Json lines spawns more individual trees

Fanout into multiple docs
      /-->
--> [*] --->
      \-->

Faninto single document
--\
---> [-] -->
--/


Data pipeline structure

1. A single JSON document `pq json.json` contains JSON, yet can be queried with
    a fanout query, producing JSON LINES
2. A JSONL document starts out immediately as multiple JSON documents
3. JSONL can be fanned into an array, producing a single JSON document
4. Every query produces a new root tree


Concurrency structure

Jobs take 1 query each with 1 root doc each


queryies are already performed on each root node in a fanout.
this works just as if it's multiple docs on the cli or stdin pipes
fanning into an array pipes them all into a single root
