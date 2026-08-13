# Full package qualification

Run the bounded end-to-end check with:

```sh
python3 scripts/private/qualify-full-package.py
```

When the local recovered ExportedProject exists, this builds the private
package under `/tmp`, audits route/scene/asset/story closure, validates the
project, loads the story entry, renders boot, and packages/smoke-tests the
arm64 macOS app. If the private source is absent, only the repository-safe
audit self-test runs and the result is explicitly reported as skipped.
