# Public Seismic Open-Data API Candidates

Last verified: 2026-04-16

This note captures the public sources that currently look relevant for machine-assisted discovery of open seismic datasets beyond the limited public NLOG F3-style products.

## Shortlist

### 1. SODIR / Diskos (Norway)

- Official open-data entry point: `https://www.sodir.no/en/about-us/open-data/`
- ArcGIS REST root: `https://factmaps.sodir.no/api/rest/`
- Seismic survey layer root:
  `https://factmaps.sodir.no/api/rest/services/Factmaps/FactMapsWGS84/FeatureServer`
- Example seismic layer:
  `https://factmaps.sodir.no/api/rest/services/Factmaps/FactMapsWGS84/FeatureServer/405`
- Official Diskos seismic page:
  `https://www.sodir.no/en/diskos/seismic/`

What is useful:

- This is a real queryable ArcGIS REST surface, not just a brochure page.
- The seismic layers expose machine-usable metadata such as survey name, status, company, main type, subtype, source type, area, and links to fact pages/maps.
- The public Diskos page explicitly says Diskos contains "post-stack seismic, velocity and navigation data" and that "from 2012, field and pre-stack data" are reported as well.
- The live REST sample we checked includes active 4D survey metadata, so the exposed metadata is already richer than the public NLOG seismic REST.

Limits:

- The public REST we verified is a metadata/GIS surface, not an obvious file-product download API.
- Actual seismic file acquisition still appears to flow through Diskos/fact pages rather than a clean public REST file inventory.

Assessment:

- Best public REST metadata lead for richer survey families.
- Good candidate for automated survey discovery and prioritization.
- Not yet confirmed as a direct raw-file download API.

### 2. NZP&M Geodata + GIS (New Zealand)

- Official web-services guide:
  `https://www.nzpam.govt.nz/assets/Uploads/maps-geoscience/using-web-services.pdf`
- Official geodata catalogue page:
  `https://www.nzpam.govt.nz/maps-geoscience/geodata-catalogue`
- Official petroleum webmaps page:
  `https://www.nzpam.govt.nz/maps-geoscience/petroleum-webmaps`
- ArcGIS REST service root:
  `https://gis.nzpam.govt.nz/server/rest/services/Public/GeodataCatalogue_Layers/MapServer`
- Seismic 3D layer:
  `https://gis.nzpam.govt.nz/server/rest/services/Public/GeodataCatalogue_Layers/MapServer/7`
- Catalogue root:
  `https://geodata.nzpam.govt.nz/`

What is useful:

- The official PDF says NZP&M publishes WMS, WFS, and ESRI Feature Services.
- The public ArcGIS REST service exposes `Seismic Surveys 2D`, `Seismic Surveys 3D`, and `Geophysical Surveys` layers.
- The live 3D seismic layer exposes fields such as `Title`, `Alias`, `Survey_Subtype`, `Dimension`, `Operator`, `Contractor`, `Environment`, and `Open_File`.
- Sample public records include 3D marine reprocessed surveys such as `MAUI-2018-4D : PR5752`.
- The public CKAN-backed catalogue advertises downloadable file resources, and the HTML search facets we verified include `SGY`, `SEGY`, `SEGD`, `P190`, `ZIP`, `LAS`, and related formats.

Limits:

- The CKAN API endpoints we tried at `https://geodata.nzpam.govt.nz/api/3/action/...` were blocked by the site for our automated requests, even though the catalogue root clearly identifies itself as CKAN 2.11.3.
- The public GIS REST is clearly useful for discovery, but it is not itself the raw-file delivery surface.
- The catalogue says downloads require registration with a RealMe account.

Assessment:

- Best public combination of machine-readable GIS discovery and an apparent file-resource catalogue.
- Very strong candidate for finding real open SEG-Y-like resources.
- Automation may need to use permitted HTML/search flows, an allowed API path, or an authenticated session.

Current verified browser findings from 2026-04-17:

- The public catalogue search page is reachable once a browser clears a simple human-verification challenge cookie.
- The live search page reports `253` datasets for `q=seismic` with `dimension=3D`.
- The same live page reports these useful public format facet counts:
  - `SGY`: 168
  - `SEGY`: 3
  - `segy`: 3
  - `sgy`: 11
  - `ZIP`: 99
  - `SEGD`: 1
  - `P190`: 2
- Sample public survey pages we verified:
  - `KAPUNI-2016-3D : PR5496`
  - `MAUI-2018-4D : PR5770`
  - `MAUI-2018-4D : PR5752`
- These survey pages expose much richer file families than the public NLOG F3 products, including:
  - final stack
  - raw stack
  - near / mid / far angle stacks
  - velocity volumes
  - azimuth volumes
  - time-shift volumes
  - anisotropy-style attributes such as epsilon, delta, and phi
- Example sizes on the verified pages range from about `6.22 GB` to about `28.04 GB`.

Important limit:

- Actual file acquisition still appears to require a `RealMe` login.
- The direct `.../resource/<id>/fpx` file endpoints triggered a second human-verification page when called programmatically without a logged-in download session.
- So NZP&M is now confirmed as a strong survey and product discovery source, but not yet a no-auth bulk-download source.

### 3. NLOG Datacenter + Mapviewer (Netherlands)

- Seismic overview:
  `https://www.nlog.nl/en/seismic-data`
- Datacenter:
  `https://www.nlog.nl/datacenter`
- REST config:
  `https://www.nlog.nl/nlog-mapviewer/rest/config`
- 3D surveys:
  `https://www.nlog.nl/nlog-mapviewer/rest/smc/3d/surveys`

What is useful:

- Public REST exists and is queryable.
- It exposes 2D/3D survey metadata and export flows.
- The public NLOG seismic page explicitly says pre-stack data can be requested and that pre-stack datasets are not yet visible on the maps.

Limits:

- The public REST we verified is survey-level metadata only.
- We did not find a public file-level product inventory for angle stacks, prestack gathers, or velocity volumes.
- For richer products, the workflow appears to fall back to service-desk or owner-request flows.

Assessment:

- Good for public survey discovery.
- Weak for direct automated acquisition of richer open products.
- Not the right place to spend more automation effort unless we want better metadata harvesting.

### 4. Poseidon 3D on AWS (Australia)

- Registry page:
  `https://registry.opendata.aws/tgs-opendata-poseidon/`
- Public bucket browser:
  `https://tgs-opendata-poseidon.s3.amazonaws.com/index.html`
- Raw bucket:
  `s3://tgs-opendata-poseidon`

What is useful:

- This is directly machine-accessible without a portal workflow.
- The registry page says the dataset includes near, mid, far, and full stacks, plus a decimated stacking velocity field.
- The data is already converted to MDIO, which is attractive for modern volumetric IO experiments.

Limits:

- This is public object storage, not a rich metadata REST catalogue.
- The bucket root listing we checked exposed `near_stack.mdio/`, `mid_stack.mdio/`, `far_stack.mdio/`, and `full_stack_agc.mdio/`.
- On 2026-04-16 we did not see an obvious top-level velocity prefix in the public listing, so the published description and current visible layout may not be fully aligned.
- This is not SEG-Y-first distribution; it is MDIO-first in the public bucket.

Assessment:

- Fastest path to richer, openly accessible 3D stacks for ingestion experiments.
- Best option when we want real data right now, not just survey discovery.
- Worth a follow-up pass to determine where the published stacking velocity field actually lives.

Current verified format notes from 2026-04-17:

- The public MDIO root metadata for `far_stack.mdio` identifies:
  - `apiVersion: 1.0.0a1`
  - `processingStage: post-stack`
  - `surveyDimensionality: 3D`
- The public `seismic` array is Zarr v2 with dimensions:
  - `inline`
  - `crossline`
  - `time`
- The public metadata also exposes:
  - `inline`
  - `crossline`
  - `time`
  - `trace_mask`
  - `cdp-x`
  - `cdp-y`
  - `headers`
- The checked `far_stack.mdio` shape is `3437 x 5053 x 1501`.
- A locally downloaded working set of real chunks decoded cleanly with:
  - axis ranges `inline=[983, 4419]`, `crossline=[504, 5556]`, `time=[0, 6000]`
  - a live `seismic` chunk shape `128 x 128 x 128`
  - `Accept-Ranges: bytes` on direct S3 object requests
- A practical implication is that Poseidon supports selective chunk pulls and ROI-style experiments without downloading an entire stack first.

Ophiolite implication:

- MDIO is clearly Zarr-based, but it is not the same layout as the current in-repo TraceBoost Zarr import path.
- The current runtime expects a local TraceBoost manifest-backed Zarr store, while Poseidon MDIO uses MDIO metadata plus `seismic`, `trace_mask`, axis arrays, and coordinate arrays.
- So Poseidon is a good target for a future dedicated `MDIO -> tbvol` adapter, but it is not a drop-in ingest with the current runtime.

### 5. NOPIMS / Geoscience Australia

- NOPIMS root:
  `https://www.ga.gov.au/nopims`
- Discovery and access note:
  `https://www.ga.gov.au/about/projects/resources/the-repository/discovery-and-access`
- Rich 3D example:
  `https://www.ga.gov.au/nopims/news/barrow-dampier-ccs-presdm-repro-2022-3d`
- Public 2D example:
  `https://www.ga.gov.au/about/projects/resources/northern-houtman-sub-basin-project`

What is useful:

- The official repository page now explicitly says data is accessible either directly via NOPIMS or via repository packaging support.
- We verified a concrete rich 3D example: `Barrow-Dampier CCS PreSDM Repro 2022 3D`.
- The official Barrow-Dampier page says the dataset covers about `26,150 km2` and includes:
  - raw and final migrated angle stacks
  - velocity model
  - AVO products
  - migrated gathers
- The same page points to `ENO0810814` in NOPIMS for details.
- We also verified a concrete public 2D example: `GA349` in the Northern Houtman Sub-basin project.
- The Houtman project page says the `GA349` data was processed to `PreSTM` and `PreSDM` and is available for download via NOPIMS or on request.

Limits:

- NOPIMS is still not presenting itself as a clean public REST file inventory like S3 object storage.
- Some richer families still appear to require email/request workflows or account-mediated portal flows.
- We have not yet verified a direct unauthenticated bulk-download endpoint for the Barrow-Dampier 3D products.

Assessment:

- Strong candidate for specific curated surveys with much richer product families than F3-style public examples.
- Better for targeted survey selection than for generic crawl-and-download automation.
- Worth prioritizing when we want angle stacks, gathers, and velocity-model examples tied to a known survey id.

### 6. NAMSS / BOEM

- NAMSS web services:
  `https://walrus.wr.usgs.gov/namss/web-services/`
- BOEM seismic inventory:
  `https://www.boem.gov/oil-gas-energy/resource-evaluation/seismic-data-inventory`

What is useful:

- These are official, machine-reachable surfaces.
- NAMSS exposes a documented WMS with filterable survey metadata.
- BOEM is a useful discovery/inventory portal.

Limits:

- They look more like metadata, WMS, and order/request surfaces than rich public REST file catalogs for 3D stacks or velocity cubes.
- Based on what we verified, they are less immediately useful than SODIR, NZP&M, or Poseidon for the dataset families we want.

Assessment:

- Secondary discovery surfaces.
- Not first-priority for direct acquisition automation.

## Recommendation

If the goal is to find richer open datasets than the public NLOG F3 set, the next most useful path is:

1. `NZP&M` for open survey/file discovery, especially where `Open_File=Yes` and the catalogue advertises `SGY` or `SEGY` resources.
2. `Poseidon` for immediate ingestion experiments on richer public stacks with direct selective-download behavior.
3. `NOPIMS` for targeted richer survey families, especially where official pages already name angle stacks, velocity models, AVO products, and gathers.
4. `SODIR` for broader survey-family discovery, especially 4D, post-stack, velocity, and post-2012 prestack/field-data candidates.

## Why This Matters For Ophiolite

These sources map well onto the current gaps we are closing:

- `NLOG` remains useful for baseline survey discovery, but not for testing richer acquisition families.
- `NZP&M` is the strongest lead for public SEG-Y-ish downloadable resources with machine-readable discovery metadata.
- `Poseidon` is the strongest lead for exercising broader stack ingestion paths immediately, even though it is MDIO rather than raw SEG-Y.
- `NOPIMS` now looks like the strongest lead for curated Australian surveys where angle stacks, velocity products, and gathers are explicitly called out.
- `SODIR` remains the strongest lead for discovering surveys where richer families probably exist, even if the file handoff is less REST-native.
