# Third-party notices

## Excalifont

The Excalifont subsets in `assets/fonts/` originate from the local reference checkout at `excalidraw/packages/excalidraw/fonts/Excalifont/`. The WOFF2 subsets were decoded into TrueType files without changing outlines, metrics, or family names, for use by native Fontconfig/Pango.

Copyright (c) 2024 by Excalidraw. All rights reserved.

Designers: Your Own Font Foundry (Virgil); Ján Filípek / DizajnDesign (Excalifont, modifications).

Licensed under the SIL Open Font License, Version 1.1. The full license and copyright notice are included in [assets/fonts/OFL.txt](assets/fonts/OFL.txt). Excalifont is a trademark of Excalidraw.

## Reference implementation

The local `excalidraw/` clone is a read-only reference, excluded from normal builds. `crates/rsdraw-core/` adapts its shape/options/arrowhead geometry, constant-width laser outlines, font metrics, text normalization, and color transform. Excalidraw code is MIT licensed; full notices are retained in [assets/vendor/excalidraw-LICENSE.txt](assets/vendor/excalidraw-LICENSE.txt) and [assets/vendor/laser-pointer-LICENSE.txt](assets/vendor/laser-pointer-LICENSE.txt).

## RoughJS 4.6.4

`crates/rsdraw-core/src/rough.rs` ports the seeded rendering algorithms to Rust. Copyright Preet Shihn. MIT license: [assets/vendor/roughjs-LICENSE.txt](assets/vendor/roughjs-LICENSE.txt).

Its curve subdivision/path sampling and hachure scan conversion also adapt points-on-curve 0.2.0, points-on-path 0.2.1, and hachure-fill 0.5.2, all by Preet Shihn under MIT licenses: [points-on-curve](assets/vendor/points-on-curve-LICENSE.txt), [points-on-path](assets/vendor/points-on-path-LICENSE.txt), [hachure-fill](assets/vendor/hachure-fill-LICENSE.txt).

## perfect-freehand 1.2.0

`crates/rsdraw-core/src/variable_freehand.rs` ports the reference's simulated-pressure mode to Rust. Copyright Steve Ruiz. MIT license: [assets/vendor/perfect-freehand-LICENSE.txt](assets/vendor/perfect-freehand-LICENSE.txt).

The unchanged upstream JavaScript bundles remain only as development oracles for generating test fixtures. They are not embedded or executed by either Rust crate. Bundle source URLs, pinned versions, reference commit, and SHA-256 hashes are recorded in [assets/vendor/provenance.json](assets/vendor/provenance.json).
