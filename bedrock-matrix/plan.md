# Bedrock Join Crash Reproduction Matrix

This directory contains a controlled reproduction matrix for the Bedrock client crash described in `../bedrock_fix_plan.md` and `../bedrock_whatitried.md`. The goal is to separate **binary** differences from **runtime-data** differences so the next code change is targeted at the real cause.

## Layout

```
bedrock-matrix/
├── plan.md                 # this file
├── README.md               # quick-start for the impatient
├── scripts/
│   ├── build.ps1           # build both binaries
│   ├── setup-runtimes.ps1  # copy runtime dirs into the matrix
│   ├── run-cell.ps1        # run one matrix cell
│   └── run-matrix.ps1      # run all four cells
├── binaries/
│   ├── original/           # Original-Pumpkin debug build
│   └── current/            # this fork debug build
├── runtimes/
│   ├── original-clean/     # copy of ../Original-Pumpkin runtime
│   └── runner-copy/        # copy of ../PumpkinRunner runtime
└── results/
    ├── original-original.md
    ├── original-runner.md
    ├── current-original.md
    └── current-runner.md
```

## Matrix

| Cell | Binary | Runtime data | What it tests |
|------|--------|--------------|---------------|
| A | `Original-Pumpkin` | Original clean runtime | Known-working baseline |
| B | `Original-Pumpkin` | Copied Runner runtime | Does the failure follow the **data**? |
| C | Current fork (`../Pumpkin`) | Original clean runtime | Does the failure follow the **binary**? |
| D | Current fork (`../Pumpkin`) | Copied Runner runtime | Normal failing case under controlled conditions |

## Hypothesis → next action

- **If A passes, B fails, C passes, D fails** → the crash follows the Runner world data. Investigate chunk/entity serialization for that specific world (Phase 6 of `../bedrock_fix_plan.md`).
- **If A passes, B passes, C fails, D fails** → the crash follows the current binary. The leading suspect is `pumpkin/src/net/bedrock/mod.rs` queue refactor; try the lossy `try_enqueue_packet` revert next.
- **If A passes, B fails, C fails, D fails** → both the binary and the data contribute. Fix the binary first on clean data, then validate with Runner data.
- **If A fails** → the "known-working" baseline is not reproducible in this environment. Stop and document what differs (client version, OS, network, config).

## Prerequisites

- Windows with PowerShell 5.1+ or PowerShell Core.
- Rust toolchain installed (the same one used for `../Pumpkin` and `../Original-Pumpkin`).
- Bedrock client 1.26.30 / 1.26.33 available for testing.
- `../Original-Pumpkin` and `../PumpkinRunner` exist at the paths expected by the scripts.
- Enough disk space for two runtime copies and two build artifacts (~few GB).

## Step-by-step usage

### 1. Build both binaries

```powershell
.\scripts\build.ps1
```

This produces:
- `binaries/original/pumpkin.exe`
- `binaries/current/pumpkin.exe`

### 2. Copy runtime data

```powershell
.\scripts\setup-runtimes.ps1
```

This produces:
- `runtimes/original-clean/`
- `runtimes/runner-copy/`

The original `../PumpkinRunner` runtime is **not modified** during testing.

### 3. Run the full matrix

```powershell
.\scripts\run-matrix.ps1
```

You will be prompted four times to connect the same Bedrock client. Each cell starts the server, waits for the join, records observations, and shuts down cleanly.

To run a single cell manually:

```powershell
.\scripts\run-cell.ps1 -Binary original -Runtime original-clean -ResultFile results/original-original.md
```

### 4. Record results

Fill in the four files under `results/` using `results/template.md` as a guide. At minimum record:

- Did the client reach the world?
- Did the server receive `SetLocalPlayerAsInitialized`?
- Time from join to initialization (or crash).
- Time and sequence range of the first substantial NACK.
- Loaded chunk and entity counts.
- Whether the connection stayed usable for 60 s.
- Any `UDP socket error 10054` or `Failed to lock network writer` messages.
- Whether the client process crashed (`0xc0000005`).

## Important notes

- Use the **same Bedrock client** for all four cells.
- Use the **same render distance** on the client (the log shows the server clamping `8 -> 6`).
- Do not run two cells at the same time; they bind the same UDP port (`19132`) by default.
- If you edit the current fork between cells, rebuild `binaries/current/` with `build.ps1` again.
- The scripts default to debug builds for fast iteration. Switch to `--release` in `build.ps1` only after the crash is understood.

## Safety

- No script modifies `../PumpkinRunner` or `../Original-Pumpkin`.
- All destructive operations (server start/stop, file copies) happen inside `bedrock-matrix/`.
- Each cell uses its own working directory so configs, logs, and world files do not leak between runs.

## After the matrix

Once the matrix identifies whether the failure follows the binary or the data, open `../bedrock_fix_plan.md` and proceed with the matching phase:

- Binary issue → Phase 2/4 (restore parity or implement ordered bounded writer).
- Data issue → Phase 6 (serializer-side chunk validation and targeted fixes).
