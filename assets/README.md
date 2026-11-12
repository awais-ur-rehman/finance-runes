# Assets

Static resources. Keep this folder small and purposeful; large binaries do not belong in git.

| Folder | Contents | Rules |
| --- | --- | --- |
| `icons/` | App icon, toolbar glyphs | SVG preferred; exported PNG/ICNS at required sizes only |
| `fonts/` | Bundled typefaces | Only with a redistributable licence; record the licence beside the file |
| `fixtures/` | Sample import files for tests and demos | **Synthetic only** — invented payees, amounts and bank names |

## Fixture rules

- Never a real bank statement, even with the name changed. Regenerate from a script instead.
- Name fixtures by shape, not by bank: `csv_debit_credit_columns_eur.csv`, `ofx_simple_usd.ofx`.
- Keep them tiny — a handful of rows is enough to exercise the parser.
- Every fixture has a matching expected result next to it.

See [data privacy and security](../docs/engineering/data-privacy-and-security.md).
