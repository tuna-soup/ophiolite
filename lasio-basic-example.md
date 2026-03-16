# Rust Equivalent to a Basic `lasio` Workflow

## Read a LAS file and inspect metadata

```rust
use lithos_las::{CurveSelector, ReadOptions, read_path};

fn main() -> Result<(), lithos_las::LasError> {
    let las = read_path("examples/sample.las", &ReadOptions::default())?;

    println!("LAS version: {}", las.summary.las_version);
    println!(
        "Well: {}",
        las.well.get("WELL").unwrap().value.display_string()
    );
    println!("Curves: {:?}", las.keys());

    let depth = las.get_curve("DEPT").unwrap().numeric_data().unwrap();
    let dt = las.get_curve("DT").unwrap().numeric_data().unwrap();
    println!("First depth/sample: {} -> {}", depth[0], dt[0]);

    let stacked = las.stack_curves(
        CurveSelector::Names(vec!["DT".into(), "RHOB".into(), "NPHI".into()]),
        true,
    )?;
    println!("First stacked row: {:?}", stacked[0]);

    Ok(())
}
```

## Use the intermediate in-memory table

```rust
use lithos_las::{ReadOptions, read_path};

fn main() -> Result<(), lithos_las::LasError> {
    let las = read_path("examples/sample.las", &ReadOptions::default())?;
    let table = las.data();

    let dt = table.column("DT").unwrap().numeric_values().unwrap();
    let window = table.slice_rows(0, 2);

    println!("DT values: {:?}", dt);
    println!("Window rows: {}", window.row_count());
    Ok(())
}
```

## Write and reopen an optimized package

```rust
use lithos_las::{ReadOptions, open_package, read_path, write_package};

fn main() -> Result<(), lithos_las::LasError> {
    let las = read_path("examples/sample.las", &ReadOptions::default())?;
    write_package(&las, "tmp/sample.laspkg")?;

    let package = open_package("tmp/sample.laspkg")?;
    println!("Package summary: {:?}", package.summary());
    println!("Stored files: metadata.json + curves.parquet");
    Ok(())
}
```
