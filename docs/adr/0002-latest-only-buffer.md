# Latest-only Clip buffer with short TTL

Missed Clips while a Device is unreachable should not become a durable clipboard history (privacy and product scope). The relay (or sender path) keeps only the newest Clip per Sync Group for delivery, with a ~15 minute TTL; a newer Clip supersedes any buffered one. Live-only drop was rejected as too brittle for phone sleep; a full queue was rejected as history-by-another-name.
