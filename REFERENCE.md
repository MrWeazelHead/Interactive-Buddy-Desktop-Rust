# Interactive Buddy v1.01 reference notes

## Primary source

- Shock-Value's original Newgrounds submission: [Interactive Buddy v.1.01, portal entry 218014](https://www.newgrounds.com/portal/view/218014). Uploaded Feb 8, 2005, credited to shock-value (design) and Matrix on Acid (audio). The author description says the loop is to use weapons to beat up the Buddy and earn money for more weapons; it also says the intentionally simple graphics target about 36 fps.
- The author comment on the page includes dated updates (Feb 9–13, 2005), including a note that v1.01 was submitted and would be released after NG review. This is a reliable contemporary source for identity and intent, but the text does not enumerate the in-game menus or prices.
- The original discussion linked by the author is [Newgrounds BBS topic 229413](https://www.newgrounds.com/bbs/topic/229413/1); Shock-Value confirms Open Hand is positive/non-damaging. Contemporary reviews also describe mouse use, the fist/flail, radio, and weapon interactions.

## Interface and play loop

**Verified from screenshots attributed to v1.01 / contemporary evidence:** The game is a sparse, gray, fixed arena with the Buddy centered/right-of-center, speech/status bubble, a top menu bar, and a bottom-left cash/current-tool readout. An image accompanying a 2025 retrospective labels the UI version 1.01 and shows the top labels `File`, `Skins`, `Items`, `Modes`, `Settings`, `Help`, version at top right, and a bottom-left balance/tool label. Screenshot source: [Vehe Mently, “Interactive Buddy: A Critical Analysis”](https://medium.com/%40vehemently/interactive-buddy-a-critical-analysis-2761cb794fb4). The original Newgrounds page itself does not expose a textual UI diagram.

**Controls:** Mouse interaction is the supported core: select a tool from menus, then click/drag/throw or use it in the arena. Newgrounds reviewers explicitly discuss cursor-controlled fist/flail and dragging; original author/forum evidence confirms hand interactions. Keyboard cheat sequences reproduced by cheat sites are not treated as normal controls. Exact keyboard bindings for v1.01 remain unverified.

**Core loop:** interact with Buddy to generate money; spend it on more tools, skins, and modes; Buddy reacts through speech/status indications. The Newgrounds Wiki's v1.01 page describes item groups as Bombs, God Powers, Objects, and Hand, and says hand actions can improve the Buddy's state. Treat the group names as secondary-source evidence.

## Extracted catalog and version boundary

The local extracted SWF is **Interactive Buddy v1.02**, and its ActionScript is now the authoritative source for the reconstruction catalog. Exact item names, prices, starting ownership, skin IDs, modes, and menu/category structure extracted from `decomp/scripts/frame_25/DoAction.as` are recorded in [`rewrite/assets/catalog.json`](rewrite/assets/catalog.json). The source definitions are in `defineTheItems()`; runtime menu hierarchy and ordering are built by `compileItems()`, `compileSkins()`, and `compileModes()`. Item groups are Explosives, God Powers, Guns, Hand, Miscellaneous, Objects, and Special. The live Items menu only exposes owned tools under these sorted category submenus, followed by `Buy New Items...`; Skins and Modes similarly end with their respective store action.

This local v1.02 extraction is definitive for the catalog used by the rewrite, but it does not independently establish whether v1.01 had identical prices or inventory. The original Newgrounds v1.01 submission remains the primary historical source; exact v1.01 differences remain unknown without its SWF. The earlier third-party catalog is superseded by the extracted source and should not be used as authority.

The exact extracted data includes initially owned hand tools, Grenades, Weak Gravity Vortex, Baseballs; Buddy skin; FPS Counter and Open Ceiling modes. Purchased items, skins, and modes are initially unowned. Modes include nine toggles and the `Scripting Engine Access...` action.

## Research links

- [Newgrounds original submission](https://www.newgrounds.com/portal/view/218014)
- [Newgrounds Wiki: Interactive Buddy v.1.01](https://newgrounds.wiki.gg/wiki/Interactive_Buddy)
- [Original discussion forum thread](https://www.newgrounds.com/bbs/topic/229413/1)
- [Extracted v1.02 catalog](rewrite/assets/catalog.json)
- [Flash Gaming Wiki catalog (secondary; superseded)](https://flashgaming.fandom.com/wiki/Interactive_Buddy)
- [v1.01-labeled UI screenshot source](https://medium.com/%40vehemently/interactive-buddy-a-critical-analysis-2761cb794fb4)
