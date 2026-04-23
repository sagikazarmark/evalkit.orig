# TODOs

## Define packaging for non-crate artifacts

- What: Define packaging and publish/install strategy for the Python shim, TypeScript shim, and GitHub Action.
- Why: These user-facing artifacts already exist in source form, but there is still no agreed install/update path for them.
- Pros: Makes adoption practical, reduces support churn, and turns source-only tooling into shippable product surface.
- Cons: Adds release and maintenance overhead before demand is fully proven.
- Context: `docs/ROADMAP.md:149-152,233-235` and `docs/gap-analysis.md:95-97,138-139` show source-level progress without a finished distribution story. This was intentionally deferred from the refreshed competitive-analysis plan.
- Depends on / blocked by: Final decision on which non-crate artifacts are strategic enough to publish and support long-term.

## Resolve long-term product naming

- What: Decide whether `evalkit` remains the shipped name or whether a real rename should be executed across docs, crates, examples, and messaging.
- Why: Strategy and positioning docs keep colliding with naming drift, which makes future product communication fuzzy and causes repeated re-litigation.
- Pros: Creates one durable naming source of truth and prevents more mixed-language docs later.
- Cons: Can consume coordination time before there is a concrete distribution or go-to-market reason to rename.
- Context: The refreshed review kept `evalkit` in `docs/verda-competitive-analysis.md` for repo clarity, but both the original document and the outside voice flagged naming drift as product-costly.
- Depends on / blocked by: Product positioning decision and appetite for a rename across package surfaces.
