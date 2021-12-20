# Proactive market making curve for token-swap

Add additional curve to token-swap program that support proactive market making.

The core of PMM is essentially calculating one integral and solving two quadratic equations. It is implemented in Dodo protocol described [here](https://dodoex.github.io/docs/docs/pmm).

As a part of PR please implement as described in [[1](https://dodoex.github.io/docs/docs/pmmDetails)] and [[2](https://dodoex.github.io/docs/docs/math)].


For reference pricing formula is: 

<img src="https://render.githubusercontent.com/render/math?math=P_{margin}=iR">

Where <img src="https://render.githubusercontent.com/render/math?math=R"> is defined to be the piecewise function below:

<img src="https://render.githubusercontent.com/render/math?math=if \ B<B_0, \ R=1-k+(\frac{B_0}{B})^2k">
<br>
<img src="https://render.githubusercontent.com/render/math?math=if \ Q<Q_0, \ R=1/(1-k+(\frac{Q_0}{Q})^2k)">
<br>
<img src="https://render.githubusercontent.com/render/math?math=else \ R=1else R=1,">

<img src="https://render.githubusercontent.com/render/math?math=i"> is the market price provided by an oracle, and <img src="https://render.githubusercontent.com/render/math?math=k"> is a parameter in the range (0, 1).

The funding pool of PMM is described by four parameters:

- <img src="https://render.githubusercontent.com/render/math?math=B_0">: base token regression target - total number of base tokens deposited by liquidity providers
- <img src="https://render.githubusercontent.com/render/math?math=Q_0">: quote token regression target - total number of quote tokens deposited by liquidity providers
- <img src="https://render.githubusercontent.com/render/math?math=B">: base token balance - number of base tokens currently in the pool
- <img src="https://render.githubusercontent.com/render/math?math=Q">: quote token balance - number of quote tokens currently in the pool
