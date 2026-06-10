# Security policy

## Supported versions

Only the **latest released minor** of animaEngine receives security
fixes. Older minors are out of scope; upgrade to the current release
to pick up patches.

| Version | Supported |
|---------|-----------|
| 0.5.x   | yes       |
| < 0.5   | no        |

## Reporting a vulnerability

Open a private security advisory on GitHub:

<https://github.com/Alexandru2984/animaEngine/security/advisories/new>

Include:

- a description of the issue and the impact you observed,
- a minimal reproducer (config snippet, asset file, drop sequence,
  D-Bus payload, etc.),
- the version of animaEngine, the Linux distribution, and the
  desktop session (X11 / XWayland / wlroots).

Please do **not** open a public issue or pull request that names the
vulnerability until a fix is released. Public issues for general bug
reports remain welcome.

## Threat model

animaEngine is a single-user desktop overlay. The full trust
boundaries, caps, atomic-write guarantees, and the things explicitly
treated as out of scope (same-user processes, root, kernel) are
documented in [docs/threat-model.md](docs/threat-model.md). Read that
before reporting — issues already documented as accepted trade-offs
(e.g., same-user D-Bus access, AT-SPI broadcast) will be closed with
a link back to the relevant section.

## Disclosure timeline

After a report is acknowledged we aim to:

- triage within 7 days,
- ship a fix in the next patch release if the impact is HIGH or
  CRITICAL,
- credit the reporter in the release notes if they want to be named.

No bug bounty is offered.
