

# Sentinel — Usage & Command-Line Documentation

This document explains **how to run Sentinel**, **what commands are available**, and **how to correctly interpret example commands**.

Sentinel is a **transparent, fail-open observability sidecar** for MCP (Model Context Protocol).  
It wraps an MCP server, observes agent ↔ tool traffic, and produces a canonical event stream, optional audit logs, and a real-time dashboard.

---

## Core Mental Model

**Sentinel is not a client.**  
**Sentinel is not a server.**  
**Sentinel is a wrapper.**

If you normally start an MCP server like this:

```bash
<your-mcp-server-command>
```

Run it with Sentinel like this:
```bash
sentinel run -- <your-mcp-server-command>
```

This rule applies universally.

----------

## What Sentinel Wraps

Sentinel wraps MCP servers, not agents.

-   Agents (Claude Desktop, frameworks, SDKs) already communicate with MCP servers
    
-   Sentinel sits between the agent and the server
    
-   Sentinel observes traffic without modifying execution
    
-   You do not manually run an agent when using Sentinel.
    

----------

## Command Overview

```bash
sentinel run
sentinel install
sentinel keygen
sentinel recipient-keygen
sentinel verify
```

Use `sentinel <command> --help` for detailed flags.

----------

## `sentinel run`

Runs Sentinel as a transparent stdio proxy.

### Basic Usage

```bash
sentinel run -- <mcp-server-command>
```

Everything after `--` is treated as the MCP server command.


### Example (Reference MCP Server)

```bash
sentinel run -- npx -y @modelcontextprotocol/server-filesystem
```

> **Important:**  
> This is an example, not a requirement.  
> `@modelcontextprotocol/server-filesystem` is a lightweight reference MCP server  
> It requires no setup and works cross-platform  
> Any MCP server in any language can be used instead

----------

## What the `--` Means

The double dash (`--`) tells Sentinel:

-   Stop parsing Sentinel flags.
    
-   Everything after this is the child process Sentinel should wrap.
    

This is standard CLI behavior.

----------

## Data Flow

```
Agent
  ↓
Sentinel (observes traffic)
  ↓
MCP Server
```

Sentinel:

-   Forwards stdin/stdout unchanged
    
-   Observes JSON-RPC messages
    
-   Derives ordering from observation
    
-   Never blocks execution
    

----------

## WebSocket Dashboard

Default address:

```
http://127.0.0.1:3000
```

### Enable Authentication (Recommended)

```bash
sentinel run \
  --ws-bind "127.0.0.1:3000" \
  --ws-token "secret123" \
  -- <mcp-server-command>
```

If the UI disconnects or crashes:

-   Sentinel continues proxying
    
-   Agent execution is unaffected
    
---

## Audit Logging

Sentinel can write append-only audit logs of observed agent behavior.

### Enable Signed Audit Logs

```bash
sentinel run \
  --audit-log audit.jsonl \
  --signing-key-b64-path ./keys/signing_key.b64 \
  -- <mcp-server-command>
```

**Properties:**

-   NDJSON format
    
-   Hash-chained
    
-   Periodically signed
    
-   Tamper-evident
    

----------

### Enable Encrypted Audit Logs (Optional)

```bash
sentinel run \
  --audit-log audit.jsonl \
  --signing-key-b64-path ./keys/signing_key.b64 \
  --encrypt-recipient-pubkey-b64-path ./keys/recipient_pub.b64 \
  -- <mcp-server-command>
```

> Encryption applies only to telemetry, never to live execution.

----------

## Key Generation

Sentinel uses two separate keypairs for two distinct security properties.

### `sentinel keygen` — Signing Keys (Integrity)

```bash
sentinel keygen --out-dir ./keys
```

Generates:

-   `sentinel_seed.b64` — private signing key (**KEEP SECRET**)
    
-   `sentinel_pub.b64` — public verification key (**SAFE TO SHARE**)
    

**Purpose:**

-   Sign audit checkpoints
    
-   Prove logs have not been modified
    

---

### `sentinel recipient-keygen` — Encryption Keys (Confidentiality)

```bash
sentinel recipient-keygen --out-dir ./keys
```

Generates:

-   `recipient_priv.b64` — private decryption key (**KEEP SECRET**)
    
-   `recipient_pub.b64` — public encryption key (**DISTRIBUTE**)
    

**Purpose:**

-   Encrypt audit logs at rest
    
-   Control who can read logs
    

----------

## Verifying Audit Logs

Verification is done offline, without Sentinel running.

```bash
sentinel verify \
  --log audit.jsonl \
  --pubkey-b64-path ./keys/sentinel_pub.b64 \
  --decrypt-recipient-privkey-b64-path ./keys/recipient_priv.b64
```

Verification confirms:

-   No events were removed
    
-   No events were modified
    
-   Event ordering is intact
    
-   Signatures are valid
    
-   Encrypted payloads decrypt correctly
    

----------


## Claude Desktop Integration

Claude Desktop integration is done by **manually editing the Claude Desktop MCP configuration** to run Sentinel as the MCP server wrapper. No automated installer is involved.

---

### How Integration Works

Claude Desktop launches MCP servers based on its configuration file.

To integrate Sentinel, you simply:

1. Locate the Claude Desktop MCP config file
2. Replace the MCP server command with `sentinel run -- <your-server-command>`
3. Include your MCP server (for example, a Node.js server file) as the wrapped command

That is all that is required.

---

### Example Configuration Pattern

Conceptually, if Claude Desktop previously ran:

```bash
node server.js
```

You change it to:

```bash
sentinel run -- node server.js
```

Sentinel now sits transparently between Claude Desktop (the agent) and the MCP server.

No other changes are needed.

----------

### Notes

-   Claude Desktop continues to function normally
    
-   Sentinel observes MCP traffic without modifying execution
    
-   This approach works with **any MCP server**, not just Node.js servers
    
-   Sentinel does not patch or modify Claude Desktop automatically
    
----------

### Backups

When editing the Claude Desktop config manually, it is recommended to keep a backup.

Typical config locations:

-   **macOS / Linux**  
    `~/.config/claude-desktop/claude_desktop_config.json`
    
-   **Windows**  
    `%APPDATA%\Claude\claude_desktop_config.json`
    

You may optionally create a backup copy before editing:

```text
claude_desktop_config.json.backup
```

    

---

## Environment Variables

```bash
export SENTINEL_WS_TOKEN="secret123"
```

> This avoids leaking tokens into shell history.

----------

## Advanced: Custom Stdio Pipelines (Developer Use)

```bash
node test-client.js | sentinel run -- node test-server.js
```

Useful for:

-   Development testing
    
-   Protocol fuzzing
    
-   Reproducing edge cases
    

> Most users do not need this.

----------

## Common Misconceptions

**“Do I need to run a client manually?”**  
No. Agents already exist and connect to MCP servers automatically.

**“Is Sentinel tied to Node.js?”**  
No. MCP servers are language-agnostic.

**“Is the example command mandatory?”**  
No. It is provided purely as a reference example.

----------

## Failure Behavior

Sentinel is **fail-open** by design.

If:

-   The dashboard crashes
    
-   The WebSocket disconnects
    
-   Observability code panics
    

Then:

-   MCP traffic continues uninterrupted
    
-   Agent execution is unaffected
    
-   Audit logs are flushed on shutdown
    

> Observability is never a control plane.

----------

## Summary

Sentinel provides:

-   Transparent MCP observability
    
-   Canonical event ordering
    
-   Cryptographically verifiable telemetry
    

Without:

-   Modifying execution
    
-   Enforcing policy
    
-   Becoming a single point of failure
    

> Observe, never decide.  
> Record, never enforce.