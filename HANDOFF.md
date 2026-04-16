# Bitmapflow-CLI Handoff

## What was built

A standalone Rust CLI tool (`cli/` directory) that extracts the core optical flow frame interpolation from the [Bitmapflow](https://github.com/Bauxitedev/bitmapflow) GUI app and makes it usable from the command line. **It works and was tested successfully on Linux.**

- **Repo**: `johnfalcon/bitmapflow-cli`
- **Branch**: `claude/add-cli-animation-support-ebuo1`
- **Latest commit**: `5731688`

## How it works

Takes a **spritesheet PNG** (one image with N frames laid out in a grid), generates interpolated inbetween frames using optical flow, and outputs a **new spritesheet PNG**.

**Example**: 5-frame sprite with `--inbetweens 1 --loop` produces 10 frames.

```bash
bitmapflow-cli \
  --input spaceship.png \
  --output spaceship_smooth.png \
  --frame-width 100 --frame-height 100 --frame-count 5 \
  --inbetweens 1 \
  --loop \
  --columns 5 \
  --json
```

**JSON output** (for machine/MCP consumption):

```json
{
  "success": true,
  "input": "spaceship.png",
  "output": "spaceship_smooth.png",
  "input_frame_count": 5,
  "output_frame_count": 10,
  "frame_width": 100,
  "frame_height": 100,
  "output_columns": 5,
  "algorithm": "SimpleFlow",
  "inbetweens": 1,
  "looped": true
}
```

## Source files (554 lines total)

| File | Lines | Purpose |
|------|-------|---------|
| `cli/src/main.rs` | 250 | CLI arg parsing (clap), orchestration, JSON output |
| `cli/src/interpolation.rs` | 172 | Optical flow computation (OpenCV), frame warping |
| `cli/src/spritesheet.rs` | 82 | Spritesheet load/save (split into frames, reassemble grid) |
| `cli/src/types.rs` | 50 | Frame, FlowAlg, InterpolationParams types |
| `cli/Cargo.toml` | — | Dependencies: opencv 0.93, clap 4, image 0.25, serde |

## CLI flags reference

### Required

| Flag | Description |
|------|-------------|
| `--input <PATH>` | Input spritesheet PNG |
| `--output <PATH>` | Output spritesheet PNG |
| `--frame-width <PX>` | Width of each frame in pixels |
| `--frame-height <PX>` | Height of each frame in pixels |
| `--frame-count <N>` | Number of frames in the input |

### Optional

| Flag | Default | Description |
|------|---------|-------------|
| `--inbetweens <N>` | 1 | Inbetweens per frame pair |
| `--columns <N>` | total frames | Output spritesheet columns |
| `--loop` | off | Seamless loop (inbetweens between last and first frame) |
| `--motion-multiplier <F>` | 1.0 | Exaggerate (>1) or diminish (<1) motion |
| `--algorithm <name>` | simpleflow | `simpleflow` or `denserlof` |
| `--json` | off | Machine-readable JSON output |

### SimpleFlow parameters (optional)

| Flag | Default |
|------|---------|
| `--layers` | 3 |
| `--averaging-block-size` | 2 |
| `--max-flow` | 4 |

### DenseRLOF parameters (optional)

| Flag | Default |
|------|---------|
| `--forward-backward-threshold` | 1.0 |
| `--grid-step-x` | 6 |
| `--grid-step-y` | 6 |
| `--use-post-proc` | off |
| `--use-variational-refinement` | off |

## Build requirements

The tool depends on **OpenCV 4.x** (C++ library) + **libclang** at build time.

### Linux (confirmed working)

```bash
sudo apt install libopencv-dev libopencv-contrib-dev libclang-dev
cd cli && cargo build --release
```

### macOS (confirmed working in CI)

```bash
brew install opencv llvm
export LLVM_CONFIG_PATH=$(brew --prefix llvm)/bin/llvm-config
cd cli && cargo build --release
```

### Windows (broken in CI, may work locally)

The OpenCV Rust binding generator panics in GitHub Actions because clang cannot find MSVC standard library headers. Building locally with Visual Studio installed should work since VS properly configures all include paths. See [Windows CI issue](#windows-ci-issue) below.

## GitHub Actions workflow

**File**: `.github/workflows/cli-build.yml`

- **Triggers**: push to `main`/`claude/**`, PRs to `main`, `workflow_dispatch`, tags `cli-v*`
- **Build matrix**: Linux x86_64, Windows x86_64, macOS ARM64 (with `fail-fast: false`)
- **Release**: Creates GitHub Release on tag push or `workflow_dispatch`
- **Status**: Linux and macOS builds pass. Windows fails.

## What's NOT done

### Windows CI issue

The `opencv-rust` crate's binding generator fails on GitHub Actions Windows runners. The generator spawns subprocesses that use libclang to parse C++ headers, and those subprocesses cannot find MSVC's standard library includes (`<string>`, `<vector>`, etc.) regardless of environment configuration.

**Attempted fixes** (all failed):

1. `ilammy/msvc-dev-cmd@v1` to set up MSVC developer environment
2. Setting `LIBCLANG_PATH` to LLVM bin directory
3. Passing MSVC/SDK include paths via `OPENCV_CLANG_ARGS` with `-isystem` flags
4. Adding OpenCV DLLs and LLVM to `PATH`

**Potential paths forward**:

- Try `vcpkg` instead of `choco` (officially recommended by opencv-rust for Windows, but slow ~30min build from source)
- Try pinning specific compatible LLVM + OpenCV versions via choco
- Try disabling the `clang-runtime` Cargo feature (static clang linking instead)
- Build on a local Windows machine where Visual Studio environment is properly configured
- Use WSL2 on Windows

### GitHub Release

The release job is configured (`workflow_dispatch` trigger) but hasn't produced a release yet because the Windows build fails. With `fail-fast: false`, Linux and macOS build fine — a release could be created from just those two by removing Windows from the matrix.

### MCP server wrapper

Not started. To expose this as an MCP tool on a NAS, you would:

1. Build the Linux binary on the NAS
2. Write a thin MCP server (Python or Node) that wraps the CLI
3. The MCP tool would accept: input image (base64 or file path), frame dimensions, frame count, inbetweens, loop flag
4. Run `bitmapflow-cli` as a subprocess and return the output image
5. Claude Desktop connects via MCP transport (stdio or SSE)

## Recommended next steps

### Option A — NAS + MCP (recommended)

1. SSH into your NAS (assuming Linux-based)
2. Install dependencies:
   ```bash
   sudo apt install libopencv-dev libopencv-contrib-dev libclang-dev
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```
3. Clone and build:
   ```bash
   git clone https://github.com/johnfalcon/bitmapflow-cli.git
   cd bitmapflow-cli
   git checkout claude/add-cli-animation-support-ebuo1
   cd cli && cargo build --release
   ```
4. Test:
   ```bash
   ./target/release/bitmapflow-cli \
     --input ../examples/Hell-Hound-Files/PNG/hell-hound-run.png \
     --output /tmp/test.png \
     --frame-width 67 --frame-height 32 --frame-count 5 \
     --inbetweens 1 --loop --columns 5 --json
   ```
5. Create an MCP server that wraps the binary so Claude Desktop can call it as a tool directly

### Option B — Fix Windows CI

1. Try building locally on your Windows machine first (with Visual Studio installed)
2. If local build works, the issue is CI-specific — consider the `vcpkg` approach in CI

### Option C — Remove Windows from CI, release Linux + macOS only

1. Remove Windows from the build matrix (was done in commit `7936d26`, then reverted)
2. Trigger a release via `workflow_dispatch` from the Actions tab
3. Use WSL2 on your Windows machine to run the Linux binary
