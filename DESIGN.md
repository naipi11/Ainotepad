---
name: Ainotepad
description: Native Windows writing instrument with a theme-responsive Paper Cut shell and editor page.
colors:
  base: "#101113"
  raised: "#17191D"
  hover: "#232730"
  selected: "#1D222B"
  text: "#E7E9ED"
  muted: "#9BA1AA"
  rule: "#30343B"
  light-base: "#F7F7F8"
  light-raised: "#FFFFFF"
  light-text: "#1C1D20"
  light-rule: "#D7DAE0"
  focus: "#3B82F6"
  ghost: "#5BD68B"
  editor-divider: "rgba(122, 122, 122, 0.43)"
typography:
  editor:
    fontFamily: "YaHei, SimHei, Consolas, Cascadia Mono, Segoe UI"
    fontSize: "14px"
rounded:
  control: "3px"
  profile-rail: "4px"
spacing:
  menu-rail: "36px"
  tab-rail: "34px"
  status-rail: "26px"
  settings-footer: "46px"
  editor-gutter: "34px"
components:
  menu-rail:
    backgroundColor: "{colors.base}"
    height: "{spacing.menu-rail}"
  tab-rail:
    backgroundColor: "{colors.base}"
    height: "{spacing.tab-rail}"
  status-rail:
    backgroundColor: "{colors.base}"
    height: "{spacing.status-rail}"
  active-tab:
    backgroundColor: "{colors.selected}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
  settings-window:
    backgroundColor: "{colors.raised}"
    rounded: "{rounded.control}"
---

# Design System: Ainotepad

## Overview

**Creative North Star: "Paper Cut"**

Paper Cut treats Ainotepad as a precise native Windows writing instrument: a stable rail-and-page structure follows the selected light or dark theme family while preserving the same hierarchy, spacing, and semantic colors. The shell is quiet by design so the document remains the visual subject.

Meaning is intentionally scarce. Focus and navigation use blue, an available completion uses green, and the gutter uses a quiet neutral-gray divider with no semantic meaning. Simplified Chinese and English are complete interface modes, while product and protocol terms stay literal where recognition matters.

**Key Characteristics:**

- Native Windows frame and title controls; no simulated title bar.
- Dense horizontal rails around one full-height editor, never an IDE workbench.
- Flat tonal layering, hairline separation, and compact controls.
- Theme-responsive chrome and page inside a structurally stable shell.
- Visible keyboard focus, caret-anchored IME composition, and preview-only completion.

## Colors

The shell uses either a near-black neutral family or a crisp light neutral family; its two semantic accents are reserved for distinct, non-overlapping jobs.

### Primary

- **Focus Blue** (`focus`): active-tab rule, keyboard focus, and focused-control stroke.

### Secondary

- **Ghost Green** (`ghost`): inline completion preview and completion-ready status only.

### Neutral

- **Matte Frame** (`base`): the menu, tab, status, and surrounding shell.
- **Raised Tool Surface** (`raised`): menus, Settings, Find/Replace, About, and shortcut help.
- **Light Frame** (`light-base`, `light-raised`): White and macOS Light chrome and tool surfaces.
- **Light Chrome Text** (`light-text`, `light-rule`): readable text and separators on light chrome.
- **Hover and Selection** (`hover`, `selected`): restrained navigation and control-state fills.
- **Chrome Text** (`text`, `muted`): primary and secondary shell hierarchy.
- **Hairline Rule** (`rule`): one-pixel separation between shell regions and controls.
- **Editor Divider** (`editor-divider`): a translucent neutral gray separating line numbers from text without becoming decoration.

**The Two-Signal Rule.** Blue means focus or navigation; green means a suggested continuation. The gutter divider stays neutral and carries no state.

White and macOS Light select the light shell family. Black Green, VS Code Dark, Dark, Lamp paper, and Custom retain the dark shell family. High Contrast uses its explicit accessible override.

## Typography

The default editor face is YaHei at 14 px. A user may select YaHei, SimHei, Consolas, Cascadia Mono, or Segoe UI and set editor size from 10 to 28 px. The runtime installs the available Windows fonts into egui's proportional and monospace fallback paths, preserving Chinese coverage alongside Latin and code glyphs.

Chrome inherits the native egui/Windows fallback path rather than introducing a display face. Typography remains utilitarian: compact labels, ordinary sentence case, and text sized for editing rather than decoration.

### Hierarchy

- **Editor** (default 14 px; user range 10–28 px): document content, line numbers, syntax, caret, selected text, IME preedit, and ghost continuation.
- **Section heading** (strong; shell `focus`): Settings section title with a smaller muted explanatory detail.
- **Chrome label** (compact default UI text): menus, tabs, status items, buttons, and field labels.
- **Secondary label** (small; shell `muted`): inactive tabs, helper copy, and non-ready status.

**The Reading-First Rule.** Do not use decorative display faces, Inter, or monospace as chrome costume; font choice must support the document and Windows CJK fallback behavior.

## Layout

The native Windows title bar remains outside the application shell. Ainotepad opens at 1100 × 720 px, then the shell stacks a 36 px menu rail, a 34 px tab rail, the remaining editor area, and a 26 px status rail. Find/Replace appears as an inline raised rail above the editor rather than reducing the app to a floating-card composition.

The editor owns all remaining space. Its compact 34 px gutter right-aligns line numbers 6 px before a translucent one-pixel gray divider. Document text, the initial caret, selections, IME anchoring, syntax colors, and ghost text begin 4 px after that divider.

Settings opens centered. Its initial size is viewport-relative and clamped to 360–860 px wide and 300–680 px high; it maintains a 46 px footer for Save and Close while the selected Profiles, Appearance, or Status Bar section scrolls above it. The Profiles section may use a compact internal profile rail; it is never a permanent application sidebar.

## Elevation & Depth

Paper Cut uses no app-defined shadows. Depth comes from tonal layering—`base` for fixed rails, `raised` for transient tools, and `selected` for active navigation—plus one-pixel rules. Windows may provide native frame depth outside the client surface; the app itself stays flat.

**The Flat Instrument Rule.** A surface earns a raised fill only when it is transient or interactive. Do not add glass, blur, drop shadows, or stacked card layers to manufacture depth.

## Shapes

Rails and the editor are square-edged planes. Interactive controls use a restrained 3 px corner radius; profile-rail items use 4 px. The selected tab and neutral editor divider are deliberately square-ended: the active tab rule is 2 px, while divider and separation rules are 1 px.

## Components

### Native Window Shell

The standard Windows title controls and resize behavior remain native. The client shell begins below that frame and uses the same Paper Cut rails in a light or dark neutral family.

### Menu Rail

Menus sit in the compact `base` rail with 8 px horizontal and 3 px vertical button padding. Open menus use `raised`; hover and focus are visible without introducing a second navigation system. Enter continues to insert a newline in the editor and never activates File.

### Tabs

Tabs sit in the 34 px rail with 6 px item gaps. The active tab has `selected` fill, primary text, and a 2 px bottom rule in `focus`; inactive tabs use muted text. Dirty state is the existing `*`, and close remains an adjacent compact native control.

### Editor

The full editor uses the selected editor theme for its page and syntax palette. It retains compact right-aligned line numbers, a neutral gray divider, a quiet theme-relative current-line fill, selection, caret, and caret-anchored IME composition. Ghost text is rendered at the caret in a theme-adjusted green with no glow and never becomes document content until Tab accepts it; Esc dismisses it.

### Status Rail

The 26 px `base` rail uses short separators and muted text for configured cursor, encoding, newline, language, profile/model, message, and custom items. The language item is an interactive Ainotepad document-type chip: new documents start at Markdown, while the popup groups text formats and programming languages. Only a suggested completion may use `ghost`.

### Find/Replace and Tool Windows

Find/Replace is an inline `raised` rail with hairline boundaries. Settings, About, and shortcut help use the same raised tool-surface grammar. Settings keeps compact section selectors, ordinary form rows, visible focus, and its fixed Save/Close footer rather than card stacks. Sliders use a visible neutral track and blue value fill in both shell families.

## Do's and Don'ts

### Do:

- **Do** keep the Paper Cut structure stable while light themes use light chrome and dark themes use dark chrome.
- **Do** reserve blue for focus and green for completion; keep the gutter divider neutral.
- **Do** preserve the native Windows title bar, compact rails, full editor plane, and fixed Settings footer.
- **Do** source all static UI text from the Simplified Chinese/English catalog; keep brand names, model IDs, URLs, encodings, API protocol names, and keyboard chords unchanged.
- **Do** keep focus visible and preserve Enter, Tab, Esc, and IME semantics.

### Don't:

- **Don't** add gradients, glass blur, purple shell accents, neon glow, or thick colored borders.
- **Don't** turn settings or editing tools into stacked cards, a permanent IDE sidebar, a file tree, or an activity rail.
- **Don't** copy Helix or Notepad++ source, or imitate either product's source-oriented chrome.
- **Don't** replace the one green inline suggestion with a completion list or chat sidebar.
