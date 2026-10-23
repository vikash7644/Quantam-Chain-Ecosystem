// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

contract QuantumDEX {
    using SafeERC20 for IERC20;

    IERC20 public immutable qtm;
    IERC20 public immutable pairToken;

    uint256 public reserveQTM;
    uint256 public reservePair;
    uint256 public totalShares;

    mapping(address => uint256) public shares;

    uint256 public constant FEE_NUM = 997;
    uint256 public constant FEE_DEN = 1000;

    event LiquidityAdded(
        address indexed provider,
        uint256 qtmAmount,
        uint256 pairAmount,
        uint256 liquidity
    );

    event LiquidityRemoved(
        address indexed provider,
        uint256 qtmAmount,
        uint256 pairAmount,
        uint256 liquidity
    );

    event Swap(
        address indexed user,
        bool indexed isQTMToPair,
        uint256 amountIn,
        uint256 amountOut
    );

    constructor(address _qtm, address _pairToken) {
        require(_qtm != address(0) && _pairToken != address(0), "Zero address");
        qtm = IERC20(_qtm);
        pairToken = IERC20(_pairToken);
    }

    function addLiquidity(uint256 _qtmAmount, uint256 _pairAmount)
        external
        returns (uint256 liquidity)
    {
        require(_qtmAmount > 0 && _pairAmount > 0, "Zero amount");

        qtm.safeTransferFrom(msg.sender, address(this), _qtmAmount);
        pairToken.safeTransferFrom(msg.sender, address(this), _pairAmount);

        if (totalShares == 0) {
            liquidity = sqrt(_qtmAmount * _pairAmount);
        } else {
            uint256 qtmPortion = (_qtmAmount * totalShares) / reserveQTM;
            uint256 pairPortion = (_pairAmount * totalShares) / reservePair;
            liquidity = qtmPortion < pairPortion ? qtmPortion : pairPortion;
        }

        require(liquidity > 0, "No liquidity minted");

        shares[msg.sender] += liquidity;
        totalShares += liquidity;

        reserveQTM += _qtmAmount;
        reservePair += _pairAmount;

        emit LiquidityAdded(msg.sender, _qtmAmount, _pairAmount, liquidity);
    }

    function removeLiquidity(uint256 _liquidity)
        external
        returns (uint256 qtmOut, uint256 pairOut)
    {
        require(_liquidity > 0 && _liquidity <= shares[msg.sender], "Invalid liquidity");

        qtmOut = (_liquidity * reserveQTM) / totalShares;
        pairOut = (_liquidity * reservePair) / totalShares;

        shares[msg.sender] -= _liquidity;
        totalShares -= _liquidity;

        reserveQTM -= qtmOut;
        reservePair -= pairOut;

        qtm.safeTransfer(msg.sender, qtmOut);
        pairToken.safeTransfer(msg.sender, pairOut);

        emit LiquidityRemoved(msg.sender, qtmOut, pairOut, _liquidity);
    }

    function swapQTMForPair(uint256 _amountIn) external returns (uint256 amountOut) {
        require(_amountIn > 0, "Zero amount");
        require(reserveQTM > 0 && reservePair > 0, "No liquidity");

        qtm.safeTransferFrom(msg.sender, address(this), _amountIn);

        uint256 feeAdjusted = (_amountIn * FEE_NUM) / FEE_DEN;
        amountOut = (reservePair * feeAdjusted) / (reserveQTM + feeAdjusted);

        require(amountOut < reservePair, "Not enough liquidity");

        reserveQTM += _amountIn;
        reservePair -= amountOut;

        pairToken.safeTransfer(msg.sender, amountOut);

        emit Swap(msg.sender, true, _amountIn, amountOut);
    }

    function swapPairForQTM(uint256 _amountIn) external returns (uint256 amountOut) {
        require(_amountIn > 0, "Zero amount");
        require(reserveQTM > 0 && reservePair > 0, "No liquidity");

        pairToken.safeTransferFrom(msg.sender, address(this), _amountIn);

        uint256 feeAdjusted = (_amountIn * FEE_NUM) / FEE_DEN;
        amountOut = (reserveQTM * feeAdjusted) / (reservePair + feeAdjusted);

        require(amountOut < reserveQTM, "Not enough liquidity");

        reservePair += _amountIn;
        reserveQTM -= amountOut;

        qtm.safeTransfer(msg.sender, amountOut);

        emit Swap(msg.sender, false, _amountIn, amountOut);
    }

    function sqrt(uint256 y) internal pure returns (uint256 z) {
        if (y > 3) {
            z = y;
            uint256 x = y / 2 + 1;
            while (x < z) {
                z = x;
                x = (y / x + x) / 2;
            }
        } else if (y != 0) {
            z = 1;
        }
    }
}
