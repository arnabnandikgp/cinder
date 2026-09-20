<p align="center">
  <img src="assets/cinder-header.png" alt="Cinder" width="620" />
</p>

# Cinder

Cinder is being designed as a private, programmatic perpetual-futures trading
layer that integrates existing venue liquidity through an attested confidential
runtime.

This branch intentionally starts without an implementation scaffold. The
financial model, target venue, custody boundary, TEE architecture, technology
stack, and repository layout will be derived from reviewed product requirements
and authoritative integration specifications before code is introduced.

The former implementation remains recoverable from Git history. It is not an
architectural template for this branch.

## Current status

- Product and integration context is being collected locally under ignored
  `work/`.
- No venue adapter, on-chain program, confidential service, client SDK, web
  application, or CI pipeline has been selected or scaffolded.
- Brand artwork is retained under `assets/` independently of implementation.

The next tracked change should be an evidence-backed architecture and workspace
plan produced after the venue and TEE materials have been reviewed.
