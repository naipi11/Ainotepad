# Aitext showcase media

The Paper Cut base scene was generated with the built-in image generation tool and refined from the approved `aitext-hero.png` reference. The prompt required a matte black/white paper split, a compact editor instrument, blue focus, green ghost text, a neutral gray divider, no orange line, and no watermark. The final editor text is intentionally explicit rather than abstract: each language variant shows a natural-language ghost continuation and a Python `print("Hello, World!")` completion.

- `aitext-paper-cut-base.png`: generated base scene kept for provenance.
- `hero-en.png`: English Paper Cut hero with a short readable sentence suggestion and `print("Hello, World!")` inside the editor.
- `hero-zh-CN.png`: Simplified Chinese Paper Cut hero with `今天的天气` plus a green continuation and the same Python completion.
- `demo-en.gif`: 32-frame, 8 fps English editor showcase where the sentence ghost text appears first and `"Hello, World!")` then completes after the typed `print(` prefix, 1200×675.
- `demo-zh-CN.gif`: 28-frame, 8 fps Simplified Chinese editor showcase with the same two-stage sentence and code completion flow, 1200×675.

The animated frames are illustrative showcase material, not a live API capture. The green continuation communicates the product interaction while avoiding a claim about a specific model response. Bitmap backgrounds were generated with the built-in imagegen tool; the editor text was then redrawn with one font, size, and baseline per language, and the character-by-character reveal was assembled locally with ffmpeg so README text stays aligned, legible, and deterministic.
