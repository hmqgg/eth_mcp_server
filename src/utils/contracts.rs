use alloy::sol;

sol!(
    #[sol(rpc)]
    IERC20,
    "abi/IERC20.json"
);

sol!(
    #[sol(rpc)]
    UniswapV3Quoter,
    "abi/UniswapV3Quoter.json"
);

sol!(
    #[sol(rpc)]
    UniswapV3Router,
    "abi/UniswapV3Router.json"
);

sol!(
    #[sol(rpc)]
    UniswapUniversalRouter,
    "abi/UniswapUniversalRouter.json"
);

sol!(
    #[sol(rpc)]
    UniswapPermit2,
    "abi/UniswapPermit2.json"
);
