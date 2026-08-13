# Private package route closure

Run the route audit after `build-full-content-package.py` and before compiling
or packaging. It fails closed when a route has no scene document, a scene
references an absent asset, or a story entry is not declared.

```sh
python3 scripts/private/build-full-content-package.py --source PATH --output /tmp/keygen-content
python3 scripts/private/audit-package-routes.py /tmp/keygen-content
```

The synthetic test is repository-safe:

```sh
python3 scripts/private/audit-package-routes.py --self-test
```
