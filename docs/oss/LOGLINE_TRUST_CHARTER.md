# LogLine Trust Charter

**Status**: active  
**Owner**: LogLine Foundation  
**Last reviewed**: 2026-02-22

## Purpose

Define the institutional trust contract for the official public registry experience and its receipt model.

## Core Assertions

1. The official trust-facing app is **Chip Registry and Certified Runtime**.
2. It is the only foundation product allowed to carry `LogLine` in the product identity.
3. A receipt is the materialization of trust for runtime execution.
4. Public receipt identity is CID-first and portable.

## Branding and Naming Rule

- The foundation keeps strict naming discipline for products.
- The trust-facing registry experience is the exception that represents trust and reputation across all app domains.
- Trademark constraints still apply; see `TRADEMARK_POLICY.md`.

## Domain and URL Model

- Default API/runtime and operations surface can remain under `ubl.agency` and related subdomains.
- Public receipt model is standardized as:
  - `https://logline.world/r#ubl:v1:<token>`
- This is an explicit exception for trust portability and universal CID addressing.

## Onboarding and Scope Rule

- Chips can technically be submitted through the gate interface.
- Institutional ALLOW policy must be app-scoped:
  - unknown app scope -> `DENY` or `REQUIRE` with receipt
  - known app scope + allowed type + passing policy -> `ALLOW`
- The trust app is a gateway of trust, not a replacement for domain-specific product apps.

## Receipt Signature Line

Every official trust receipt UI must end with the exact phrase:

`We Trust and Build with LogLine.`

## Governance

- Changes to this charter require RFC/maintainer approval.
- Any conflict is resolved by `GOVERNANCE.md`, `TRADEMARK_POLICY.md`, and `SECURITY.md`.
