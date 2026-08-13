# DDLC Plus launcher / boot audit

Source: private AssetRipper export `Assets/GameObject/LauncherMainCanvas.prefab`, `Assets/RenPyParser/Launcher/LauncherScene.unity` (not committed).

## Canonical canvas/layout

The prefab uses a 1920x1080 reference canvas, centered pivot `(0.5,0.5)`. The outer `LauncherMainCanvas` scales to `1.4000043,1.4000043,0.9776`; its top-level child is a 1920x720 canvas anchored top-right. The visible launcher is not a generic centered menu: it is a staged sequence of inactive canvases:

* `BootupCanvas` inactive in prefab; black panel, with content inset 40 px horizontally and 22.4995 px vertically (`sizeDelta -80,-45`).
* `LoginCanvas` inactive; login wallpaper/background stage.
* `DesktopCanvas` inactive; final desktop/start-menu stage.
* `BiosCanvas` inactive; BIOS diagnostic stage.
* `LauncherWallpaper` and `FadeFromBlackImage` are active behind those stages.

## Exact source assets

* Desktop wallpaper: `Assets/Texture2D/gallery_default_wallpaper.png`, 1920x1080, sprite `gallery_default_wallpaper`, GUID `31894ba6486c08f4b9b42ac77a554387`.
* Start menu panel: `Assets/Texture2D/start menu background.png`, 872x1267, sprite `start menu background`, GUID `4192cea6a21386146ae25d8d79d86dde`.
* Start menu selected row: texture `Assets/Texture2D/sactx-2048x4096-Uncompressed-4k - iPad - iPhone-cdc01e52.png`, sprite `start menu highlight`, rect `x=0,y=1503,width=870,height=145`, GUID `ab42403e3d61dfb4ba6744ce7146530a`.
* DDLC app icon: `Assets/Sprite/ddlc icon.asset`, 85x85, GUID `0ed25cd7e6ffe7540bbbe6562f2b1796`; highlighted variant `ddlc icon highlight.asset`, 85x85, GUID `1d396d7731e62664cb4e2578c2eea8b1`.
* Side Stories icon: `side stories icon.asset` GUID `91b50c6580bd75e47be152e4517ff447`; highlight `side stories icon highlight.asset` GUID `debaabb9faa2bab4e866e850f1c867ea`.
* Files icon: `files icon_0.asset` GUID `8eb431de023406440a406c1f74f59d12`; highlight `file icons highlight.asset` GUID `6f976e102c2194646a809c5e389df0aa`.
* User icon: `Assets/Texture2D/user icon.png`, 300x300 source; sprite rect 261.84778 square at `(19.07612,19.07612)`, GUID `a5806aefa896d7440807f9665b2bdc65`.
* BIOS MES logo: `Assets/Texture2D/MES Logo bios.png`, 640x524, sprite GUID `d7827060546b3334c99d77b9d293f194`; compact logo: `MES Logo bios 2.png`, 372x120, sprite rect `372x113.92388` at y=6.07612, GUID `2ee74f0e8f41d5644b8678aa339ea977`.
* BIOS font: `Assets/Font/ModernDOS8x16.ttf`; source boot text: `Assets/TextAsset/bios.txt`; boot audio: `Assets/AudioClip/boot.ogg`.

## Desktop/start-menu geometry

The desktop root is 1920x1080. The visible desktop item/start-menu region is inset from the top-left. In the prefab's 1920x1080 coordinate space:

* Start menu container is anchored lower-left; panel size is 436x633 and anchored at `(0,300)` relative to its parent (the panel is visually bottom-left after Unity's bottom-origin transform).
* Menu rows are 436x73. Row positions are y `-83`, `-10`, `-156`, `-229` for the menu's children (the prefab includes more rows in the same 73-pixel rhythm). Each row has a 300x73 text region and an icon region approximately 38–42 px, offset x≈35–36.
* Start-menu wallpaper/background panel is a real sprite, not a flat color; selected state uses the 870x145 atlas crop scaled into a 436x73 row.
* User/login panel includes a 640x640 image slot and a 300x300 user icon source; do not substitute a plain text login box.

## Why the current KeyGen boot looked drastically different

The existing generic boot package selects `MES Logo bios 2.png`/boot text and renders a generic `SceneSpec`; it does not compose the inactive-stage sequence, 1920x1080 wallpaper, login, desktop, start-menu panel, atlas-selected row, or per-app icon layers above. A one-for-one launcher needs these assets represented as layered image nodes with explicit z-order and 1920x1080 reference coordinates. Scaling must preserve the 1920x1080 design viewport; do not fit the 872x1267 panel as a full-screen menu.

## Recommended package composition

Create a canonical launcher scene with layers in this order: black/fade; `gallery_default_wallpaper`; login wallpaper + user icon; desktop chrome; start-menu panel; selected-row atlas crop; app icons/text; taskbar/clock; transition overlay. Keep boot as a separate scene using `boot.ogg`, `bios.txt`, `MES Logo bios.png`, and ModernDOS. Route boot→login→desktop should toggle stage visibility, not replace the scene with a generic list. This report intentionally makes no engine edits.
