# KGD-102: canonical kg_ddlc_plus identities

The independent product uses these namespaces:

| Purpose | Value |
| --- | --- |
| target and display name | `kg_ddlc_plus` |
| application bundle | `kg_ddlc_plus.app` |
| bundle identifier | `com.julian.keygen.kg-ddlc-plus` |
| save namespace | `com.julian.keygen.kg-ddlc-plus` |
| target architecture | `arm64` |

The product must never use the official DDLC Plus bundle, save, or package
identity. Installers may replace only the exact explicit
`/Applications/kg_ddlc_plus.app` target after staged validation. They must not
scan-delete similarly named development bundles.
