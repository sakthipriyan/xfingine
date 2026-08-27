# xfingine

WebAssembly bindings for [**Xfingine**](https://github.com/sakthipriyan/xfingine) —
pure computation engines for personal finance planning.

Data in, arithmetic, data out. No UI, no network, no clock. The engines run
entirely in your browser.

```bash
npm i xfingine
```

## Usage

```js
import init, { compute_emi, version } from 'xfingine';

await init();

const result = compute_emi({
  mode: 'emi',            // 'emi' | 'tenure' | 'loanAmount'
  loanAmount: 5_000_000,
  months: 240,
  interestRate: 9,        // percent per year
  inflationRate: 6,       // percent per year
  start: '2026-01',       // optional, enables dates + per-year breakdown
});

result.emi;                   // 44986   — the monthly payment
result.totals.nominalPaid;    // 10796818 — rupees actually debited
result.totals.realPaid;       // 6391327  — the same, in 2026 rupees
result.schedule;              // 240 rows, month by month
result.years;                 // 20 rows, one per calendar year
```

Every function has a `_json` twin that takes and returns a JSON string, for when
you already have one in hand:

```js
const json = compute_emi_json(JSON.stringify(request));
```

Invalid input throws a readable string:

```js
try {
  compute_emi({ mode: 'tenure', loanAmount: 5_000_000, emi: 1000, interestRate: 12 });
} catch (e) {
  // "monthly payment of 1000.00 does not cover the first month's interest
  //  of 50000.00; the loan would never be repaid"
}
```

## Conventions

- Money comes back as whole rupees (integers).
- Rates are percentages, not fractions: `9` means 9%.
- All keys are `camelCase`.
- Omit `start` and the engine reads no calendar at all — `schedule` rows carry
  no dates and `years` is empty.

Full documentation: [github.com/sakthipriyan/xfingine](https://github.com/sakthipriyan/xfingine)

## License

Apache-2.0
