# DDLC Plus vs KeyGen launcher comparison audit

Date: 2026-08-12
Reference: installed `Doki Doki Literature Club Plus.app` plus the recovered
Unity `LauncherMainCanvas.prefab` / `LauncherScene.unity`.  Package under
test: `/tmp/keygen-ddlc-rebuild`.

## Finding

The current KeyGen package is closer in the final desktop wallpaper and menu
palette, but it is not a one-for-one DDLC Plus launcher. It renders one static
`SceneSpec` at 1920x1080. DDLC Plus is a staged launcher with stateful canvases,
Unity `Image`/`TMP_Text` nodes, a selected-row atlas crop, and native-window
chrome behavior.

## Mismatch matrix

| Area | DDLC Plus evidence | Current KeyGen | Impact |
| --- | --- | --- | --- |
| Boot sequence | `BootupCanvas` is a separate active state in the launcher scene; BIOS text, MES logo, and `boot.ogg` are distinct assets. | No boot state in the packaged launcher; it jumps directly to the desktop wallpaper/menu. | User never sees DDLC's boot/login progression. |
| Login stage | `LoginCanvas` and a 300x300 user-icon source are present; it transitions into the desktop. | Login is absent. | Missing an entire visible stage and transition. |
| Desktop chrome | `DesktopCanvas` contains the start-menu button, desktop box, taskbar/time and app surfaces. | Only wallpaper, panel, text, and seven icons. | The desktop feels like a bare image instead of a launcher desktop. |
| Start-menu panel | Real 872x1267 sprite displayed as 436x633 with a separate drop shadow. | Real panel is present at the right size; no separate shadow node. | Geometry is close, but depth/edge treatment differs. |
| Menu rows | Eight 436x73 rows. Text is a separate 300x73 TMP region; icons are separate 85px sprites. | Text is drawn by generic menu rows (52px high + 21px gap) and icons are manually placed. | Text/icon baselines and hit regions are not the prefab's row geometry. |
| Selection | Unity uses `sactx-...png`, cropped to rect `870x145`, scaled to 436x73; every app has a highlighted icon variant. | No selected-row crop. `focused_outline` only changes text outline. | The defining pink selection bar is missing. |
| App surface | DDLC, Side Stories, Files, Mail, Pictures, Music, Settings, Quit. | Labels/icons now exist for all seven app rows plus Quit, but routes are category aliases, not app behaviors. | Visual labels are present; behavior remains approximate. |
| Icon fidelity | Prefab references normal + highlight icon sprites, including exact DDLC/Side Stories/Files GUIDs. | Normal PNGs only. | Focused icons never change. |
| Typography | Prefab uses Vera SDF, 32px for menu labels and a distinct TMP font/material pipeline. | Package selects a raster font fallback (`RifficFree`/first available); no SDF/material/kerning/shadow model. | Letter shapes, weight, spacing, and antialiasing differ. |
| Window presentation | Reference window is 1710x1073 on this display and hides the desktop chrome while active. | Minifb resizable window is 1710x995 with visible macOS title bar and Dock when captured. | App framing is visibly different even when content is aligned. |
| Transitions | Prefab includes `FadeFromBlackImage`, inactive-stage toggles, and launcher controller transitions. | Static `time=0` scene; no boot/login/desktop state machine. | No natural DDLC boot progression. |
| Background treatment | Wallpaper is a stage behind login/desktop; custom wallpaper and fade layers are separate. | Wallpaper is always visible under the menu. | Cannot reproduce state-specific fade/occlusion. |
| Audio | `boot.ogg` is explicitly referenced by BIOS/boot flow. | Full package contains audio records, but launcher scene has no boot audio trigger. | Boot sound behavior is missing. |

## Coordinate observations

The recovered prefab uses bottom-origin Unity `RectTransform` coordinates and
places the start-menu children at local y offsets `-10`, `-83`, `-156`,
`-229`, `-302`, `-375`, `-448`, and `-521`, each with size `436x73`. KeyGen
currently approximates the same visual rhythm with top-origin menu coordinates
`y=505`, `row_height=52`, `spacing=21`; this totals the same 73px step but does
not preserve the row's full 436px interactive surface or its child layout.

## Required implementation order

1. Add a generic staged-launcher model: `boot -> login -> desktop`, with
   explicit layer visibility and transition timing. Keep it title-neutral.
2. Add image source-rectangle cropping for atlas sprites and use the recovered
   selected-row rect. This is needed for the pink focus bar.
3. Add per-row icon variants and a row-level focus state, including the full
   436x73 hit target.
4. Add a desktop/taskbar layer contract and native window policy (content size,
   aspect behavior, title-bar policy) so the app frame is not an accidental
   minifb default.
5. Add launcher audio events and a login/user-icon stage.
6. Rebuild the private DDLC package and compare screenshots at the same
   reference viewport before claiming parity.

The current package is therefore a useful final-desktop approximation, not a
completed DDLC Plus boot-menu reproduction.
