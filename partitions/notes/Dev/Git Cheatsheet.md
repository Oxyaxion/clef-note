---
area: dev
lastModified: 2026-06-09
status: active
tags:
  - git
  - cli
  - cheatsheet
title: Git Cheatsheet
---

# Git Cheatsheet

## Daily workflow

```bash
git status
git add -p                     # interactive staging (review each hunk)
git commit -m "msg"
git push
```

## Branching

```bash
git switch -c feature/my-feature
git switch main && git merge --no-ff feature/my-feature
git branch -d feature/my-feature
```

## History

```bash
git log --oneline --graph --all
git show <commit>
git blame <file>
```

## Fixing mistakes

```bash
git commit --amend --no-edit   # fold staged changes into the last commit
git revert HEAD                # safe undo — creates a new commit
git restore --staged <file>    # unstage without discarding changes
```

See also: [[Linux Commands]]