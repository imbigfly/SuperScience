---
name: academic-shared
description: Internal shared contracts, protocols, and schemas for academic-paper, academic-paper-reviewer, academic-pipeline, and deep-research. Do not invoke it as a standalone user workflow. Load only the specific shared file requested by another academic skill.
---

# Academic Shared References

Use this package only as a dependency of another installed academic-research skill.

- Load the exact referenced file; do not preload the whole package.
- Treat `contracts/`, `references/`, and the root protocols as shared definitions, not standalone workflows.
- Return to the requesting skill for task logic, output format, and final QA.

Consuming skills reference files as `../academic-shared/...`.
