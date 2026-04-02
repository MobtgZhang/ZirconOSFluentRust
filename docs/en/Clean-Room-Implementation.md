# Clean-room implementation (ZirconOSFluent)

This project targets **documented, public** behavior (for example MSDN/WDK-style descriptions of roles and ABIs) and **observations from our own tests** in QEMU or controlled environments.

## Allowed

- Public specifications, ABI descriptions, and behavioral documentation.
- Independent test programs and hardware/QEMU experiments.
- Third-party **text** descriptions of published interfaces, respecting their licenses and citing sources when appropriate.

## Not allowed

- Copying or adapting code from Windows retail/preview binaries.
- Using leaked source code or confidential materials as an implementation source.
- Treating internal symbol names or private structure layouts as authoritative without independent verification.

## Practice in this tree

- Use **ZirconOSFluent-local** names and module boundaries; align semantics with public docs, not with reverse-engineered dumps as a primary source.
- When layout or calling convention must match an external contract, **cross-check** with documentation and self-tests, and note the basis in code comments where helpful.

See also: [Roadmap-and-TODO.md](Roadmap-and-TODO.md) baseline statement.
