# ophiolite-automation

`ophiolite-automation` is a thin Python wrapper around the local `ophiolite-cli`.

It does not introduce a second backend. It shells out to the existing JSON-producing CLI so
scripts, notebooks, and local automation can reuse the same platform-owned operation surface.

## Scope

Current wrapped platform operations:

- `operation-catalog`
- `create-project`
- `open-project`
- `project-summary`
- `list-project-wells`
- `list-project-wellbores`
- `import`
- `inspect-file`
- `summary`
- `list-curves`
- `examples`
- `generate-fixture-packages`

## Usage

From this directory:

```bash
python -m pip install -e .
ophiolite-automation operation-catalog
```

Or from Python:

```python
from ophiolite_automation import OphioliteApp

app = OphioliteApp()
catalog = app.operation_catalog()
print(catalog["catalog_name"])
```

## Surface Conformance

Ophiolite keeps a checked-in platform operation catalog at
`crates/ophiolite-cli/operations/catalog.json`.

Run the Python-side conformance check with:

```bash
ophiolite-automation verify-surface-contracts
```
