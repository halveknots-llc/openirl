# Migration Guide

## From manual OBS + NOALBS-style setups

Map your existing live, BRB, low signal, and privacy scenes into OpenIRL scene roles. Then use the OBS template endpoint to reconcile missing scenes and sources.

## From self-hosted relay setups

Keep your relay process if it works. Add it to `config/openirl.example.toml` as a supervised process and expose its metrics endpoint where possible.

## From managed Cloud OBS workflows

Move final production back to local OBS. Use OpenIRL for contribution ingest, fallback switching, diagnostics, and dashboard control.
