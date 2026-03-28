# Third-party references (`references/`)

**中文**：[../cn/References-Policy.md](../cn/References-Policy.md)

This project may vendor or clone **read-only** trees under `references/` for local lookup. **They are not part of the ZirconOS license for your contributions** unless explicitly stated.

## `references/win32` (Microsoft Learn–style content)

- Use **only** to understand **public, factual** behavior (API names, call order, subsystem roles).
- **Do not** copy paragraphs, long tables, tutorial prose, or sample code from those files into ZirconOS source or documentation.
- Prefer linking to **official** Microsoft documentation in reviews when external citation is needed; keep **in-repo** text original.
- Compatibility goals and trademark notice: [Architecture.md](Architecture.md).

## `references/r-efi` (UEFI bindings)

- UEFI is a **public specification**; `r-efi` is a Rust mapping useful for **semantic alignment** (e.g. memory types, protocol shapes).
- The **canonical** handoff layout for this workspace is [`nt10-boot-protocol`](../../crates/nt10-boot-protocol/src/lib.rs) and the ZBM10 loader; do not duplicate `r-efi` types inside the kernel unless there is a clear interoperability need.
- Comments in our code should be **short and original**—avoid pasting UEFI spec text verbatim.

## Implementation rule of thumb

- **Standards**: UEFI, ACPI, SMBIOS, PE/COFF — cite the standard by name when it matters; implement from the spec’s semantics, not from copyrighted prose.
- **Rust modules**: document **ZirconOS** behavior in module and item docs; point to [PROCESS_NT10.md](PROCESS_NT10.md) for workflow.
