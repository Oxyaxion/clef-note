---
area: dev
lastModified: 2026-06-09
status: active
tags:
  - linux
  - cli
  - cheatsheet
title: Linux Commands
---

# Linux Commands

## File navigation

```bash
ls -lah                                # detailed listing with hidden files
find . -name "*.log" -mtime -7         # files modified in the last 7 days
du -sh *                               # disk usage per directory
```

## Processes

```bash
ps aux | grep nginx
kill -9 <pid>
htop                                   # interactive process viewer
```

## Network

```bash
curl -I https://example.com            # response headers only
ss -tlnp                               # listening ports
```

## Text processing

```bash
grep -r "TODO" --include="*.rs" .
awk '{print $1}' access.log | sort | uniq -c | sort -rn
```

See also: [[Git Cheatsheet]]