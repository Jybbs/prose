"""
Long single-line dict with multiple levels of nested dicts. The full
pipeline exercises every dict-handling feature in concert:
collection_layout expands the outer dict and recurses into any value
whose inline form overflows, alphabetize sorts and partitions
single-line entries before multi-line entries with a blank-line
divider at every level, align_colons aligns colons within each
line-adjacent group so the divider closes the active alignment run,
and strip_trailing_commas removes the dangling commas the recursive
expansion leaves behind. Every level carries multiple siblings of each
shape, so the sort, partition, and per-group alignment are visible at
every depth. The full pipeline running twice on this input produces
no further change.
"""

DEPLOY = {"region": "us-east", "version": 12, "stage": "prod", "owner": "core", "services": {"api": {"timeout": 30, "retries": 3, "port": 8080, "endpoints": {"health": "/health", "metrics": "/metrics", "readiness": "/ready", "version": "/v"}, "headers": {"trace_id": "x-trace", "user_agent": "deploy/1", "request_id": "x-req"}}, "worker": {"concurrency": 8, "queue": "tasks"}, "gateway": {"timeout": 5, "rate_limit": 1000, "upstream": {"primary": "api.internal", "fallback": "api.backup"}}}, "telemetry": {"sink": "datadog", "interval": 30, "logs": {"level": "info", "format": "json", "destinations": {"file": "/var/log/app", "stdout": True, "syslog": False, "remote_sink": "logserver"}}}}
