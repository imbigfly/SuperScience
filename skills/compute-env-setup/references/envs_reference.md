---
name: compute-envs-reference
description: Historical scientific environment recipes translated into package-order, cache, and validation examples for Wisp direct execution contexts. Read alongside compute-env-setup when building a matching user-space environment on SSH.
---

# Compute environment reference — worked examples

These are recipe examples, not environments Wisp automatically provides. Translate
the relevant package phases, cache variables, and validation witness into an
idempotent setup script for the selected direct SSH context. Container paths and
resource tiers are historical reference values; replace them with probed,
user-writable paths and actual hardware. Record the resulting activation command
and validation evidence in the project rather than assuming an environment-name
resolver exists.

| env | base | weights | sm_90 | tier default |
|---|---|---|---|---|
| dataml-cpu | python:3.12-slim | — | n/a | 4c/16G |
| bio-cpu | python:3.12-slim | — | n/a | 4c/16G |
| chem-cpu | python:3.12-slim | — | n/a | 4c/16G |
| singlecell-cpu | python:3.12-slim | — | n/a | 8c/32G |
| genomics-cpu | python:3.12-slim | — | n/a | 8c/64G |
| imaging-cpu | python:3.12-slim | — | n/a | 4c/32G |
| torch-geometric-gpu | pytorch:2.7.1-cu126-runtime | — | ✅ | 1gpu/32G |

---

## CPU envs

All six share `base: python:3.12-slim`. No weight mounts, no egress. Single pip phase. The only per-env decisions are which apt `.so` deps the wheels link against and which CLI binaries to bake.

### dataml-cpu
**apt:** `libgomp1 build-essential`
**pip:** scikit-learn xgboost statsmodels pymc arviz shap umap-learn networkx dask[complete] polars zarr gcsfs s3fs aeon pymoo
**weights:** none
**egress_hosts:** none
**validated:** RF + XGBoost fit on 200×5 → score (1.0, 1.0); polars DataFrame round-trip
**gotchas:** `aeon` PyPI name resolves to a 0.0.0 squatter on some mirrors — pin `aeon>=1.0`. xgboost wheel pulls `nvidia-nccl-cu12` (~200MB dead weight on CPU).

### bio-cpu
**apt:** `libgomp1 build-essential libgl1 libglib2.0-0` — `libglib2.0-0` is for pyopenms `.so`
**pip:** biopython prody biotite scikit-bio pyopenms ete3 cobra neurokit2 FlowIO matchms numpy scipy pandas
**weights:** none
**egress_hosts:** none
**validated:** ubiquitin FASTA → ProtParam → 76aa, MW 8564.7, pI 6.56
**gotchas:** none hit

### chem-cpu
**apt:** `build-essential libxrender1 libxext6 libsm6 libgomp1` — X libs for rdkit's drawing code
**pip:** rdkit openbabel-wheel datamol useful_rdkit_utils molfeat PyTDC aizynthfinder
**weights:** none (aizynthfinder retro data NOT baked — `download_public_data` left for runtime)
**egress_hosts:** none
**validated:** aspirin SMILES → MolWt 180.16, Morgan FP 24 on-bits
**gotchas:** PyTDC transitively pulls torch + jupyter + scanpy + ~250 deps and forces a sklearn-from-source build → ~19 min build, ~5GB image. If you don't need TDC, drop it.

### singlecell-cpu
**apt:** `libgomp1 build-essential`
**pip:** scanpy anndata leidenalg igraph scrublet cellxgene-census samap
**weights:** none
**egress_hosts:** none
**validated:** scanpy normalize+PCA+neighbors+leiden on 100×50 random AnnData → 1 cluster
**gotchas:** louvain dropped (no py3.12 wheel; leidenalg covers it). samap historically pins scanpy<1.10 — drop if it conflicts.

### genomics-cpu
**apt:** `samtools bedtools bwa spades wget bzip2 build-essential libgomp1 libcurl4-openssl-dev libbz2-dev liblzma-dev`
**run_commands:** fetch bwa-mem2 v2.2.1 static binary tarball → `/opt`, symlink dispatcher + arch variants into `/usr/local/bin/`. Debian's apt has only legacy `bwa`, not `bwa-mem2`.
**pip:** pysam deeptools gtars pydeseq2 anndata biopython
**weights:** none
**egress_hosts:** none
**validated:** bwa-mem2 index 800bp ref → align 2 reads → pysam parse SAM (`2.2.1 2`)
**gotchas:** bwa-mem2 has a fixed 3.6GB host-RAM prealloc regardless of ref size — tier needs `mem_gib≥32`.

### imaging-cpu
**apt:** `libopenslide0 libopenslide-dev libvips42 libgl1 libglib2.0-0 build-essential`
**pip:** pydicom pylibjpeg pylibjpeg-libjpeg openslide-python pillow scikit-image
**weights:** none
**egress_hosts:** none
**validated:** sobel filter on 128×128 random uint8 → mean 0.2256; pydicom imports
**gotchas:** histolab dropped (numpy<1.22 pin). openslide-python needs the apt `libopenslide0`, not just the wheel.

---

## GPU envs

### torch-geometric-gpu
**base:** `pytorch/pytorch:2.7.1-cuda12.6-cudnn9-runtime`
**apt:** git build-essential
**pip_phases:**
1. `pyg_lib torch_scatter torch_sparse torch_cluster torch_spline_conv` with `find_links=https://data.pyg.org/whl/torch-2.7.0+cu126.html` — **find_links not extra_index** (flat HTML, not PEP-503). Wheel URL encodes torch-minor + CUDA; this is why pyg lives in its own env.
2. `torch_geometric` (pure-python, no version coupling)
3. `lightning>=2.2` — Trainer workflows are the common pyg consumer; ship it here so the env is self-contained.
**weights:** none
**egress_hosts:** `github.com raw.githubusercontent.com codeload.github.com data.pyg.org` — `torch_geometric.datasets.*` fetch benchmark data from there
**validated:** GCNConv(8→4) forward → `(4,4)` cuda tensor; KarateClub 2-layer fwd+bwd loss decreases
**gotchas:** isolated specifically because pyg wheels lag torch releases by weeks.
