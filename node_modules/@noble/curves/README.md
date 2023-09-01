# noble-curves

Audited & minimal JS implementation of elliptic curve cryptography.

- 🔒 [**Audited**](#security) by an independent security firm
- 🔻 Tree-shaking-friendly: use only what's necessary, other code won't be included
- 🏎 Ultra-fast, hand-optimized for caveats of JS engines
- 🔍 Unique tests ensure correctness: property-based, cross-library and Wycheproof vectors, fuzzing 
- ➰ Short Weierstrass, Edwards, Montgomery curves
- ✍️ ECDSA, EdDSA, Schnorr, BLS signature schemes, ECDH key agreement
- #️⃣ Hash-to-curve
  for encoding or hashing an arbitrary string to an elliptic curve point
- 🧜‍♂️ Poseidon ZK-friendly hash

Check out [Upgrading](#upgrading) if you've previously used single-feature noble
packages. See [Resources](#resources) for articles and real-world software that uses curves.

### This library belongs to _noble_ crypto

> **noble-crypto** — high-security, easily auditable set of contained cryptographic libraries and tools.

- No dependencies, protection against supply chain attacks
- Auditable TypeScript / JS code
- Supported in all major browsers and stable node.js versions
- All releases are signed with PGP keys
- Check out [homepage](https://paulmillr.com/noble/) & all libraries:
  [curves](https://github.com/paulmillr/noble-curves)
  (4kb versions [secp256k1](https://github.com/paulmillr/noble-secp256k1),
  [ed25519](https://github.com/paulmillr/noble-ed25519)),
  [hashes](https://github.com/paulmillr/noble-hashes)

## Usage

Browser, deno and node.js are supported:

> npm install @noble/curves

For [Deno](https://deno.land), use it with
[npm specifier](https://deno.land/manual@v1.28.0/node/npm_specifiers).
In browser, you could also include the single file from
[GitHub's releases page](https://github.com/paulmillr/noble-curves/releases).

The library is tree-shaking-friendly and does NOT expose root entry point as
`import c from '@noble/curves'`. Instead, you need to import specific primitives.
This is done to ensure small size of your apps.

Package consists of two parts:

1. [Implementations](#implementations), utilizing one dependency [noble-hashes](https://github.com/paulmillr/noble-hashes),
   providing ready-to-use:
   - NIST curves secp256r1 / p256, secp384r1 / p384, secp521r1 / p521
   - SECG curve secp256k1
   - ed25519 / curve25519 / x25519 / ristretto255,
     edwards448 / curve448 / x448
     implementing
     [RFC7748](https://www.rfc-editor.org/rfc/rfc7748) /
     [RFC8032](https://www.rfc-editor.org/rfc/rfc8032) /
     [FIPS 186-5](https://csrc.nist.gov/publications/detail/fips/186/5/final) /
     [ZIP215](https://zips.z.cash/zip-0215) standards
   - pairing-friendly curves bls12-381, bn254
   - [pasta](https://electriccoin.co/blog/the-pasta-curves-for-halo-2-and-beyond/) curves
2. [Abstract](#abstract-api), zero-dependency EC algorithms

### Implementations

Each curve can be used in the following way:

```ts
import { secp256k1 } from '@noble/curves/secp256k1'; // ESM and Common.js
// import { secp256k1 } from 'npm:@noble/curves@1.2.0/secp256k1'; // Deno
const priv = secp256k1.utils.randomPrivateKey();
const pub = secp256k1.getPublicKey(priv);
const msg = new Uint8Array(32).fill(1);
const sig = secp256k1.sign(msg, priv);
const isValid = secp256k1.verify(sig, msg, pub) === true;

// hex strings are also supported besides Uint8Arrays:
const privHex = '46c930bc7bb4db7f55da20798697421b98c4175a52c630294d75a84b9c126236';
const pub2 = secp256k1.getPublicKey(privHex);
```

All curves:

```typescript
import { secp256k1, schnorr } from '@noble/curves/secp256k1';
import { ed25519, ed25519ph, ed25519ctx, x25519, RistrettoPoint } from '@noble/curves/ed25519';
import { ed448, ed448ph, ed448ctx, x448 } from '@noble/curves/ed448';
import { p256 } from '@noble/curves/p256';
import { p384 } from '@noble/curves/p384';
import { p521 } from '@noble/curves/p521';
import { pallas, vesta } from '@noble/curves/pasta';
import { bls12_381 } from '@noble/curves/bls12-381';
import { bn254 } from '@noble/curves/bn254';
import { jubjub } from '@noble/curves/jubjub';
```

Recovering public keys from weierstrass ECDSA signatures; using ECDH:

```ts
// extraEntropy https://moderncrypto.org/mail-archive/curves/2017/000925.html
const sigImprovedSecurity = secp256k1.sign(msg, priv, { extraEntropy: true });
sig.recoverPublicKey(msg) === pub; // public key recovery
const someonesPub = secp256k1.getPublicKey(secp256k1.utils.randomPrivateKey());
const shared = secp256k1.getSharedSecret(priv, someonesPub); // ECDH
```

Schnorr signatures over secp256k1 following
[BIP340](https://github.com/bitcoin/bips/blob/master/bip-0340.mediawiki):

```ts
import { schnorr } from '@noble/curves/secp256k1';
const priv = schnorr.utils.randomPrivateKey();
const pub = schnorr.getPublicKey(priv);
const msg = new TextEncoder().encode('hello');
const sig = schnorr.sign(msg, priv);
const isValid = schnorr.verify(sig, msg, pub);
```

ed25519 module has ed25519ctx / ed25519ph variants,
x25519 ECDH and [ristretto255](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-ristretto255-decaf448).

Default `verify` behavior follows [ZIP215](https://zips.z.cash/zip-0215) and
[can be used in consensus-critical applications](https://hdevalence.ca/blog/2020-10-04-its-25519am).
`zip215: false` option switches verification criteria to RFC8032 / FIPS 186-5.

```ts
import { ed25519 } from '@noble/curves/ed25519';
const priv = ed25519.utils.randomPrivateKey();
const pub = ed25519.getPublicKey(priv);
const msg = new TextEncoder().encode('hello');
const sig = ed25519.sign(msg, priv);
ed25519.verify(sig, msg, pub); // Default mode: follows ZIP215
ed25519.verify(sig, msg, pub, { zip215: false }); // RFC8032 / FIPS 186-5

// Variants from RFC8032: with context, prehashed
import { ed25519ctx, ed25519ph } from '@noble/curves/ed25519';

// ECDH using curve25519 aka x25519
import { x25519 } from '@noble/curves/ed25519';
const priv = 'a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4';
const pub = 'e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c';
x25519.getSharedSecret(priv, pub) === x25519.scalarMult(priv, pub); // aliases
x25519.getPublicKey(priv) === x25519.scalarMultBase(priv);

// hash-to-curve
import { hashToCurve, encodeToCurve } from '@noble/curves/ed25519';

import { RistrettoPoint } from '@noble/curves/ed25519';
const rp = RistrettoPoint.fromHex(
  '6a493210f7499cd17fecb510ae0cea23a110e8d5b901f8acadd3095c73a3b919'
);
RistrettoPoint.hashToCurve('Ristretto is traditionally a short shot of espresso coffee');
// also has add(), equals(), multiply(), toRawBytes() methods
```

ed448 is similar:

```ts
import { ed448, ed448ph, ed448ctx, x448 } from '@noble/curves/ed448';
import { hashToCurve, encodeToCurve } from '@noble/curves/ed448';
ed448.getPublicKey(ed448.utils.randomPrivateKey());
```

Every curve has `CURVE` object that contains its parameters, field, and others:

```ts
import { secp256k1 } from '@noble/curves/secp256k1'; // ESM and Common.js
console.log(secp256k1.CURVE.p, secp256k1.CURVE.n, secp256k1.CURVE.a, secp256k1.CURVE.b);
```

## Abstract API

Abstract API allows to define custom curves. All arithmetics is done with JS
bigints over finite fields, which is defined from `modular` sub-module. For
scalar multiplication, we use
[precomputed tables with w-ary non-adjacent form (wNAF)](https://paulmillr.com/posts/noble-secp256k1-fast-ecc/).
Precomputes are enabled for weierstrass and edwards BASE points of a curve. You
could precompute any other point (e.g. for ECDH) using `utils.precompute()`
method: check out examples.

There are following zero-dependency algorithms:

- [abstract/weierstrass: Short Weierstrass curve](#abstractweierstrass-short-weierstrass-curve)
- [abstract/edwards: Twisted Edwards curve](#abstractedwards-twisted-edwards-curve)
- [abstract/montgomery: Montgomery curve](#abstractmontgomery-montgomery-curve)
- [abstract/bls: Barreto-Lynn-Scott curves](#abstractbls-barreto-lynn-scott-curves)
- [abstract/hash-to-curve: Hashing strings to curve points](#abstracthash-to-curve-hashing-strings-to-curve-points)
- [abstract/poseidon: Poseidon hash](#abstractposeidon-poseidon-hash)
- [abstract/modular: Modular arithmetics utilities](#abstractmodular-modular-arithmetics-utilities)
- [abstract/utils: General utilities](#abstractutils-general-utilities)

### abstract/weierstrass: Short Weierstrass curve

```ts
import { weierstrass } from '@noble/curves/abstract/weierstrass';
import { Field } from '@noble/curves/abstract/modular'; // finite field for mod arithmetics
import { sha256 } from '@noble/hashes/sha256'; // 3rd-party sha256() of type utils.CHash
import { hmac } from '@noble/hashes/hmac'; // 3rd-party hmac() that will accept sha256()
import { concatBytes, randomBytes } from '@noble/hashes/utils'; // 3rd-party utilities
const secq256k1 = weierstrass({
  // secq256k1: cycle of secp256k1 with Fp/N flipped.
  // https://personaelabs.org/posts/spartan-ecdsa
  // https://zcash.github.io/halo2/background/curves.html#cycles-of-curves
  a: 0n,
  b: 7n,
  Fp: Field(2n ** 256n - 432420386565659656852420866394968145599n),
  n: 2n ** 256n - 2n ** 32n - 2n ** 9n - 2n ** 8n - 2n ** 7n - 2n ** 6n - 2n ** 4n - 1n,
  Gx: 55066263022277343669578718895168534326250603453777594175500187360389116729240n,
  Gy: 32670510020758816978083085130507043184471273380659243275938904335757337482424n,
  hash: sha256,
  hmac: (key: Uint8Array, ...msgs: Uint8Array[]) => hmac(sha256, key, concatBytes(...msgs)),
  randomBytes,
});

// Replace weierstrass with weierstrassPoints if you don't need ECDSA, hash, hmac, randomBytes
```

Short Weierstrass curve's formula is `y² = x³ + ax + b`. `weierstrass`
expects arguments `a`, `b`, field `Fp`, curve order `n`, cofactor `h`
and coordinates `Gx`, `Gy` of generator point.

**`k` generation** is done deterministically, following
[RFC6979](https://www.rfc-editor.org/rfc/rfc6979). For this you will need
`hmac` & `hash`, which in our implementations is provided by noble-hashes. If
you're using different hashing library, make sure to wrap it in the following interface:

```ts
type CHash = {
  (message: Uint8Array): Uint8Array;
  blockLen: number;
  outputLen: number;
  create(): any;
};
```

**Weierstrass points:**

1. Exported as `ProjectivePoint`
2. Represented in projective (homogeneous) coordinates: (x, y, z) ∋ (x=x/z, y=y/z)
3. Use complete exception-free formulas for addition and doubling
4. Can be decoded/encoded from/to Uint8Array / hex strings using
   `ProjectivePoint.fromHex` and `ProjectivePoint#toRawBytes()`
5. Have `assertValidity()` which checks for being on-curve
6. Have `toAffine()` and `x` / `y` getters which convert to 2d xy affine coordinates

```ts
// `weierstrassPoints()` returns `CURVE` and `ProjectivePoint`
// `weierstrass()` returns `CurveFn`
type SignOpts = { lowS?: boolean; prehash?: boolean; extraEntropy: boolean | Uint8Array };
type CurveFn = {
  CURVE: ReturnType<typeof validateOpts>;
  getPublicKey: (privateKey: PrivKey, isCompressed?: boolean) => Uint8Array;
  getSharedSecret: (privateA: PrivKey, publicB: Hex, isCompressed?: boolean) => Uint8Array;
  sign: (msgHash: Hex, privKey: PrivKey, opts?: SignOpts) => SignatureType;
  verify: (
    signature: Hex | SignatureType,
    msgHash: Hex,
    publicKey: Hex,
    opts?: { lowS?: boolean; prehash?: boolean }
  ) => boolean;
  ProjectivePoint: ProjectivePointConstructor;
  Signature: SignatureConstructor;
  utils: {
    normPrivateKeyToScalar: (key: PrivKey) => bigint;
    isValidPrivateKey(key: PrivKey): boolean;
    randomPrivateKey: () => Uint8Array;
    precompute: (windowSize?: number, point?: ProjPointType<bigint>) => ProjPointType<bigint>;
  };
};

// T is usually bigint, but can be something else like complex numbers in BLS curves
interface ProjPointType<T> extends Group<ProjPointType<T>> {
  readonly px: T;
  readonly py: T;
  readonly pz: T;
  get x(): bigint;
  get y(): bigint;
  multiply(scalar: bigint): ProjPointType<T>;
  multiplyUnsafe(scalar: bigint): ProjPointType<T>;
  multiplyAndAddUnsafe(Q: ProjPointType<T>, a: bigint, b: bigint): ProjPointType<T> | undefined;
  toAffine(iz?: T): AffinePoint<T>;
  isTorsionFree(): boolean;
  clearCofactor(): ProjPointType<T>;
  assertValidity(): void;
  hasEvenY(): boolean;
  toRawBytes(isCompressed?: boolean): Uint8Array;
  toHex(isCompressed?: boolean): string;
}
// Static methods for 3d XYZ points
interface ProjConstructor<T> extends GroupConstructor<ProjPointType<T>> {
  new (x: T, y: T, z: T): ProjPointType<T>;
  fromAffine(p: AffinePoint<T>): ProjPointType<T>;
  fromHex(hex: Hex): ProjPointType<T>;
  fromPrivateKey(privateKey: PrivKey): ProjPointType<T>;
}
```

**ECDSA signatures** are represented by `Signature` instances and can be
described by the interface:

```ts
interface SignatureType {
  readonly r: bigint;
  readonly s: bigint;
  readonly recovery?: number;
  assertValidity(): void;
  addRecoveryBit(recovery: number): SignatureType;
  hasHighS(): boolean;
  normalizeS(): SignatureType;
  recoverPublicKey(msgHash: Hex): ProjPointType<bigint>;
  toCompactRawBytes(): Uint8Array;
  toCompactHex(): string;
  // DER-encoded
  toDERRawBytes(): Uint8Array;
  toDERHex(): string;
}
type SignatureConstructor = {
  new (r: bigint, s: bigint): SignatureType;
  fromCompact(hex: Hex): SignatureType;
  fromDER(hex: Hex): SignatureType;
};
```

More examples:

```typescript
// All curves expose same generic interface.
const priv = secq256k1.utils.randomPrivateKey();
secq256k1.getPublicKey(priv); // Convert private key to public.
const sig = secq256k1.sign(msg, priv); // Sign msg with private key.
secq256k1.verify(sig, msg, priv); // Verify if sig is correct.

const Point = secq256k1.ProjectivePoint;
const point = Point.BASE; // Elliptic curve Point class and BASE point static var.
point.add(point).equals(point.double()); // add(), equals(), double() methods
point.subtract(point).equals(Point.ZERO); // subtract() method, ZERO static var
point.negate(); // Flips point over x/y coordinate.
point.multiply(31415n); // Multiplication of Point by scalar.

point.assertValidity(); // Checks for being on-curve
point.toAffine(); // Converts to 2d affine xy coordinates

secq256k1.CURVE.n;
secq256k1.CURVE.p;
secq256k1.CURVE.Fp.mod();
secq256k1.CURVE.hash();

// precomputes
const fast = secq256k1.utils.precompute(8, Point.fromHex(someonesPubKey));
fast.multiply(privKey); // much faster ECDH now
```

### abstract/edwards: Twisted Edwards curve

```ts
import { twistedEdwards } from '@noble/curves/abstract/edwards';
import { Field } from '@noble/curves/abstract/modular';
import { sha512 } from '@noble/hashes/sha512';
import { randomBytes } from '@noble/hashes/utils';

const Fp = Field(2n ** 255n - 19n);
const ed25519 = twistedEdwards({
  a: Fp.create(-1n),
  d: Fp.div(-121665n, 121666n), // -121665n/121666n mod p
  Fp: Fp,
  n: 2n ** 252n + 27742317777372353535851937790883648493n,
  h: 8n,
  Gx: 15112221349535400772501151409588531511454012693041857206046113283949847762202n,
  Gy: 46316835694926478169428394003475163141307993866256225615783033603165251855960n,
  hash: sha512,
  randomBytes,
  adjustScalarBytes(bytes) {
    // optional; but mandatory in ed25519
    bytes[0] &= 248;
    bytes[31] &= 127;
    bytes[31] |= 64;
    return bytes;
  },
} as const);
```

Twisted Edwards curve's formula is `ax² + y² = 1 + dx²y²`. You must specify `a`, `d`, field `Fp`, order `n`, cofactor `h`
and coordinates `Gx`, `Gy` of generator point.

For EdDSA signatures, `hash` param required. `adjustScalarBytes` which instructs how to change private scalars could be specified.

**Edwards points:**

1. Exported as `ExtendedPoint`
2. Represented in extended coordinates: (x, y, z, t) ∋ (x=x/z, y=y/z)
3. Use complete exception-free formulas for addition and doubling
4. Can be decoded/encoded from/to Uint8Array / hex strings using `ExtendedPoint.fromHex` and `ExtendedPoint#toRawBytes()`
5. Have `assertValidity()` which checks for being on-curve
6. Have `toAffine()` and `x` / `y` getters which convert to 2d xy affine coordinates
7. Have `isTorsionFree()`, `clearCofactor()` and `isSmallOrder()` utilities to handle torsions

```ts
// `twistedEdwards()` returns `CurveFn` of following type:
type CurveFn = {
  CURVE: ReturnType<typeof validateOpts>;
  getPublicKey: (privateKey: Hex) => Uint8Array;
  sign: (message: Hex, privateKey: Hex, context?: Hex) => Uint8Array;
  verify: (sig: SigType, message: Hex, publicKey: Hex, context?: Hex) => boolean;
  ExtendedPoint: ExtPointConstructor;
  utils: {
    randomPrivateKey: () => Uint8Array;
    getExtendedPublicKey: (key: PrivKey) => {
      head: Uint8Array;
      prefix: Uint8Array;
      scalar: bigint;
      point: PointType;
      pointBytes: Uint8Array;
    };
  };
};

interface ExtPointType extends Group<ExtPointType> {
  readonly ex: bigint;
  readonly ey: bigint;
  readonly ez: bigint;
  readonly et: bigint;
  get x(): bigint;
  get y(): bigint;
  assertValidity(): void;
  multiply(scalar: bigint): ExtPointType;
  multiplyUnsafe(scalar: bigint): ExtPointType;
  isSmallOrder(): boolean;
  isTorsionFree(): boolean;
  clearCofactor(): ExtPointType;
  toAffine(iz?: bigint): AffinePoint<bigint>;
  toRawBytes(isCompressed?: boolean): Uint8Array;
  toHex(isCompressed?: boolean): string;
}
// Static methods of Extended Point with coordinates in X, Y, Z, T
interface ExtPointConstructor extends GroupConstructor<ExtPointType> {
  new (x: bigint, y: bigint, z: bigint, t: bigint): ExtPointType;
  fromAffine(p: AffinePoint<bigint>): ExtPointType;
  fromHex(hex: Hex): ExtPointType;
  fromPrivateKey(privateKey: Hex): ExtPointType;
}
```

### abstract/montgomery: Montgomery curve

```typescript
import { montgomery } from '@noble/curves/abstract/montgomery';
import { Field } from '@noble/curves/abstract/modular';

const x25519 = montgomery({
  a: 486662n,
  Gu: 9n,
  Fp: Field(2n ** 255n - 19n),
  montgomeryBits: 255,
  nByteLength: 32,
  // Optional param
  adjustScalarBytes(bytes) {
    bytes[0] &= 248;
    bytes[31] &= 127;
    bytes[31] |= 64;
    return bytes;
  },
});
```

The module contains methods for x-only ECDH on Curve25519 / Curve448 from RFC7748.
Proper Elliptic Curve Points are not implemented yet.

You must specify curve params `Fp`, `a`, `Gu` coordinate of u, `montgomeryBits` and `nByteLength`.

### abstract/bls: Barreto-Lynn-Scott curves

The module abstracts BLS (Barreto-Lynn-Scott) pairing-friendly elliptic curve construction.
They allow to construct [zk-SNARKs](https://z.cash/technology/zksnarks/) and
use aggregated, batch-verifiable
[threshold signatures](https://medium.com/snigirev.stepan/bls-signatures-better-than-schnorr-5a7fe30ea716),
using Boneh-Lynn-Shacham signature scheme.

Main methods and properties are:

- `getPublicKey(privateKey)`
- `sign(message, privateKey)`
- `verify(signature, message, publicKey)`
- `aggregatePublicKeys(publicKeys)`
- `aggregateSignatures(signatures)`
- `G1` and `G2` curves containing `CURVE` and `ProjectivePoint`
- `Signature` property with `fromHex`, `toHex` methods
- `fields` containing `Fp`, `Fp2`, `Fp6`, `Fp12`, `Fr`

Right now we only implement BLS12-381 (compatible with ETH and others),
but in theory defining BLS12-377, BLS24 should be straightforward. An example:

```ts
import { bls12_381 as bls } from '@noble/curves/bls12-381';
const privateKey = '67d53f170b908cabb9eb326c3c337762d59289a8fec79f7bc9254b584b73265c';
const message = '64726e3da8';
const publicKey = bls.getPublicKey(privateKey);
const signature = bls.sign(message, privateKey);
const isValid = bls.verify(signature, message, publicKey);
console.log({ publicKey, signature, isValid });

// Sign 1 msg with 3 keys
const privateKeys = [
  '18f020b98eb798752a50ed0563b079c125b0db5dd0b1060d1c1b47d4a193e1e4',
  'ed69a8c50cf8c9836be3b67c7eeff416612d45ba39a5c099d48fa668bf558c9c',
  '16ae669f3be7a2121e17d0c68c05a8f3d6bef21ec0f2315f1d7aec12484e4cf5',
];
const messages = ['d2', '0d98', '05caf3'];
const publicKeys = privateKeys.map(bls.getPublicKey);
const signatures2 = privateKeys.map((p) => bls.sign(message, p));
const aggPubKey2 = bls.aggregatePublicKeys(publicKeys);
const aggSignature2 = bls.aggregateSignatures(signatures2);
const isValid2 = bls.verify(aggSignature2, message, aggPubKey2);
console.log({ signatures2, aggSignature2, isValid2 });

// Sign 3 msgs with 3 keys
const signatures3 = privateKeys.map((p, i) => bls.sign(messages[i], p));
const aggSignature3 = bls.aggregateSignatures(signatures3);
const isValid3 = bls.verifyBatch(aggSignature3, messages, publicKeys);
console.log({ publicKeys, signatures3, aggSignature3, isValid3 });

// bls.pairing(PointG1, PointG2) // pairings
// bls.G1.ProjectivePoint.BASE, bls.G2.ProjectivePoint.BASE
// bls.fields.Fp, bls.fields.Fp2, bls.fields.Fp12, bls.fields.Fr

// hash-to-curve examples can be seen below
```

Full types:

```ts
getPublicKey: (privateKey: PrivKey) => Uint8Array;
sign: {
  (message: Hex, privateKey: PrivKey): Uint8Array;
  (message: ProjPointType<Fp2>, privateKey: PrivKey): ProjPointType<Fp2>;
};
verify: (
  signature: Hex | ProjPointType<Fp2>,
  message: Hex | ProjPointType<Fp2>,
  publicKey: Hex | ProjPointType<Fp>
) => boolean;
verifyBatch: (
  signature: Hex | ProjPointType<Fp2>,
  messages: (Hex | ProjPointType<Fp2>)[],
  publicKeys: (Hex | ProjPointType<Fp>)[]
) => boolean;
aggregatePublicKeys: {
  (publicKeys: Hex[]): Uint8Array;
  (publicKeys: ProjPointType<Fp>[]): ProjPointType<Fp>;
};
aggregateSignatures: {
  (signatures: Hex[]): Uint8Array;
  (signatures: ProjPointType<Fp2>[]): ProjPointType<Fp2>;
};
millerLoop: (ell: [Fp2, Fp2, Fp2][], g1: [Fp, Fp]) => Fp12;
pairing: (P: ProjPointType<Fp>, Q: ProjPointType<Fp2>, withFinalExponent?: boolean) => Fp12;
G1: CurvePointsRes<Fp> & ReturnType<typeof htf.createHasher<Fp>>;
G2: CurvePointsRes<Fp2> & ReturnType<typeof htf.createHasher<Fp2>>;  
Signature: SignatureCoder<Fp2>;
params: {
  x: bigint;
  r: bigint;
  G1b: bigint;
  G2b: Fp2;
};
fields: {
  Fp: IField<Fp>;
  Fp2: IField<Fp2>;
  Fp6: IField<Fp6>;
  Fp12: IField<Fp12>;
  Fr: IField<bigint>;  
};
utils: {
  randomPrivateKey: () => Uint8Array;
  calcPairingPrecomputes: (p: AffinePoint<Fp2>) => [Fp2, Fp2, Fp2][];
};
```

### abstract/hash-to-curve: Hashing strings to curve points

The module allows to hash arbitrary strings to elliptic curve points. Implements [hash-to-curve v16](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-16).

Every curve has exported `hashToCurve` and `encodeToCurve` methods. You should always prefer `hashToCurve` for security:

```ts
import { hashToCurve, encodeToCurve } from '@noble/curves/secp256k1';
import { randomBytes } from '@noble/hashes/utils';
hashToCurve('0102abcd');
console.log(hashToCurve(randomBytes()));
console.log(encodeToCurve(randomBytes()));

import { bls12_381 } from '@noble/curves/bls12-381';
bls12_381.G1.hashToCurve(randomBytes(), { DST: 'another' });
bls12_381.G2.hashToCurve(randomBytes(), { DST: 'custom' });
```

If you need low-level methods from spec:

`expand_message_xmd` [(spec)](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-5.4.1) produces a uniformly random byte string using a cryptographic hash function H that outputs b bits.

Hash must conform to `CHash` interface (see [weierstrass section](#abstractweierstrass-short-weierstrass-curve)).

```ts
function expand_message_xmd(
  msg: Uint8Array,
  DST: Uint8Array,
  lenInBytes: number,
  H: CHash
): Uint8Array;
function expand_message_xof(
  msg: Uint8Array,
  DST: Uint8Array,
  lenInBytes: number,
  k: number,
  H: CHash
): Uint8Array;
```

`hash_to_field(msg, count, options)`
[(spec)](https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-5.3)
hashes arbitrary-length byte strings to a list of one or more elements of a finite field F.

```ts
/**
 * * `DST` is a domain separation tag, defined in section 2.2.5
 * * `p` characteristic of F, where F is a finite field of characteristic p and order q = p^m
 * * `m` is extension degree (1 for prime fields)
 * * `k` is the target security target in bits (e.g. 128), from section 5.1
 * * `expand` is `xmd` (SHA2, SHA3, BLAKE) or `xof` (SHAKE, BLAKE-XOF)
 * * `hash` conforming to `utils.CHash` interface, with `outputLen` / `blockLen` props
 */
type UnicodeOrBytes = string | Uint8Array;
type Opts = {
    DST: UnicodeOrBytes;
    p: bigint;
    m: number;
    k: number;
    expand?: 'xmd' | 'xof';
    hash: CHash;
};

/**
 * Hashes arbitrary-length byte strings to a list of one or more elements of a finite field F
 * https://datatracker.ietf.org/doc/html/draft-irtf-cfrg-hash-to-curve-11#section-5.3
 * @param msg a byte string containing the message to hash
 * @param count the number of elements of F to output
 * @param options `{DST: string, p: bigint, m: number, k: number, expand: 'xmd' | 'xof', hash: H}`, see above
 * @returns [u_0, ..., u_(count - 1)], a list of field elements.
 */
function hash_to_field(msg: Uint8Array, count: number, options: Opts): bigint[][];
```

### abstract/poseidon: Poseidon hash

Implements [Poseidon](https://www.poseidon-hash.info) ZK-friendly hash.

There are many poseidon variants with different constants.
We don't provide them: you should construct them manually.
Check out [micro-starknet](https://github.com/paulmillr/micro-starknet) package for a proper example.

```ts
import { poseidon } from '@noble/curves/abstract/poseidon';

type PoseidonOpts = {
  Fp: Field<bigint>;
  t: number;
  roundsFull: number;
  roundsPartial: number;
  sboxPower?: number;
  reversePartialPowIdx?: boolean;
  mds: bigint[][];
  roundConstants: bigint[][];
};
const instance = poseidon(opts: PoseidonOpts);
```

### abstract/modular: Modular arithmetics utilities

```ts
import * as mod from '@noble/curves/abstract/modular';
const fp = mod.Field(2n ** 255n - 19n); // Finite field over 2^255-19
fp.mul(591n, 932n); // multiplication
fp.pow(481n, 11024858120n); // exponentiation
fp.div(5n, 17n); // division: 5/17 mod 2^255-19 == 5 * invert(17)
fp.sqrt(21n); // square root

// Generic non-FP utils are also available
mod.mod(21n, 10n); // 21 mod 10 == 1n; fixed version of 21 % 10
mod.invert(17n, 10n); // invert(17) mod 10; modular multiplicative inverse
mod.invertBatch([1n, 2n, 4n], 21n); // => [1n, 11n, 16n] in one inversion
```

#### Creating private keys from hashes

Suppose you have `sha256(something)` (e.g. from HMAC) and you want to make a private key from it.
Even though p256 or secp256k1 may have 32-byte private keys,
and sha256 output is also 32-byte, you can't just use it and reduce it modulo `CURVE.n`.

Doing so will make the result key [biased](https://research.kudelskisecurity.com/2020/07/28/the-definitive-guide-to-modulo-bias-and-how-to-avoid-it/).

To avoid the bias, we implement FIPS 186 B.4.1, which allows to take arbitrary
byte array and produce valid scalars / private keys with bias being neglible.

Use [hash-to-curve](#abstracthash-to-curve-hashing-strings-to-curve-points) if you need
hashing to **public keys**; the function in the module instead operates on **private keys**.

```ts
import { p256 } from '@noble/curves/p256';
import { sha256 } from '@noble/hashes/sha256';
import { hkdf } from '@noble/hashes/hkdf';
const someKey = new Uint8Array(32).fill(2); // Needs to actually be random, not .fill(2)
const derived = hkdf(sha256, someKey, undefined, 'application', 40); // 40 bytes
const validPrivateKey = mod.hashToPrivateScalar(derived, p256.CURVE.n);
```

### abstract/utils: General utilities

```ts
import * as utils from '@noble/curves/abstract/utils';

utils.bytesToHex(Uint8Array.from([0xde, 0xad, 0xbe, 0xef]));
utils.hexToBytes('deadbeef');
utils.numberToHexUnpadded(123n);
utils.hexToNumber();

utils.bytesToNumberBE(Uint8Array.from([0xde, 0xad, 0xbe, 0xef]));
utils.bytesToNumberLE(Uint8Array.from([0xde, 0xad, 0xbe, 0xef]));
utils.numberToBytesBE(123n, 32);
utils.numberToBytesLE(123n, 64);

utils.concatBytes(Uint8Array.from([0xde, 0xad]), Uint8Array.from([0xbe, 0xef]));
utils.nLength(255n);
utils.equalBytes(Uint8Array.from([0xde]), Uint8Array.from([0xde]));
```

## Security

1. The library has been audited during Jan-Feb 2023 by an independent security firm [Trail of Bits](https://www.trailofbits.com):
[PDF](https://github.com/trailofbits/publications/blob/master/reviews/2023-01-ryanshea-noblecurveslibrary-securityreview.pdf).
The audit has been funded by Ryan Shea. Audit scope was abstract modules `curve`, `hash-to-curve`, `modular`, `poseidon`, `utils`, `weierstrass`, and top-level modules `_shortw_utils` and `secp256k1`. See [changes since audit](https://github.com/paulmillr/noble-curves/compare/0.7.3..main).
2. The library has been fuzzed by [Guido Vranken's cryptofuzz](https://github.com/guidovranken/cryptofuzz). You can run the fuzzer by yourself to check it.
3. [Timing attack](https://en.wikipedia.org/wiki/Timing_attack) considerations: _JIT-compiler_ and _Garbage Collector_ make "constant time" extremely hard to achieve in a scripting language. Which means _any other JS library can't have constant-timeness_. Even statically typed Rust, a language without GC, [makes it harder to achieve constant-time](https://www.chosenplaintext.ca/open-source/rust-timing-shield/security) for some cases. If your goal is absolute security, don't use any JS lib — including bindings to native ones. Use low-level libraries & languages. Nonetheless we're targetting algorithmic constant time.

We consider infrastructure attacks like rogue NPM modules very important; that's why it's crucial to minimize the amount of 3rd-party dependencies & native bindings. If your app uses 500 dependencies, any dep could get hacked and you'll be downloading malware with every `npm install`. Our goal is to minimize this attack vector. As for devDependencies used by the library:

- `@scure` base, bip32, bip39 (used in tests), micro-bmark (benchmark), micro-should (testing) are developed by us
  and follow the same practices such as: minimal library size, auditability, signed releases
- prettier (linter), fast-check (property-based testing),
  typescript versions are locked and rarely updated. Every update is checked with `npm-diff`.
  The packages are big, which makes it hard to audit their source code thoroughly and fully.
- They are only used if you clone the git repo and want to add some feature to it. End-users won't use them.

## Speed

Benchmark results on Apple M2 with node v19:

```
secp256k1
init x 58 ops/sec @ 17ms/op
getPublicKey x 5,640 ops/sec @ 177μs/op
sign x 4,471 ops/sec @ 223μs/op
verify x 780 ops/sec @ 1ms/op
getSharedSecret x 465 ops/sec @ 2ms/op
recoverPublicKey x 740 ops/sec @ 1ms/op
schnorr.sign x 597 ops/sec @ 1ms/op
schnorr.verify x 775 ops/sec @ 1ms/op

P256
init x 31 ops/sec @ 31ms/op
getPublicKey x 5,607 ops/sec @ 178μs/op
sign x 4,583 ops/sec @ 218μs/op
verify x 540 ops/sec @ 1ms/op

P384
init x 15 ops/sec @ 63ms/op
getPublicKey x 2,622 ops/sec @ 381μs/op
sign x 2,106 ops/sec @ 474μs/op
verify x 222 ops/sec @ 4ms/op

P521
init x 8 ops/sec @ 119ms/op
getPublicKey x 1,371 ops/sec @ 729μs/op
sign x 1,164 ops/sec @ 858μs/op
verify x 118 ops/sec @ 8ms/op

ed25519
init x 47 ops/sec @ 20ms/op
getPublicKey x 9,414 ops/sec @ 106μs/op
sign x 4,516 ops/sec @ 221μs/op
verify x 912 ops/sec @ 1ms/op

ed448
init x 17 ops/sec @ 56ms/op
getPublicKey x 3,363 ops/sec @ 297μs/op
sign x 1,615 ops/sec @ 619μs/op
verify x 319 ops/sec @ 3ms/op

ecdh
├─x25519 x 1,337 ops/sec @ 747μs/op
├─secp256k1 x 461 ops/sec @ 2ms/op
├─P256 x 441 ops/sec @ 2ms/op
├─P384 x 179 ops/sec @ 5ms/op
├─P521 x 93 ops/sec @ 10ms/op
└─x448 x 496 ops/sec @ 2ms/op

bls12-381
init x 32 ops/sec @ 30ms/op
getPublicKey 1-bit x 858 ops/sec @ 1ms/op
getPublicKey x 858 ops/sec @ 1ms/op
sign x 49 ops/sec @ 20ms/op
verify x 34 ops/sec @ 28ms/op
pairing x 94 ops/sec @ 10ms/op
aggregatePublicKeys/8 x 116 ops/sec @ 8ms/op
aggregatePublicKeys/32 x 31 ops/sec @ 31ms/op
aggregatePublicKeys/128 x 7 ops/sec @ 125ms/op
aggregateSignatures/8 x 45 ops/sec @ 22ms/op
aggregateSignatures/32 x 11 ops/sec @ 84ms/op
aggregateSignatures/128 x 3 ops/sec @ 332ms/opp

hash-to-curve
hash_to_field x 850,340 ops/sec @ 1μs/op
secp256k1 x 2,143 ops/sec @ 466μs/op
P256 x 3,861 ops/sec @ 258μs/op
P384 x 1,526 ops/sec @ 655μs/op
P521 x 748 ops/sec @ 1ms/op
ed25519 x 2,772 ops/sec @ 360μs/op
ed448 x 1,146 ops/sec @ 871μs/op
```

## Contributing & testing

1. Clone the repository
2. `npm install` to install build dependencies like TypeScript
3. `npm run build` to compile TypeScript code
4. `npm run test` will execute all main tests

## Upgrading

Previously, the library was split into single-feature packages
noble-secp256k1 and noble-ed25519. curves can be thought as a continuation of their
original work. The libraries now changed their direction towards providing
minimal 4kb implementations of cryptography and are not as feature-complete.

Upgrading from @noble/secp256k1 2.0 or @noble/ed25519 2.0: no changes, libraries are compatible.

Upgrading from [@noble/secp256k1](https://github.com/paulmillr/noble-secp256k1) 1.7:

- `getPublicKey`
    - now produce 33-byte compressed signatures by default
    - to use old behavior, which produced 65-byte uncompressed keys, set
      argument `isCompressed` to `false`: `getPublicKey(priv, false)`
- `sign`
    - is now sync; use `signAsync` for async version
    - now returns `Signature` instance with `{ r, s, recovery }` properties
    - `canonical` option was renamed to `lowS`
    - `recovered` option has been removed because recovery bit is always returned now
    - `der` option has been removed. There are 2 options:
        1. Use compact encoding: `fromCompact`, `toCompactRawBytes`, `toCompactHex`.
           Compact encoding is simply a concatenation of 32-byte r and 32-byte s.
        2. If you must use DER encoding, switch to noble-curves (see above).
- `verify`
    - `strict` option was renamed to `lowS`
- `getSharedSecret`
    - now produce 33-byte compressed signatures by default
    - to use old behavior, which produced 65-byte uncompressed keys, set
      argument `isCompressed` to `false`: `getSharedSecret(a, b, false)`
- `recoverPublicKey(msg, sig, rec)` was changed to `sig.recoverPublicKey(msg)`
- `number` type for private keys have been removed: use `bigint` instead
- `Point` (2d xy) has been changed to `ProjectivePoint` (3d xyz)
- `utils` were split into `utils` (same api as in noble-curves) and
  `etc` (`hmacSha256Sync` and others)

Upgrading from [@noble/ed25519](https://github.com/paulmillr/noble-ed25519) 1.7:

- Methods are now sync by default
- `bigint` is no longer allowed in `getPublicKey`, `sign`, `verify`. Reason: ed25519 is LE, can lead to bugs
- `Point` (2d xy) has been changed to `ExtendedPoint` (xyzt)
- `Signature` was removed: just use raw bytes or hex now
- `utils` were split into `utils` (same api as in noble-curves) and
  `etc` (`sha512Sync` and others)
- `getSharedSecret` was moved to `x25519` module

Upgrading from [@noble/bls12-381](https://github.com/paulmillr/noble-bls12-381):

- Methods and classes were renamed:
    - PointG1 -> G1.Point, PointG2 -> G2.Point
    - PointG2.fromSignature -> Signature.decode, PointG2.toSignature ->  Signature.encode
- Fp2 ORDER was corrected

## Resources

Useful articles about the library or its primitives:

- [Learning fast elliptic-curve cryptography](https://paulmillr.com/posts/noble-secp256k1-fast-ecc/)
- Pairings and BLS
    - [BLS12-381 for the rest of us](https://hackmd.io/@benjaminion/bls12-381)
    - [Key concepts of pairings](https://medium.com/@alonmuroch_65570/bls-signatures-part-2-key-concepts-of-pairings-27a8a9533d0c)
    - Pairing over bls12-381:
      [part 1](https://research.nccgroup.com/2020/07/06/pairing-over-bls12-381-part-1-fields/),
      [part 2](https://research.nccgroup.com/2020/07/13/pairing-over-bls12-381-part-2-curves/),
      [part 3](https://research.nccgroup.com/2020/08/13/pairing-over-bls12-381-part-3-pairing/)
    - [Estimating the bit security of pairing-friendly curves](https://research.nccgroup.com/2022/02/03/estimating-the-bit-security-of-pairing-friendly-curves/)

Real-world software that uses curves:

- [Elliptic Curve Calculator](https://paulmillr.com/noble) online demo: add / multiply points, sign messages
- Signers for web3 projects:
  [btc-signer](https://github.com/paulmillr/scure-btc-signer), [eth-signer](https://github.com/paulmillr/micro-eth-signer),
  [sol-signer](https://github.com/paulmillr/micro-sol-signer) for Solana
- [scure-bip32](https://github.com/paulmillr/scure-bip32) and separate [bip32](https://github.com/bitcoinjs/bip32) HDkey libraries
- [ed25519-keygen](https://github.com/paulmillr/ed25519-keygen) SSH, PGP, TOR key generation
- [micro-starknet](https://github.com/paulmillr/micro-starknet) stark-friendly elliptic curve algorithms.
- BLS threshold sigs demo [genthresh.com](https://genthresh.com)
- BLS BBS signatures [github.com/Wind4Greg/BBS-Draft-Checks](https://github.com/Wind4Greg/BBS-Draft-Checks) following [draft-irtf-cfrg-bbs-signatures-latest](https://identity.foundation/bbs-signature/draft-irtf-cfrg-bbs-signatures.html)
- [KZG trusted setup ceremony](https://github.com/dsrvlabs/czg-keremony)

## License

The MIT License (MIT)

Copyright (c) 2022 Paul Miller [(https://paulmillr.com)](https://paulmillr.com)

See LICENSE file.
