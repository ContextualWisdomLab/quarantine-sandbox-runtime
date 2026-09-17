# Network lifecycle owner repair lane

Date: 2026-09-14

This branch is an ordinary descendant of network-owner RED exact `95f9e2c73b03a8b6a38f8accbb363adfea56b723`. It exists to give the canonical #22/#23/#41/#48 network tests an executable successor lane without changing production before causal RED evidence exists.

The required GREEN is intentionally narrow and must compose all network ownership controls already carried by the RED parent:

- inspect the newly created Podman network before container creation and acquire its full runtime `.ID`;
- bind container creation to that acquired ID, not to the generated `qsr-net-*` correlation name;
- verify the running container is attached to exactly the owned deny-by-default network and reject missing or additional attachments before readiness;
- never use `podman network rm --force` as sandbox cleanup authority; an in-use network containing a foreign member must fail closed rather than delete that member;
- preserve the public `qsr-net-*` value as correlation metadata unless a separately versioned public contract changes it;
- carry exact acquired network authority only in runtime-private lifecycle state.

No production GREEN is present in this staging commit. Before source repair, exact-head execution must show the inherited network-binding, foreign-safe-cleanup, and acquired-network-identity witnesses failing for their intended causes. After repair, the same witnesses plus the full repository validation suite must be GREEN on the repaired exact head. Positive effective-LSM and real negative-egress/runtime acceptance remain separate release gates.

This lane is also a prerequisite for #118. #118's application-service owner repair may retain its exact acquired-container work and public-receipt/private-authority split, but it must not publish destructive network authority derived from the correlation name.
