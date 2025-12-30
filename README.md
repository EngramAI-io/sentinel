# Sentinel - The MCP Interceptor
![MCP Proxy Demo](./assets/Sentinel_Graph_Demo.gif)

> **A transparent, fail-open observability sidecar for Model Context Protocol (MCP).**  
> Sentinel provides **identity**, **ordering**, and **integrity** guarantees for agent ↔ tool interactions - without modifying execution or coupling to agents.

Sentinel sits invisibly between an LLM client and its MCP tools, acting as a **passive, non-blocking tap** into live agent traffic. It does not alter requests. It does not gate execution. It does not impose policy.

Instead, Sentinel observes MCP traffic as it flows, reconstructs a **canonical event stream**, and exposes it to real-time visualization and durable audit logs.

The result:  
You can finally **see what your agent did, in what order, and trust the record afterward** - even when the agent itself is a black box.

---

> ⚠️ **Project Status**  
> Sentinel is under active development. Core architecture and guarantees are implemented, with ongoing work on hardening, UX polish, documentation, and extended verification tooling.  
> Early adopters and reviewers are encouraged to provide feedback.

---

## What Sentinel Is (and Is Not)

**Sentinel is:**
- A transparent MCP sidecar
- A canonical event recorder
- A real-time observability layer
- A cryptographically verifiable audit tap

**Sentinel is not:**
- An execution engine
- A policy enforcer
- A tool broker
- A rate limiter
- A replay controller

Agents and tools continue to function **exactly the same** with or without Sentinel.

---

## Sentinel vs Official MCP Inspector

| Feature | Official MCP Inspector | Sentinel |
|------|-----------------------|----------|
| Primary Use | Isolated tool testing | Observing a **live agent** using tools |
| Integration | Separate web UI | Transparent sidecar in your workflow |
| Ordering | Implicit / undefined | **Canonical, monotonic event ordering** |
| Correlation | Per-request only | **Session, trace, and span continuity** |
| Auditability | None | **Append-only, signed, optional encrypted logs** |
| Failure Mode | Stops visibility | **Fail-open: execution continues** |

---

## Why Sentinel Exists

LLM agents reason implicitly.  
Tool calls happen asynchronously.  
Failures often appear *after* the cause.

Traditional logging breaks down.

Sentinel exists to provide **observability guarantees** that agent systems implicitly rely on but rarely formalize.

With Sentinel, you get:

- A single, ordered history of agent activity  
- Request → response correlation across tools  
- Latency and error visibility by method  
- Durable logs you can replay and verify later  
- A real-time graph that makes causality visible  

Think **Chrome DevTools** - 
but for **agent toolchains**.

---

## 🛡 Sentinel’s Core Guarantees

Sentinel provides **optional, non-invasive guarantees** around agent interactions.  
When enabled, these guarantees support debugging, auditing, and security analysis.  
When absent, agents continue to function normally - but these properties are lost.

---

### 1. Session Identity & Trace Continuity

**Guarantee:**  
Every observed agent action is causally attributable to a session and execution trace.

**Sentinel provides:**
- Stable `session_id` and `trace_id` for the lifetime of a run
- Per-request `span_id` with request ↔ response correlation
- Consistent attribution across tools and errors

**Without it:**  
Logs become fragmented and reasoning becomes opaque.

---

### 2. Canonical, Ordered Event Stream

**Guarantee:**  
There exists a single, consistent, replayable history of agent activity.

**Sentinel provides:**
- A monotonic, globally ordered event stream
- Append-only semantics
- Stable event IDs independent of wall-clock time

**Key property:**  
Ordering is derived from **observation at the Sentinel boundary**, not timestamps.

**Without it:**  
Events exist, but ordering is ambiguous and trust erodes.

---

### 3. Cryptographic Integrity of Telemetry

**Guarantee:**  
Observed agent behavior has not been tampered with after the fact.

**Sentinel provides:**
- Hash-chained, append-only audit records
- Ed25519 digital signatures
- Optional encryption at rest
- Offline verification tooling

**Important:**  
Cryptography applies **only to telemetry**, never to execution.

---

## Why These Guarantees Matter Together

| Property | Question Answered |
|--------|------------------|
| Identity | Who did what? |
| Order | In what sequence? |
| Integrity | Can we trust the record? |

Together, these form the minimum foundation required for **serious observability** - without enforcing policy or constraining agents.

---

## Design Philosophy

- Observe, never decide  
- Record, never enforce  
- Fail open, not closed  
- Trust comes from visibility, not control  

---

# ✨ Core Features

### 📡 Transparent MCP Sidecar
Drop Sentinel in front of any MCP server and instantly get deep visibility - no code changes, no rewrites, no patching.

### ⚡ Zero-Copy, Sub-Millisecond Proxying
Sentinel intercepts JSON-RPC traffic without buffering or mangling payloads. True pass-through with <1ms overhead.

### 🧠 Real-Time Interactive Graph
Every tool call becomes a glowing edge.  
Every response updates node stats.  
Errors pulse red.  
High-latency calls glow warm.

### 🔐 Built-in PII Redaction
Sensitive fields are scrubbed **only** in observability outputs.  
Original MCP traffic is never modified.

### 💥 Fail-Open Guarantee
If the UI crashes or the WebSocket drops, **your MCP pipeline continues unfazed**.

### 🖥️ Claude Desktop Ready
One command patches Claude Desktop while preserving automatic backups.

---

# 🏗 Architecture

**Pattern:** Transparent Sidecar (T-Tap)  
**Philosophy:** Fail Open  
**Tech Stack:** Rust (Tokio) • TypeScript • React • React Flow • WebSockets  

---

# 💎 Full Feature List

- Zero-copy stdin/stdout MCP pass-through  
- Canonical event sequencing  
- Session / trace / span correlation  
- WebSocket event bus  
- Full JSON-RPC parsing  
- Per-method latency & error metrics  
- Interactive force-directed graph  
- Panic recovery & safe shutdown  
- NDJSON audit logs for tooling integration  

---
## 🚀 Quick Installation

### Option 1: Curl Install (Recommended)

**Linux/macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/EngramAI-io/sentinel/main/install.sh | sh
```

<!-- **Build from source:**
```bash
curl -fsSL https://raw.githubusercontent.com/EngramAI-io/sentinel/main/install.sh | sh -s -- --source
``` -->

**Custom install directory:**
```bash
INSTALL_DIR=$HOME/.local/bin curl -fsSL https://raw.githubusercontent.com/EngramAI-io/sentinel/main/install.sh | sh
```
### Option 2: Windows (PowerShell)
```bash
iwr https://raw.githubusercontent.com/EngramAI-io/sentinel/main/install.ps1 -UseBasicParsing | iex
```



### Option 3: Download Pre-built Binary

Download the latest release for your platform:
- [Linux x86_64](https://github.com/EngramAI-io/sentinel/releases/download/v0.1.5/sentinel-x86_64-unknown-linux-musl.tar.gz)
- [macOS x86_64](https://github.com/EngramAI-io/sentinel/releases/download/v0.1.5/sentinel-x86_64-apple-darwin.tar.gz)
- [macOS ARM64 (M1/M2)](https://github.com/EngramAI-io/sentinel/releases/download/v0.1.5/sentinel-aarch64-apple-darwin.tar.gz)
- [Windows x86_64](https://github.com/EngramAI-io/sentinel/releases/download/v0.1.5/sentinel-x86_64-pc-windows-msvc.zip)

---

After Download (Linux/macOS only):
```bash
chmod +x sentinel-*
sudo mv sentinel-* /usr/local/bin/sentinel
```
---

## 📖 Usage & Documentation

Once Sentinel is installed, see the full usage guide for:

- Running Sentinel with MCP servers
- WebSocket authentication
- Audit logging & encryption
- Claude Desktop integration
- Common deployment patterns
- Troubleshooting

👉 **Read the full guide:** 
📄 [`docs/usage.md`](./docs/usage.md)
---

## 📚 Manual Build from Source

### Step 1: Install Prerequisites

#### 1.1 Install Rust and Cargo

**On Windows:**
1. Download and run the Rust installer by following the instructions at [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
2. Follow the installation prompts (defaults are recommended)
3. Restart your terminal/PowerShell after installation
4. Verify installation:
   ```powershell
   rustc --version
   cargo --version
   ```
   You should see versions like `rustc 1.70.0` or higher, and `cargo 1.70.0` or higher.
5. If you're a **Windows Subsystem for Linux (WSL)** user, run the following in your terminal, then follow the on-screen instructions to install Rust.
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

**On macOS:**
```bash
# Install using Homebrew
brew install rust

# Or use rustup (recommended)
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

**On Linux:**
```bash
# Install using rustup (recommended)
curl --proto '=https' --tlsv1.2 https://sh.rustup.rs -sSf | sh
source $HOME/.cargo/env

# Verify installation
rustc --version
cargo --version
```

#### 1.2 Install Node.js and npm

**On Windows:**
1. Download Node.js from [https://nodejs.org/](https://nodejs.org/)
2. Choose the LTS version (18.x or higher recommended)
3. Run the installer and follow the prompts
4. Verify installation:
   ```powershell
   node --version
   npm --version
   ```
   You should see `v18.x.x` or higher for Node.js, and `9.x.x` or higher for npm.

**On macOS:**
```bash
# Using Homebrew
brew install node

# Verify installation
node --version
npm --version
```

**On Linux:**
```bash
# Install nvm (Node Version Manager)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.3/install.sh | bash

# Install Node.js using nvm
nvm install node

# Verify installation
node --version
npm --version
```

### Step 2: Clone and Navigate to Project

```bash
# If you haven't already cloned the repository
git clone https://github.com/EngramAI-io/sentinel.git
cd sentinel

# Or if you're already in the project directory, verify you're in the right place
# You should see Cargo.toml and a frontend/ directory
```

### Step 3: Build the Frontend

1. Navigate to the frontend directory:
   ```bash
   cd frontend/my-react-flow-app
   ```

2. Install frontend dependencies:
   ```bash
   npm install
   ```
   This will download and install all required packages (React, React Flow, Vite, TypeScript, etc.)

3. Build the frontend for production:
   ```bash
   npm run build
   ```
   This compiles TypeScript, bundles the React app, and outputs to `frontend/dist/` directory.

4. Return to the project root:
   ```bash
   cd ..
   ```

### Step 4: Build the Rust Binary

1. Build the release version (optimized):
   ```bash
   cargo build --release
   ```
   
   **Note:** The first build may take 5-10 minutes as it compiles all dependencies. Subsequent builds will be much faster.

2. Verify the binary was created:
   - **Windows:** `target\release\sentinel.exe`
   - **macOS/Linux:** `target/release/sentinel`

3. (Optional) Test that the binary runs:
   ```bash
   # Windows
   .\target\release\sentinel.exe --help
   
   # macOS/Linux
   ./target/release/sentinel --help
   ```

### Step 5: Test Locally

#### Option A: Run with Development Build (Recommended for Testing)

1. In one terminal, run the sentinel proxy with an MCP server:
   ```bash
   cargo run -- run -- npx -y @modelcontextprotocol/server-filesystem
   ```
   
   This will:
   - Start the sentinel proxy
   - Launch the MCP filesystem server
   - Start the HTTP/WebSocket server on port 3000

2. Open your browser and navigate to:
   ```
   http://localhost:3000
   ```

3. You should see the dashboard with:
   - A force-directed graph visualization
   - An "Agent" node in the center
   - Tool nodes appearing as MCP requests are made

4. To test with a different MCP server:
   ```bash
   # SQLite server example
   cargo run -- run -- npx -y @modelcontextprotocol/server-sqlite path/to/database.db
   
   # Custom Python server example
   cargo run -- run -- python -m my_mcp_server
   ```

#### Option B: Run with Release Binary

1. Use the release binary you built:
   ```bash
   # Windows
   .\target\release\sentinel.exe run -- npx -y @modelcontextprotocol/server-filesystem
   
   # macOS/Linux
   ./target/release/sentinel run -- npx -y @modelcontextprotocol/server-filesystem
   ```

2. Open http://localhost:3000 in your browser

### Step 6: Verify Everything Works

1. **Check the dashboard loads:** You should see the visualization interface
2. **Check WebSocket connection:** The graph should be interactive
3. **Check logs:** Look for `sentinel_debug.jsonl` in the project directory for debug logs
4. **Test interactivity:** Click on nodes to see detailed JSON-RPC payloads

### Troubleshooting

**Issue: `cargo: command not found`**
- Solution: Restart your terminal after installing Rust, or manually add `~/.cargo/bin` to your PATH

**Issue: `npm: command not found`**
- Solution: Restart your terminal after installing Node.js, or verify Node.js is in your PATH

**Issue: Frontend build fails**
- Solution: Make sure you're in the `frontend/` directory and run `npm install` first

**Issue: Port 3000 already in use**
- Solution: Kill the process using port 3000, or modify the port in the code

**Issue: `cargo build` fails with linker errors (Windows)**
- Solution: Install Microsoft C++ Build Tools from [https://visualstudio.microsoft.com/visual-cpp-build-tools/](https://visualstudio.microsoft.com/visual-cpp-build-tools/)

**Issue: Binary not found after build**
- Solution: Check that the build completed successfully. Look for `target/release/sentinel` (or `.exe` on Windows)

### Install for Claude Desktop

```bash
# Install sentinel for a specific MCP server
./target/release/sentinel install filesystem

# The config is automatically updated and backed up
# Your Claude Desktop will now use sentinel as a proxy
```

To restore the original config:
```bash
# Config backup is at: ~/.config/claude-desktop/claude_desktop_config.json.backup (Unix)
# or %APPDATA%/Claude/claude_desktop_config.json.backup (Windows)
```

## Project Structure

```
sentinel/
├── src/
│   ├── audit.rs             # Audit log writer and lifecycle management
│   ├── audit_crypto.rs      # Signing, hashing, and encryption logic for tamper-evident logs
│   ├── config.rs            # Claude Desktop config helper  
│   ├── decrypt_audit_log.rs # Signing, hashing, and encryption logic for tamper-evident logs
│   ├── events.rs            # Event logging structures
│   ├── keygen.rs            # Offline audit log verification and decryption
│   ├── main.rs              # CLI and orchestration
│   ├── panic.rs             # Panic recovery
│   ├── proxy.rs             # Zero-copy stdio proxy
│   ├── protocol.rs          # JSON-RPC structures
│   ├── parser.rs            # NDJSON streaming parser
│   ├── session.rs           # Request/response correlation
│   ├── server.rs            # HTTP/WebSocket server
│   └── redaction.rs         # PII redaction
└── frontend/                # React dashboard
    └── src/
        ├── App.tsx
        ├── components/
        │   ├── Graph.tsx
        │   └── NodeDetails.tsx
        └── hooks/
            └── useWebSocket.ts
```

## Development

### Frontend Development

```bash
cd frontend
npm install
npm run dev
```

This runs the frontend in dev mode (not embedded). To connect to a running sentinel instance, update the WebSocket URL in `src/hooks/useWebSocket.ts`.

### Build Frontend for Embedding

```bash
cd frontend
npm run build
# Output goes to frontend/dist/ which gets embedded into the binary
```

## Usage Examples

### Example: MCP Filesystem Server

```bash
# Development (no auth)
cargo run -- run -- npx -y @modelcontextprotocol/server-filesystem

# Production (with auth)
cargo run -- run --ws-token "secret123" --ws-bind "127.0.0.1:3000" -- npx -y @modelcontextprotocol/server-filesystem
```

### Example: MCP SQLite Server

```bash
cargo run -- run --ws-token "$SENTINEL_WS_TOKEN" -- npx -y @modelcontextprotocol/server-sqlite path/to/database.db
```

### Example: Custom Server with Audit Logging

```bash
# First generate keys
cargo run -- keygen --out-dir ./keys
cargo run -- recipient-keygen --out-dir ./keys

# Run with full audit logging and encryption
cargo run -- run \
  --signing-key-b64-path ./keys/signing_key.b64 \
  --encrypt-recipient-pubkey-b64-path ./keys/recipient_pub.b64 \
  --audit-log ./audit.jsonl \
  --ws-token "secret123" \
  -- python -m my_mcp_server
```

## Dashboard

The dashboard visualizes:
- **Agent Node** (center): The LLM client
- **Tool Nodes** (satellites): MCP servers and their methods
- **Edges**: Active communication paths
- **Colors**: 
  - Yellow = Pending request
  - Green = Successful response
  - Red = Error response

Click on any node to see detailed JSON-RPC payloads.

## Debugging

- Audit logs written to `sentinel_audit.jsonl` (configurable via `--audit-log`)
- Panic logs go to `sentinel_panic.log`
- Config backups are created automatically with `.backup` extension
- Console output shows:
  - ✅ Success indicators (green checkmarks)
  - ❌ Error indicators (red X marks)
  - 🔒 Security status (lock icons)
  - ⚠️  Warnings (warning triangles)
  - 📝 Checkpoints and audit events

## Security

### 🔒 Built-in Security Features

Sentinel is designed with security-first principles:

#### **Authentication**
- **WebSocket Token Authentication**: Protect your observability endpoint with token-based auth
- **Environment Variable Support**: Set `SENTINEL_WS_TOKEN` to avoid exposing tokens in command history
- **Automatic Warning**: Sentinel warns when running without authentication in production

```bash
# Secure mode (recommended)
./sentinel run --ws-token "your-secret-token-here" -- npx @modelcontextprotocol/server-filesystem

# Using environment variable (recommended for production)
export SENTINEL_WS_TOKEN="your-secret-token-here"
./sentinel run -- npx @modelcontextprotocol/server-filesystem

# Connect from browser/client
ws://localhost:3000/ws?token=your-secret-token-here
```

#### **Configurable Network Binding**
- **Custom Bind Address**: Control where the WebSocket server listens
- **Production-Ready**: Bind to specific interfaces or use Unix sockets

```bash
# Localhost only (default - most secure)
./sentinel run --ws-bind "127.0.0.1:3000" -- your-mcp-server

# All interfaces (use with authentication!)
./sentinel run --ws-bind "0.0.0.0:3000" --ws-token "secret" -- your-mcp-server

# Custom port
./sentinel run --ws-bind "127.0.0.1:8080" -- your-mcp-server
```

#### **PII Redaction**
- API keys, tokens, emails automatically scrubbed before visualization
- Original MCP traffic is **never** modified - redaction only applies to observability data
- Regex-based detection with multiple pattern types:
  - `api_key`, `apikey`, `access_token`, `secret_key`
  - OpenAI-style `sk-*` keys
  - Email addresses
  - Bearer tokens

#### **Cryptographic Audit Logging**
- **Ed25519 Digital Signatures**: Every checkpoint is cryptographically signed
- **ChaCha20-Poly1305 Encryption**: Optional end-to-end encryption for audit logs
- **Tamper-Evident Chains**: Hash-chained events prevent retroactive modification
- **Verifiable Logs**: Independent verification with `sentinel verify` command

```bash
# Generate signing keypair
./sentinel keygen --out-dir ./keys

# Generate encryption keypair (optional)
./sentinel recipient-keygen --out-dir ./keys

# Run with audit logging
./sentinel run \
  --signing-key-b64-path ./keys/signing_key.b64 \
  --encrypt-recipient-pubkey-b64-path ./keys/recipient_pub.b64 \
  --audit-log ./audit.jsonl \
  -- your-mcp-server

# Verify audit log integrity
./sentinel verify \
  --log ./audit.jsonl \
  --pubkey-b64-path ./keys/signing_pubkey.b64 \
  --decrypt-recipient-privkey-b64-path ./keys/recipient_priv.b64
```

#### **Graceful Shutdown**
- **Signal Handling**: CTRL+C triggers graceful shutdown
- **Flush Guarantees**: All audit logs are flushed to disk before exit
- **Timeout Protection**: 10-second timeout prevents hanging on shutdown
- **No Data Loss**: Event buffers are drained completely

#### **Secure Dependencies**
- **Pinned Versions**: All dependencies use specific versions (no wildcards)
- **Latest Security Patches**: tokio-tungstenite 0.24 with all CVEs patched
- **Memory Safety**: Pure Rust implementation with zero `unsafe` blocks
- **Crypto Libraries**: RustCrypto audited implementations

#### **Error Handling**
- **Detailed Logging**: Comprehensive error information for debugging
- **User-Friendly Messages**: Generic error messages prevent information disclosure
- **Fail-Open Proxy**: MCP traffic continues even if observability fails
- **Panic Recovery**: Custom panic handler prevents crashes from affecting MCP pipeline

### Security Best Practices

1. **Always use authentication** in production environments
2. **Bind to localhost** unless you need remote access
3. **Enable audit log encryption** for sensitive environments
4. **Rotate keys periodically** using the keygen commands
5. **Monitor logs** for unauthorized access attempts
6. **Keep Sentinel updated** to get latest security patches

### Security Disclosure

Found a security issue? Please email security@engramai.io (or your contact) instead of opening a public issue.

### Config Backups

- Config backups are created automatically before any modifications
- Backup location: 
  - **Linux/macOS**: `~/.config/claude-desktop/claude_desktop_config.json.backup`
  - **Windows**: `%APPDATA%\Claude\claude_desktop_config.json.backup`

## License

MIT