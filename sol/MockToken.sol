// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

contract MockToken {
    // Slot 0
    mapping(address => uint256) public balanceOf;

    // Slot 1
    uint256 public totalSupply;

    event Transfer(address indexed from, address indexed to, uint256 value);
    event Approval(address indexed owner, address indexed spender, uint256 value);

    string public name = "Mock Token";
    string public symbol = "MCK";
    uint8 public decimals = 18;

    function transfer(address to, uint256 amount) external returns (bool) {
        balanceOf[msg.sender] -= amount;
        balanceOf[to] += amount;
        emit Transfer(msg.sender, to, amount);
        return true;
    }

    // Skip Allowance check.
    function transferFrom(address from, address to, uint256 amount) external returns (bool) {
        // Skip allowance[from][msg.sender] -= amount;
        balanceOf[from] -= amount;
        balanceOf[to] += amount;
        emit Transfer(from, to, amount);
        return true;
    }

    function approve(address spender, uint256 amount) external returns (bool) {
        emit Approval(msg.sender, spender, amount);
        return true;
    }

    function allowance(address, address) external pure returns (uint256) {
        return type(uint256).max;
    }
}