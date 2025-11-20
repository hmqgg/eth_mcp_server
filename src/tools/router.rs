use crate::tools::{
    balance::{BalanceRequest, get_balance},
    price::{PriceRequest, get_token_price},
    swap::{SwapRequest, swap_tokens},
};
use rmcp::{
    ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, ErrorData, ServerCapabilities, ServerInfo},
    tool, tool_handler, tool_router,
};

#[derive(Debug, Clone)]
pub struct EthTools {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl EthTools {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Query ETH and ERC20 token balances;\n
    If token address is not provided, the balance of native asset will be returned;\n
    Output: balance in formatted decimal format.
    ")]
    async fn get_balance(
        &self,
        Parameters(BalanceRequest {
            wallet_address,
            token,
        }): Parameters<BalanceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match get_balance(wallet_address, token).await {
            Ok(resp) => {
                let value = serde_json::to_value(resp)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::structured(value))
            }
            Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
        }
    }

    #[tool(
        description = "Get the price of a token in the specified currency by querying Uniswap V3 Quoter.\n
    Output: price in formatted decimal format.
    "
    )]
    async fn get_token_price(
        &self,
        Parameters(PriceRequest { token, currency }): Parameters<PriceRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match get_token_price(token, currency).await {
            Ok(resp) => {
                let value = serde_json::to_value(resp)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::structured(value))
            }
            Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
        }
    }

    #[tool(
        description = "Simulate a Uniswap V3 token swap to estimate output amount and gas cost.\n
        This is a simulation only - no transaction will be broadcast to the blockchain.\n
        Output: estimated amount_out and gas_estimate.
        "
    )]
    async fn swap_tokens(
        &self,
        Parameters(SwapRequest {
            from_token,
            to_token,
            amount_from,
            slippage_percent,
        }): Parameters<SwapRequest>,
    ) -> Result<CallToolResult, ErrorData> {
        match swap_tokens(from_token, to_token, amount_from, slippage_percent).await {
            Ok(resp) => {
                let value = serde_json::to_value(resp)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::structured(value))
            }
            Err(e) => Err(ErrorData::internal_error(e.to_string(), None)),
        }
    }
}

#[tool_handler]
impl ServerHandler for EthTools {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some("ETH trading MCP server".into()),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}
