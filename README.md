# Ethereum Trading MCP Server

A Rust implementation of the [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) that enables AI agents to query Ethereum balances, fetch token prices, and simulate Uniswap V3 swaps.

Implement according to [ASSIGNMENT](./ASSIGNMENT.md).

## Features

This MCP server provides three core tools:

1. **`get_balance`** - Query ETH and ERC20 token balances
   - Input: wallet address, optional token symbol or address
   - Output: formatted balance with correct decimals

2. **`get_token_price`** - Get current token price
   - Input: token symbol/address, currency symbol/address
   - Output: token price (queries all Uniswap V3 fee tiers and returns the best price)

3. **`swap_tokens`** - Simulate Uniswap V3 token swap
   - Input: from token, to token, amount, slippage tolerance
   - Output: estimated output amount and gas cost
   - **Note**: Simulation only - no transaction will be broadcast to the blockchain

## Tech Stack

- **Rust**/**Tokio**
- **Alloy** - Ethereum RPC client library
- **rmcp** - Model Context Protocol Rust SDK
- **rust_decimal** - High-precision financial calculations
- **tracing** - Structured logging

## Prerequisites

- Rust (install via [rustup](https://rustup.rs/))
- Ethereum mainnet RPC endpoint (Infura, Alchemy, or public node) **MUST** support `statesOverride` for `eth_call`
- Ethereum private key (for signing only, no transactions will be broadcast)

## Setup Instructions

### 1. Clone the Repository

```bash
git clone <repository-url>
cd eth_mcp_server_private
```

### 2. Configure Environment Variables

Create a `.env` file or export the following variables in your shell:

```bash
# Ethereum mainnet RPC URL
export ETH_RPC_URL="https://eth.llamarpc.com"

# Ethereum private key (64-character hex string with 0x prefix)
export ETH_PRIVATE_KEY="0x0000000000000000000000000000000000000000000000000000000000000001"
```

**Security Notes**:

- Do not use private keys with real funds in production
- Use a test private key or create a new wallet specifically for this purpose (e.g. `cast wallet new`)
- This server only simulates transactions and won't broadcast real ones, but always handle private keys carefully

### 3. Build the Project

```bash
cargo build --release
```

### 4. Run the Server

```bash
cargo run --release
```

The server communicates with MCP clients via standard input/output (stdio).

## Usage Examples

### Testing with Clients that support custom mcpServers

```json
{
  "mcpServers": {
    "eth_mcp_server": {
      "command": "cargo",
      "args": ["run", "--manifest-path", "${workspaceFolder}/Cargo.toml"],
      "env": {
        "ETH_PRIVATE_KEY": "0x...",
        "ETH_RPC_URL": "https://..."
      }
    }
  }
}
```

### Testing with MCP Inspector

*❤️ Recommended*

Install and use the official MCP Inspector:

```bash
npx @modelcontextprotocol/inspector -e ETH_RPC_URL="https://..." -e ETH_PRIVATE_KEY="0x..." cargo run
```

It will open up a web page that demonstrates MCP server tools.

### Example 1: Query Balance

**Request**:

```js
{
  "method": "tools/call",
  "params": {
    "name": "get_balance",
    "arguments": {
      "wallet_address": "0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045",
      "token": "USDC" // or contract address.
    }
  }
}
```

**Response**:

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"balance\":\"1234.567890\"}"
    }
  ]
}
```

### Example 2: Get Token Price

**Request**:

```json
{
  "method": "tools/call",
  "params": {
    "name": "get_token_price",
    "arguments": {
      "token": "WETH", // or contract address.
      "currency": "USDC" // or contract address.
    }
  }
}
```

**Response**:

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"price\":\"3456.789012\"}"
    }
  ]
}
```

### Example 3: Simulate Token Swap

**Request**:

```js
{
  "method": "tools/call",
  "params": {
    "name": "swap_tokens",
    "arguments": {
      "from_token": "USDC", // or contract address.
      "to_token": "WETH", // or contract address.
      "amount_from": "1000",
      "slippage_percent": "0.5"
    }
  }
}
```

**Response**:

```json
{
  "content": [
    {
      "type": "text",
      "text": "{\"amount_to\":\"0.289123456789\",\"gas_estimate\":185000}"
    }
  ]
}
```

## Testing

Run unit tests:

```bash
cargo test
```

Test coverage includes:

- Address parsing and validation
- Serialization/deserialization precision guarantees
- Decimal conversions (U256 ↔ Decimal)
- Token registry resolution
- State override generation

**Note**: Some integration tests require a valid network connection.

## Design Decisions

1. **Precision Guarantee**: Uses `rust_decimal::Decimal` for all amounts and serializes them as strings to avoid floating-point precision loss. This ensures financial calculations remain accurate across the entire pipeline.

2. **Uniswap V3 Priority**: Price queries and swaps exclusively use Uniswap V3, iterating through all fee tiers (0.01%, 0.05%, 0.3%, 1%) to find the best price/liquidity. This approach maximizes execution quality while keeping the implementation focused.

3. **State Override Simulation**: Swap simulation uses `eth_call` with state overrides to simulate transactions without holding actual tokens. This involves injecting MockToken contract bytecode (bypassing allowance checks) and setting wallet balance to `U256::MAX`, ensuring simulations don't require real funds.

4. **Flexible Token Resolution**: Supports both token symbols (e.g., "USDC") and addresses (e.g., "0x...") as inputs. Symbol resolution uses the Uniswap token list, providing a convenient user experience while maintaining the ability to use arbitrary contract addresses.

5. **Contextual Error Handling**: Uses `anyhow::Context` to add context information to every operation, making errors more debuggable and user-friendly by clearly indicating which step failed and why.

## Known Limitations

- **Ethereum Mainnet Only**: Current `CHAIN_ID` is hardcoded to 1 (extensible to other chains)
- **Token List Dependency**: Relies on Uniswap's official token list (`tokens.uniswap.org`); symbol queries will fail if the list is unavailable or the token is not listed
- **Gas Estimation Accuracy**: State overrides may cause gas estimates to differ from actual on-chain execution

## Project Structure

```shell
eth_mcp_server_private/
├── src/
│   ├── main.rs              # Server entry point
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── router.rs        # MCP tool router
│   │   ├── balance.rs       # Balance queries
│   │   ├── price.rs         # Price queries
│   │   └── swap.rs          # Swap simulation
│   └── utils/
│       ├── mod.rs
│       ├── provider.rs      # RPC provider and wallet
│       ├── contracts.rs     # Contract ABI bindings
│       ├── decimals.rs      # Precision conversion
│       └── token_registry.rs # Token symbol resolution
├── abi/                     # Uniswap contract ABIs
├── sol/                     # MockToken contract
├── Cargo.toml
└── README.md
```
