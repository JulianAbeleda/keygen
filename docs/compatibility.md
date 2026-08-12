# Compatibility and clean-room policy

## What “Unity compatible” means

KeyGen may eventually read documented Unity-origin project files and translate
supported data into KeyGen-owned project schemas. The KeyGen compiler and
player then operate only on those KeyGen schemas and assets.

It does not mean:

- executing the Unity Editor or Unity Player headlessly;
- linking against Unity runtime assemblies;
- compiling arbitrary Unity C# behavior unchanged;
- reproducing undocumented behavior by embedding recovered source; or
- granting permission to redistribute a project's content.

Each importer version must publish a support matrix. Unknown components,
properties, scripts, shaders, or serialization forms fail with actionable
diagnostics. No silent approximation is labeled compatible.

## Provenance rules

- Implementation comes from KeyGen-owned design, public specifications,
  documented formats, and observable behavior.
- Compatibility fixtures containing third-party content stay local and ignored.
- Golden evidence committed to this repository must be original or
  redistribution-compatible and include provenance.
- Product and engine trademarks remain their owners' property; documentation
  uses names only to describe interoperability.

This engineering boundary is designed to keep KeyGen independent. It is not
legal advice and does not override licenses or law applicable to a contributor
or asset owner.
