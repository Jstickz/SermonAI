# M3 projector checklist — staging slot and renderer, by eye

**What this verifies:** M3 deliverables 1 and 3 — a verse travels from a card
to the staged slot to the projector with one keypress, in each of the three
layouts, with the translation label where the operator decided it goes
(16 Oct 2026: short abbreviation, small, bottom-right corner). Run on a
second monitor; the projector window is fullscreen and frameless, so keep
the operator window on the primary.

**Build:** `npm run build && node scripts/build-app.mjs`, then
`src-tauri\target\release\sermonai.exe`. The log is at
`%LOCALAPPDATA%\SermonAI\logs\sermonai.log`.

## Setup

1. Launch the app. Settings → Displays & themes → pick the second monitor →
   **Use as projector**. The second monitor goes canvas-coloured (dark, not
   black) and stays that way: nothing is live yet.
2. On the same Settings page, below the display cards: **Projector layout**
   shows three choices with **Verse and reference** selected.
3. Live tab. The Staging panel sits under the transcript: two empty slots,
   "Staged" dashed, "Live on projector" with the violet frame, and a greyed
   **Go Live · Enter** button.

## Stage and go live from a card

4. Start transcription on the Speakers loopback and play the test sermon
   (`.\scripts\make-test-sermon.ps1 -Minutes 10`, then
   `Start-Process "$env:TEMP\sermonai-test-sermon.wav"`). Wait for the
   first confirmed card — Jeremiah 29:11 in the synthesised sermon.
5. Click **Stage · ↵** on that card. Expected: the card leaves the stack;
   the **Staged** slot shows "Jeremiah 29:11 · KJV" and the text; the
   projector is still blank; Go Live is now lit.
6. Press **Enter** (click anywhere neutral first so no text field has
   focus). Expected, within a blink: the projector shows the verse in the
   serif face, large, with **JEREMIAH 29:11** under it and a short violet
   rule under that; the **Live** slot in the panel shows the same; the
   Staged slot is empty again; Go Live greys out.
7. Press **Enter** again with nothing staged and no cards. Expected: nothing
   happens. (If a new card has arrived, Enter stages it instead — that is
   the rule: Enter does the next thing in the path.)

## Enter, Shift+Enter, Escape

8. Wait for the next card (Romans 8:28). Press **Enter** once: it is staged,
   not live — the projector still shows Jeremiah. Press **Enter** again: it
   goes live and Jeremiah is replaced.
9. On the next card (1 Thessalonians 5:1), press **Shift+Enter**. Expected:
   it goes straight to the projector in one keystroke; the Staged slot ends
   empty.
10. Stage any card with **Stage · ↵**, then press **Escape**. Expected: the
    Staged slot empties; the projector keeps showing what was live.
11. Click in the card's **Edit** field (or any text input) and press Enter.
    Expected: Enter acts on the field, not on staging.

## The three layouts

Keep a verse live for these. Changes apply to the verse on screen at once.

12. Settings → Displays & themes → Projector layout → **Verse only**.
    Expected: the projector shows the words alone — no reference, no violet
    rule, no corner label.
13. **Verse and reference.** Expected: the reference returns under the verse
    with the violet rule. No corner label.
14. **Verse, reference and translation.** Expected: as 13, plus **KJV** in
    small capitals in the bottom-right corner, muted, monospaced — small
    enough to read from the booth and ignore from a pew. Nothing else
    changes size or position.
15. Quit and relaunch. Expected: the layout you left it on is the one in
    effect, and the setting shows it.

## Length, blank, and the stage monitor

16. Edit a card to **Psalm 119:105** (short) and send it live, then edit one
    to **Esther 8:9** (the longest verse in the KJV) and send that live.
    Expected: the short verse is large; the long one steps down to a
    smaller size and fits the screen without scrolling or clipping. This is
    the length-tier fit; deliverable 9 replaces it with a measured one.
17. If you also assign the stage monitor (Use for stage), it shows the live
    verse large and the staged one small, as before — unchanged by this
    work.

## What to send me

- The log lines `went live … staged_to_broadcast_us=` and
  `projector painted the live verse staged_to_paint_ms=` for a few go-lives.
  The DoD line is **under 100 ms** to the paint; the first figure is our
  share of it.
- Anything in steps 6, 9 or 14 that did not look as described.
