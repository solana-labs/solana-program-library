# TypeScript bindings for stake-pool program

For use with both node.js and in-browser.

## Installation

```
npm install
```

## Build and run

In the `js` folder:

```
npm run build
```

The build is available at `dist/index.js` (or `dist.browser/index.iife.js` in the browser).

## Browser bundle
```html
<!-- Development (un-minified) -->
<script src="https://unpkg.com/@solana/spl-stake-pool@latest/dist.browser/index.iife.js"></script>

<!-- Production (minified) -->
<script src="https://unpkg.com/@solana/spl-stake-pool@latest/dist.browser/index.iife.min.js"></script>
```

## Test

```
npm test
```

## Usage

### JavaScript
```javascript
const solanaStakePool = require('@solana/spl-stake-pool');
console.log(solanaStakePool);
```

### ES6
```javascript
import * as solanaStakePool from '@solana/spl-stake-pool';
console.log(solanaStakePool);
```

### Browser bundle
```javascript
// `solanaStakePool` is provided in the global namespace by the script bundle.
console.log(solanaStakePool);
```