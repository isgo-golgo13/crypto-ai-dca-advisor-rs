# Crypto AI DCA (Dollar-Cost Averaging) Invest Advisor (Rust)
AI-Driven (LLM Agent using Ollama RS) Cryptocurrency DCA (Dollar-Cost-Averaging) Invest Advisor using Rust, Rust Leptos WASM (Front-End) Framework, Rust Axum WASM Web Framework, Rust Tokio Async, Rust Stripe API for Stripe Checkout

A full-stack Rust LLM agent with local Ollama inference, Stripe payments, and a cryptocurrency investment advisor.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              rust-agent                                     │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌─────────────┐     ┌─────────────┐     ┌──────────────────┐              │
│   │ agent-web   │────▶│agent-server │────▶│  agent-runtime   │              │
│   │  (Leptos)   │     │   (Axum)    │     │    (Ollama)      │              │
│   └─────────────┘     └──────┬──────┘     └──────────────────┘              │
│                              │                                              │
│        ┌─────────────────────┼─────────────────────┐                        │
│        │                     │                     │                        │
│        ▼                     ▼                     ▼                        │
│   ┌─────────────┐     ┌─────────────┐     ┌──────────────────┐              │
│   │ agent-core  │     │agent-payments│    │  crypto-advisor  │              │
│   │ (Strategy)  │     │  (Stripe)    │    │ (Domain Service) │              │
│   └─────────────┘     └─────────────┘     └──────────────────┘              │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

## Features

- **Full-Stack Rust** - Backend (Axum), frontend (Leptos/WASM), CLI
- **Local LLM** - Runs on Ollama, your data never leaves your machine  
- **Stripe Checkout** - Simple hosted payment integration
- **Crypto Advisor** - DCA calculator, risk analyzer, portfolio tracker


## Quick Start

```bash
# 1. Clone and setup
git clone <your-repo>
cd rust-agent
make setup

# 2. Start Ollama
ollama serve &
ollama pull llama3.2

# 3. Run the server
make dev

# 4. Open http://localhost:3000
```

## Project Structure

```shell
rust-agent/
├── Cargo.toml                 # Workspace root
├── Makefile                   # Dev commands
├── .env.example               # Configuration template
├── index.html                 # WASM entry point
├── styles.css                 # Frontend styles
├── Trunk.toml                 # WASM build config
│
├── crates/
│   ├── agent-core/            # Core abstractions
│   │   ├── provider.rs        # LlmProvider trait (Strategy pattern)
│   │   ├── tool.rs            # Tool trait + registry
│   │   ├── reasoning.rs       # Agent loop (ReAct)
│   │   └── ...
│   │
│   ├── agent-runtime/         # Provider implementations
│   │   └── ollama.rs          # Ollama integration
│   │
│   ├── agent-server/          # HTTP server
│   │   ├── main.rs            # Entry point
│   │   ├── handlers.rs        # API endpoints
│   │   └── state.rs           # Shared state
│   │
│   ├── agent-payments/        # Payment processing
│   │   ├── checkout.rs        # Stripe Checkout (Hosted)
│   │   ├── webhook.rs         # Webhook handling
│   │   └── license.rs         # License management
│   │
│   ├── agent-web/             # WASM frontend
│   │   └── src/pages/         # Leptos components
│   │
│   └── crypto-advisor/        # Domain-specific tools
│       ├── svckit/            # Tools (price_lookup, dca_calculator, etc.)
│       ├── strategy/          # DCA, diversification algorithms
│       ├── exchange/          # Exchange API abstractions
│       └── model.rs           # Portfolio, Asset, Position types
│
└── docs/
    └── STRIPE_CHECKOUT_EXPLAINED.md
```



## Crates Overview

| Crate | Purpose |
|-------|---------|
| `agent-core` | Provider trait, Tool trait, Agent reasoning loop |
| `agent-runtime` | Ollama provider (add OpenAI/Anthropic later) |
| `agent-server` | Axum HTTP/WebSocket server |
| `agent-payments` | Stripe Checkout + license management |
| `agent-web` | Leptos WASM frontend |
| `crypto-advisor` | Cryptocurrency investment tools |

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check + tool list |
| `/api/models` | GET | List available Ollama models |
| `/api/chat` | POST | Send message, get response |
| `/api/chat/stream` | WS | Streaming responses |
| `/api/checkout` | POST | Create Stripe checkout session |
| `/api/license/verify` | POST | Verify license key |
| `/webhook/stripe` | POST | Stripe webhook handler |


### Chat Request

```json
{
  "message": "I want to invest $1000 in crypto conservatively",
  "crypto_mode": true,
  "model": "llama3.2",
  "license_key": "XXXX-XXXX-XXXX-XXXX"
}
```

## Crypto Advisor Tools

| Tool | Description |
|------|-------------|
| `price_lookup` | Get current cryptocurrency prices |
| `dca_calculator` | Calculate DCA allocations based on risk profile |
| `risk_analyzer` | Analyze volatility, max drawdown, risk tiers |
| `portfolio_tracker` | Track positions, P&L, allocations |

### Template Conversation

```
User: I have $1000 to invest. What's a safe approach?

Agent: Let me analyze this for you.

[Uses risk_analyzer to get volatility data]
[Uses dca_calculator with conservative profile]

For $1000 with a conservative approach, I recommend spreading 
across 10 assets with these allocations:

  BTC   20.0%  $200.00  (0.002051 units) - Blue chip
  ETH   20.0%  $200.00  (0.057971 units) - Blue chip
  SOL   10.0%  $100.00  (0.512821 units) - Large cap
  ...

Risk Distribution:
  Low risk (BTC/ETH):  $400.00 (40.0%)
  Medium risk:         $400.00 (40.0%)
  Higher risk:         $200.00 (20.0%)

⚠️ ALL-IN COMPARISON:
If you put $1000 into a single volatile asset:
- Could gain 100%+ in a bull run
- Could lose 80-100% in a crash
- Recovery could take years

Diversified approach reduces max drawdown by ~50%
```

## Stripe Integration

We use **Stripe Checkout (Hosted)** - the simpler approach where users are redirected to Stripe's secure payment page.

See [docs/STRIPE_CHECKOUT_EXPLAINED.md](docs/STRIPE_CHECKOUT_EXPLAINED.md) for the 5-year-old explanation.

```bash
# Test locally with Stripe CLI
stripe listen --forward-to localhost:3000/webhook/stripe
```

## Adding Custom Tools

```rust
use agent_core::{Tool, ToolSchema, ToolCall, ToolResult, Result};

pub struct MyTool;

#[async_trait]
impl Tool for MyTool {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: "my_tool".into(),
            description: "Does something useful".into(),
            parameters: vec![],
            category: Some("custom".into()),
            has_side_effects: false,
        }
    }
    
    async fn execute(&self, call: &ToolCall) -> Result<ToolResult> {
        Ok(ToolResult::success("my_tool", "It worked!"))
    }
}

// Register in agent-server/src/main.rs
tools.register(MyTool);
```

## Configuration

Copy `.env.example` to `.env` and configure:

```env
# Server
BIND_ADDR=0.0.0.0:3000
RUST_LOG=info

# Ollama
OLLAMA_HOST=http://localhost
OLLAMA_PORT=11434

# Stripe
STRIPE_SECRET_KEY=sk_test_xxx
STRIPE_WEBHOOK_SECRET=whsec_xxx
```

## Development Commands

```bash
make dev          # Run with auto-reload
make test         # Run tests
make lint         # Run clippy
make build-release # Build optimized binary
make stripe-listen # Forward Stripe webhooks
```

## License

MIT
