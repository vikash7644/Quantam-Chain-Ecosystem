// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @title BVRX Token
/// @notice Fixed-supply ERC-20 compatible token for BNB Smart Chain.
/// @dev Supply is chosen once at deployment. There is no public mint or owner control.
contract BVRXToken is ERC20 {
    constructor(uint256 initialSupplyWholeTokens)
        ERC20("BVRX Token", "BVRX")
    {
        require(initialSupplyWholeTokens > 0, "BVRX: supply must be positive");
        _mint(msg.sender, initialSupplyWholeTokens * (10 ** decimals()));
    }
}
