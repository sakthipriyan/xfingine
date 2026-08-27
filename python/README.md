# xfingine

Python bindings for [**Xfingine**](https://github.com/sakthipriyan/xfingine) —
pure computation engines for personal finance planning.

Data in, arithmetic, data out. No UI, no network, no clock — the same input
always produces the same output.

```bash
pip install xfingine
```

## Usage

```python
import xfingine

result = xfingine.compute_emi({
    "mode": "emi",            # "emi" | "tenure" | "loanAmount"
    "loanAmount": 5_000_000,
    "months": 240,
    "interestRate": 9,        # percent per year
    "inflationRate": 6,       # percent per year
    "start": "2026-01",       # optional, enables dates + per-year breakdown
})

result["emi"]                    # 44986    — the monthly payment
result["totals"]["nominalPaid"]  # 10796818 — rupees actually debited
result["totals"]["realPaid"]     # 6391327  — the same, in 2026 rupees
len(result["schedule"])          # 240 rows, month by month
len(result["years"])             # 20 rows, one per calendar year
```

Straight into a DataFrame:

```python
import pandas as pd

df = pd.DataFrame(result["schedule"])
df[["year", "month", "emi", "principal", "interest", "realEmi"]].head()
```

There is a `_json` twin of every function for string in / string out:

```python
xfingine.compute_emi_json('{"mode":"emi","loanAmount":5000000,"months":240,"interestRate":9}')
```

Invalid input raises `ValueError` with a readable message:

```python
xfingine.compute_emi({"mode": "tenure", "loanAmount": 5_000_000,
                      "emi": 1000, "interestRate": 12})
# ValueError: monthly payment of 1000.00 does not cover the first month's
#             interest of 50000.00; the loan would never be repaid
```

## Conventions

- Money comes back as whole rupees (`int`).
- Rates are percentages, not fractions: `9` means 9%.
- All keys are `camelCase`.
- Omit `start` and the engine reads no calendar — schedule rows carry no dates
  and `years` is empty.

Full documentation: [github.com/sakthipriyan/xfingine](https://github.com/sakthipriyan/xfingine)

## License

Apache-2.0
