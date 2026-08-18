# Quarantine Sandbox Runtime

Quarantine Sandbox Runtime is a ContextualWisdomLab leaf product: a
**source-agnostic, credential-free artifact analysis runtime**.

It analyzes hostile artifact bytes that a host already holds. It does not
log into mailboxes, forges, object stores, or ticket systems. It does not
need a sibling checkout of naruon, wardnet, gyeot, or EgressWeave. A host
calls it through published contracts. That hub-and-leaf call is the
supported MSA path — **따로 또 같이** — not a reason to merge repositories
or fold this leaf into a hub.

This repository is the product source of truth. The default branch currently
publishes product authority, architecture decisions, and consumer contracts.
It does not publish an executable package, crate, wheel, or test harness.

## What this product is

Operators and buyers use this leaf when they need **the same analysis
contract** for artifacts that arrived from different channels: email
attachments, issue or pull-request uploads, connector inputs, or a direct
API submit. The runtime identity of an artifact is the bytes themselves,
not the transport that carried them.

| Property | Meaning |
| --- | --- |
| Source-agnostic | Analysis does not change because the host is naruon, wardnet, gyeot, or an independent operator. The host supplies bounded source context; the runtime does not fetch the source. |
| Credential-free | The runtime does not receive or use source-system credentials. The host that already holds the bytes submits them. |
| Analysis, not verdict | This leaf returns analysis evidence. It does not decide malicious or benign, admit mail, block HTTP, or retain incidents. |
| Independent and callable | The leaf runs without a hub checkout. Composition hubs call it; they do not absorb it. |

This leaf owns artifact-analysis evidence. Neighbor products keep their own
authority:

- [wardnet](https://github.com/ContextualWisdomLab/wardnet) owns WAF/IDS
  edge enforcement and SOC response policy.
- [naruon](https://github.com/ContextualWisdomLab/naruon) owns email-workspace
  admission (whether an attachment enters the store).
- [EgressWeave](https://github.com/ContextualWisdomLab/EgressWeave) owns
  SSRF- and DNS-rebinding-safe outbound HTTP.

See [ADR 0001](docs/adr/0001-product-authority-boundary.md) for the full
boundary.

## Composition hubs

Leaf products stay independently deployable. Composition hubs call them as
published dependencies. Do not fold this runtime into naruon, wardnet, or
another hub repository.

| Hub | Role | How it calls this leaf |
| --- | --- | --- |
| [`naruon`](https://github.com/ContextualWisdomLab/naruon) | Email workspace and judgment surface | First-class consumer of the published analysis contract. Naruon wiring is a separate repository change. This leaf does not check out naruon. |
| [`gyeot` (곁)](https://github.com/ContextualWisdomLab/gyeot) | On-device wellness composition hub | Call this leaf through the same published contract when a host needs artifact analysis. This leaf does not check out gyeot. |

Wardnet may consume the same published contract as an authorized SOC
caller. That consumption does not move WAF/IDS authority into this
repository and does not move this leaf into wardnet.

## Use it independently

Independent use means this repository alone is enough to understand, pin,
and later operate the product. You do not clone sibling CWL repositories
next to it.

1. Pin a revision or release of
   `ContextualWisdomLab/quarantine-sandbox-runtime`.
2. Read the published consumer contract and the ADRs in this repository.
3. Keep host policy (email admission, WAF/IDS verdicts, outbound HTTP,
   retention, notification) in the host.
4. When a runtime implementation is published from **this** repository,
   call that published artifact. Do not vendor a copy into a hub repo and
   do not treat an unmerged implementation branch as the product.

Until a runtime artifact is published from this repository, the contracts
and ADRs are the operator surface. There is no package install command and
no local test harness on the default branch. Do not invent one.

## How a host consumes published contracts

Hosts consume contracts from **this** repository. They do not require a
sibling checkout of naruon, wardnet, gyeot, or EgressWeave, and they do not
copy unpublished files from those trees.

The published consumer contract is
[`docs/contracts/consumer-contract.md`](docs/contracts/consumer-contract.md).

A host:

1. Pins this repository (git revision, release tag, or a later published
   package from this same product).
2. Reads the consumer contract and the authority ADRs.
3. Submits artifact bytes the host already holds, plus bounded source
   context the host is willing to disclose.
4. Treats the response as analysis evidence. Verdict, admission, block,
   notify, and retain stay in the host.

Do not copy this product into naruon, wardnet, or a hub repository to make
the call cheaper. Do not require this leaf to clone those hubs to compile,
document, or operate.

## Architecture decisions

| ADR | Decision |
| --- | --- |
| [0001](docs/adr/0001-product-authority-boundary.md) | Product and authority boundary versus wardnet, naruon, and EgressWeave |
| [0002](docs/adr/0002-credential-free-source-agnostic-analysis.md) | Credential-free, source-agnostic quarantine analysis |
| [0003](docs/adr/0003-published-contract-consumption.md) | Published-contract consumption and MSA **따로 또 같이** |
| [0004](docs/adr/0004-omit-unimplemented-scope.md) | Omit features that are not in this product’s published scope |

Verified international sources are listed in
[`docs/REFERENCES.md`](docs/REFERENCES.md).
