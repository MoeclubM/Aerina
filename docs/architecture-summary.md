# Aerina Architecture Summary

This is the short working summary of [project-plan-v1.1.md](./project-plan-v1.1.md).

## One tree, many surfaces

```text
Conversation Tree
├─ Branch
│  └─ Round
│     └─ Candidate[]
│        └─ ContentBlock[]  # text / image / future agent summary
```

Surfaces:

- Chat
- Image generation
- SBS compare
- Arena
- Stats / ranking
- Future Agent surface

Do not create separate message systems for chat, image, arena, or agent.

## Generation is capability-driven

```text
Generation Engine
├─ text stream
├─ image generation
├─ vision input
└─ future agent run ref
```

Providers expose capabilities:

- text
- vision
- image_generation
- streaming
- reasoning
- tool_calling
- agent_runtime

Unsupported capability must fail explicitly. No silent fallback.

## Arena is not only text

Arena = `Kind + Candidate Pool + Capability Filter + Scoring Profile`

First-class kinds:

- `text`
- `image_gen`
- `vision`

Reserved:

- `mixed`
- `code`
- `agent_task`

Only anonymous random arenas update ranking.
SBS is public comparison and never updates ranking.
Text wins and image wins stay on separate boards.

## Stats are first-class

```text
AnalyticsEvent -> aggregates -> dashboard / export
RankingEvent   -> Elo cache  -> leaderboard
```

Track at least:

- requests
- tokens
- cost
- TTFT / total latency
- success / fail / cancel
- image counts
- arena votes
- model / provider breakdown

## Future local agents

Agent means local task runtime, similar to:

- Codex
- Claude Code
- OpenCode
- Pi
- Hermes Agent

Not:

- "chat model + a few MCP tools" as the whole agent model

Extension shape:

```text
Agent Orchestrator
└─ Agent Runtime Adapter
   ├─ start_run / cancel_run / events
   ├─ approval hooks
   ├─ workspace isolation
   └─ trajectory export
```

Mapping into the existing tree:

```text
Candidate
├─ summary + artifacts
└─ agent_run_id -> AgentRun / AgentStep[]
```

MCP is an optional tool source inside a runtime, not the definition of Agent.

## Hard rules

1. Conversation is a tree.
2. Round supports N candidates.
3. Chat / SBS / Arena share generation.
4. Text and image share generation + snapshots.
5. Arena identity stays in Rust core until vote reveal.
6. Rankings and stats are derived, rebuildable.
7. SQLite is local source of truth.
8. Sync and Agent are pluggable modules.
9. Frontend never holds raw API keys.
10. No silent compatibility/fallback paths in core logic.

## Milestone order

1. Domain + tree + events
2. Chat
3. Image / vision
4. SBS
5. Diversified arena
6. Ranking + stats
7. Stability
8. Optional sync
9. Local agent runtime
10. Agent arena
