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
        tracing::info!("get_balance called: wallet={}, token={:?}", wallet_address, token);
        match get_balance(wallet_address.clone(), token.clone()).await {
            Ok(resp) => {
                tracing::info!("get_balance succeeded: wallet={}, balance={}", wallet_address, resp.balance);
                let value = serde_json::to_value(resp)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::structured(value))
            }
            Err(e) => {
                tracing::error!("get_balance failed: wallet={}, token={:?}, error={}", wallet_address, token, e);
                Err(ErrorData::internal_error(e.to_string(), None))
            }
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
        tracing::info!("get_token_price called: token={}, currency={}", token, currency);
        match get_token_price(token.clone(), currency.clone()).await {
            Ok(resp) => {
                tracing::info!("get_token_price succeeded: token={}, currency={}, price={}", token, currency, resp.price);
                let value = serde_json::to_value(resp)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::structured(value))
            }
            Err(e) => {
                tracing::error!("get_token_price failed: token={}, currency={}, error={}", token, currency, e);
                Err(ErrorData::internal_error(e.to_string(), None))
            }
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
        tracing::info!("swap_tokens called: from={}, to={}, amount={}, slippage={}%", 
            from_token, to_token, amount_from, slippage_percent);
        match swap_tokens(from_token.clone(), to_token.clone(), amount_from.clone(), slippage_percent.clone()).await {
            Ok(resp) => {
                tracing::info!("swap_tokens succeeded: from={}, to={}, amount_out={}, gas={}", 
                    from_token, to_token, resp.amount_to, resp.gas_estimate);
                let value = serde_json::to_value(resp)
                    .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
                Ok(CallToolResult::structured(value))
            }
            Err(e) => {
                tracing::error!("swap_tokens failed: from={}, to={}, amount={}, error={}", 
                    from_token, to_token, amount_from, e);
                Err(ErrorData::internal_error(e.to_string(), None))
            }
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
