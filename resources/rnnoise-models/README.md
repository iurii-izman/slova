# rnnoise model placement

Place the rnnoise model file here:

- `resources/rnnoise-models/cb.rnnn`

## Why this file is not committed

`cb.rnnn` is a binary model file with separate licensing/distribution considerations.
This repository keeps only the expected path and setup instructions.

## How Slova uses it

If the file exists, Slova enables FFmpeg filter:

- `arnndn=m=resources/rnnoise-models/cb.rnnn`

If the file is missing, Slova continues without noise reduction.

## Verification

Run:

- `ffmpeg -hide_banner -filters | findstr arnndn` (Windows)
- `ffmpeg -hide_banner -encoders | findstr libopus` (Windows)

And check app logs for whether noise reduction was applied.
