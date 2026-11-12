---
name: epic
description: Turn the conversation into a parent issue (or rewrite issue #N) as a spec, then split it into sub-issues.
disable-model-invocation: true
argument-hint: "[#issue]"
---

# Epic

Two passes over one **parent issue**: write it as a spec, then split it into sub-issues. Both passes run the user's
own skills, `to-spec`, then `to-tickets`.

## 1. Spec the parent

Run `to-spec`, with these overrides:

- **No argument**: the source is the conversation. Publish the spec as a new issue: that is the parent.
- **`#N` passed**: the source is the conversation plus issue #N. The spec absorbs
  every fact, decision and open question #N holds. Show the spec, then on the user's yes replace the body,
  keeping the title unless the spec makes it wrong. #N is the parent.
- **Label**: the parent gets no `ready-for-agent`; its sub-issues carry it, so the parent never reads as a task to
  grab.

Done when the parent's body on GitHub is the spec.

## 2. Split into sub-issues

Run `to-tickets` with the parent as its source, including its quiz step: the user approves the breakdown before
anything is published.

Link each ticket to the parent as a GitHub sub-issue and its blockers as native dependencies, per
issue-tracker.md › Wayfinding operations.

Done when every approved ticket is an open sub-issue of the parent, with its blockers set.

## Report

The parent's URL, then one line per sub-issue: number, title, blocked by.
