# Third-party notices

## Excalifont

The Excalifont subsets in `assets/fonts/` originate from the local reference checkout at `excalidraw/packages/excalidraw/fonts/Excalifont/`. The WOFF2 subsets were decoded into TrueType files without changing outlines, metrics, or family names, for use by native Fontconfig/Pango.

Copyright (c) 2024 by Excalidraw. All rights reserved.

Designers: Your Own Font Foundry (Virgil); Ján Filípek / DizajnDesign (Excalifont, modifications).

Licensed under the SIL Open Font License, Version 1.1. The full license and copyright notice are included in [assets/fonts/OFL.txt](assets/fonts/OFL.txt). Excalifont is a trademark of Excalidraw.

## Reference implementation

The local `excalidraw/` clone is a read-only reference, excluded from Git files and normal build dependencies. The shape bridge adapts its shape/arrowhead/options code, the freehand generator is bundled from `packages/laser-pointer/src`, and the color transform follows `packages/common/src/colors.ts`. Excalidraw code is MIT licensed; full notices are retained in [assets/vendor/excalidraw-LICENSE.txt](assets/vendor/excalidraw-LICENSE.txt) and [assets/vendor/laser-pointer-LICENSE.txt](assets/vendor/laser-pointer-LICENSE.txt).

## RoughJS 4.6.4

The unchanged upstream CommonJS bundle is embedded to execute the same seeded geometry algorithms as the reference. Copyright Preet Shihn. MIT license: [assets/vendor/roughjs-LICENSE.txt](assets/vendor/roughjs-LICENSE.txt).

## perfect-freehand 1.2.0

The unchanged upstream CommonJS bundle is used for the reference's variable-width freehand mode. Copyright Steve Ruiz. MIT license: [assets/vendor/perfect-freehand-LICENSE.txt](assets/vendor/perfect-freehand-LICENSE.txt).

Bundle source URLs, pinned versions, reference commit, and SHA-256 hashes are recorded in [assets/vendor/provenance.json](assets/vendor/provenance.json). These runtime assets are embedded in the native binary through rquickjs/QuickJS; the Excalidraw web application is not embedded.
