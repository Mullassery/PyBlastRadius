## Description

Brief summary of the change.

## Related Issues

Closes #

## Testing

- [ ] `cargo test --all-features` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo fmt -- --check` passes
- [ ] If this touches `python/` or `src/python.rs`: `maturin develop --release`
      then `pytest tests/ -q` passes
- [ ] Added/updated tests for the change
- [ ] Manually verified the change (describe how)

## Honesty Checklist

This repo's README/ROADMAP_HONEST.md distinguish tested-and-working from
built-but-unverified from not-built. If this PR changes feature status:

- [ ] Updated README.md's Features/Known Issues/Platform Support sections
- [ ] Updated `../docs/internal/ROADMAP_HONEST.md` if a bucketed item moved (built-not-tested
      -> tested, not-built -> built, etc.)
- [ ] Did not describe untested code as working, and did not use hedge
      words ("planned", "may") for something that is simply not built
