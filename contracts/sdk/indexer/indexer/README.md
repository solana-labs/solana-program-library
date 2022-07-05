# Quick and Dirty: Typescript Indexer

Query to check backfill script by removing all the most recent
merkle tree nodes
```sql
DELETE FROM merkle
WHERE seq in 
(SELECT seq FROM 
    (SELECT node_idx, MAX(seq) AS seq 
    FROM merkle 
    WHERE level = 0 group by node_idx
));
```
