// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "./QuantumToken.sol";

contract PumpQuantum {
    QuantumToken public immutable qtm;

    uint256 public totalPumpSupply;
    uint256 public reserveQTM;

    mapping(address => uint256) public balanceOf;

    uint256 public constant INITIAL_PRICE = 1 ether;

    event Bought(address indexed buyer, uint256 qtmAmount, uint256 pumpTokens);
    event Sold(address indexed seller, uint256 pumpTokens, uint256 qtmAmount);

    constructor(address _qtm) {
        require(_qtm != address(0), "Zero address");
        qtm = QuantumToken(_qtm);
    }

    function buy(uint256 _qtmAmount) external {
        require(_qtmAmount > 0, "Zero amount");

        qtm.transferFrom(msg.sender, address(this), _qtmAmount);

        uint256 tokensToMint = _calculateMint(_qtmAmount);

        totalPumpSupply += tokensToMint;
        balanceOf[msg.sender] += tokensToMint;
        reserveQTM += _qtmAmount;

        emit Bought(msg.sender, _qtmAmount, tokensToMint);
    }

    function sell(uint256 _pumpTokens) external {
        require(_pumpTokens > 0 && _pumpTokens <= balanceOf[msg.sender], "Invalid amount");

        uint256 qtmOut = _calculateBurn(_pumpTokens);

        balanceOf[msg.sender] -= _pumpTokens;
        totalPumpSupply -= _pumpTokens;
        reserveQTM -= qtmOut;

        qtm.transfer(msg.sender, qtmOut);

        emit Sold(msg.sender, _pumpTokens, qtmOut);
    }

    function _calculateMint(uint256 _qtmAmount) internal view returns (uint256) {
        if (totalPumpSupply == 0) {
            return (_qtmAmount * 1e18) / INITIAL_PRICE;
        }

        return (_qtmAmount * totalPumpSupply) / reserveQTM;
    }

    function _calculateBurn(uint256 _pumpTokens) internal view returns (uint256) {
        if (totalPumpSupply == 0) return 0;

        return (_pumpTokens * reserveQTM) / totalPumpSupply;
    }
}
