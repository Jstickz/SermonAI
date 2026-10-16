# M3 projector checklist — staging slot and renderer, by eye

**What this verifies:** M3 deliverables 1 to 5 — a verse travels from a
card to the staged slot to the projector with one keypress, in each of the
three layouts, with the translation label where the operator decided it goes
(16 Oct 2026: short abbreviation, small, bottom-right corner); with
auto-live on, a confident card reaches the projector with no keypress, with
the indicator showing; verses change by fade, slide or dissolve at the
length you set; and **B** blanks and unblanks instantly with nothing lost.
Run on a second monitor; the projector window is fullscreen and frameless,
so keep the operator window on the primary.

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

## Auto-live (deliverable 2)

Off by default. The rule: a **confirmed** card, **above 90%**, **with cached
text**, goes straight to the projector. Everything else still waits as a card.

18. Live tab, Staging panel header: the **Auto-live** switch is off, and the
    top bar shows no auto-live chip. Settings → General → Behaviour shows
    the same switch, off, under "Auto-live detected verses".
19. Turn it on from either place. Expected: the other switch follows, and an
    amber **Auto-live on** chip appears in the top bar. Switch to Library,
    Settings, back to Live: the chip is on every tab.
20. With transcription running and the test sermon playing, wait for the
    next direct reference (an R card, 97%). Expected: the projector shows
    it with no keypress; the card still appears in the stack, with an amber
    **On projector · auto-live** chip where Stage would be, Reject and Edit
    still offered; the Live slot in the panel matches; "waiting" in the
    stack header does not count it. Press **Enter**: nothing happens for
    that card (it is shown, not waiting).
21. Stage any card by hand so the Staged slot is full, then let the next
    confident card arrive. Expected: it goes live; the Staged slot still
    holds what you staged. (Auto-live also respects a blanked projector:
    the verse goes live underneath and unblank shows it. There is no blank
    control until deliverable 5, so that line is checked then.)
22. A **V** or **A** card (semantic or paraphrase) arrives at or below 90%.
    Expected: it waits as a normal card with Stage. Quit and relaunch with
    auto-live on: the chip is there on launch and the switch shows on.
    Turn it off: the chip goes, both switches show off.

## Transitions (deliverable 4)

Settings → Displays & themes → **Transition**, below Projector layout.
Default is **Fade**, 400 ms. Keep cards arriving, or edit a card to any
reference and send it live each time.

23. With Fade at 400 ms, send a verse live over another. Expected: the old
    verse fades out, the screen rests on canvas for a blink, the new verse
    fades in. Nothing slides. The reference and rule arrive with the verse,
    not after it.
24. **Slide.** Expected: the old verse lifts up and away as the new one
    rises into place from slightly below; the two overlap. **Dissolve.**
    Expected: the two cross-fade at once, no rest on canvas.
25. Set the length to **0** and press Enter in the field: the next go-live
    is instant, no movement at all. Set **1500**: slow, and still finishes.
    Type **2000**: the field refuses it with a message and goes back to the
    last value. **Use Default** returns to 400. Quit and relaunch: the
    transition and length you left are in effect.
26. Change the projector layout while a verse is live (steps 12 to 14).
    Expected: the reference, rule and corner label appear and vanish with no
    transition; the verse does not animate. Send the **same** verse live
    twice (stage it, Go Live, stage it again, Go Live): it animates both
    times.

## Blank (deliverable 5)

27. With a verse live, press **B** on the Live tab (click somewhere neutral
    first). Expected: the projector goes to the dark canvas colour at once,
    no fade; the top bar's Blank button turns amber and reads "Blanked";
    the Staging panel shows "Projector blanked"; the Live slot still shows
    the verse. Press **B** again: the verse is back at once, no fade, no
    re-animation, exactly where it was. Click the top bar button: same as
    the key. Switch to Settings and press **B**: it still works. Click into
    any text field and type **b**: the letter is typed, nothing blanks.
28. **The DoD line.** Blank, wait a full 60 seconds with transcription
    running, unblank. Expected: the same verse, instantly, no flicker, and
    the staged slot (if you had one) is untouched. While blanked, let an
    auto-live card arrive or Go Live by hand: the projector stays dark, the
    Live slot shows the new verse, and unblank shows it.

## Next and previous verse (deliverable 6)

29. With a verse live, press **→** (click somewhere neutral first).
    Expected: the projector shows the next verse of the same chapter, in
    the same translation, with the configured transition; the Live slot
    follows; the Staged slot is untouched. Press **←** twice: back past
    where you started. The two buttons under Go Live do the same.
30. **The DoD line.** Edit a card to **Psalm 23:1** and send it live, then
    step forward with **→** to Psalm 23:6 and on into Psalm 24, then back
    with **←** to 23:1, keyboard only. Expected: every verse appears, the
    chapter boundary is crossed both ways, nothing is skipped or repeated.
    Then edit a card to **Jude 1:25**, send it live, press **→**: the
    projector keeps Jude 1:25 and a message under the buttons says it is
    the last verse of Jude. Blank with **B**, press **→**: the projector
    unblanks on the next verse. Click into a text field and press **→**:
    the caret moves, nothing steps.

## Pinned cards (deliverable 10)

31. With cards arriving, click **Pin** on one. Expected: it moves into a
    **Pinned** section above Detections, with an accent frame and a
    "Pinned · 1" chip; the waiting count drops by one; it is still there
    after 30 seconds. Pin a second card: it is "Pinned · 2", under the
    first. Press **P** with a waiting card at the top: that card is pinned
    as 3.
32. Press **Enter** with pinned cards and no waiting ones: nothing happens.
    With a waiting card: Enter stages the waiting card, never a pinned one.
    Press **1**: pinned card 1 goes straight to the projector, with no stop
    in the staged slot, and stays pinned. Click its **Stage** button: it
    appears in the Staged slot and the card stays. Press **1** again later:
    it goes live again. Click into any text field and type **p** or **1**:
    the letter is typed, nothing pins or goes live.
33. Click **Unpin** on card 2: it returns to the waiting cards and leaves
    after 30 seconds; the remaining pinned cards keep their numbers in pin
    order. Switch Transcribe off and on again: the pinned section is empty,
    since pins last for the service. In the log, each pin and unpin appears
    as `operator pinned a detection … pinned=true|false`.

## Command palette (deliverable 7)

34. On any tab press **Ctrl+K**. Expected: the palette opens over the app
    with the field focused, the current translation's code beside it, and
    "Type a reference…" below. Press **Escape**: it closes and focus is
    back where it was. The **Search Verse · Ctrl+K** button in the Staging
    header opens it too.
35. Open it and type **john 3 16** (no colon). Expected: within a beat a
    **Typed** row shows "John 3:16" with the start of the verse. Press
    **Enter**: the palette closes and John 3:16 is in the Staged slot.
    Open it again, type **rom 8 28**, press **Shift+Enter**: the palette
    closes and Romans 8:28 is on the projector, Staged empty. Type
    **jon 3 16** (misspelt): a message says it is not a reference and
    suggests the form. Type **jude 25**: Jude 1:25.
36. Open it with nothing typed. Expected: a **Recent** group lists John
    3:16 and Romans 8:28, newest first, and ↑↓ move the highlight; Enter
    on one stages it. Type a reference, press **Tab**: the code beside the
    field changes to the next bundled translation (KJV → WEB → ASV …) and
    the Typed row's text changes with it; the Live tab's translation
    picker shows the same. Click into a card's Edit field and press
    Ctrl+K: the palette still opens (it is a chord, not a letter).

## Manual override (deliverable 8)

37. On a waiting card click **Edit**. Expected: an inline form with the
    reference and a translation box showing the card's translation; the
    box lists the bundled translations (KJV, WEB, ASV). Change the
    reference to **Psalm 23:1**, leave the translation, click **Use
    Reference**: the card becomes "Psalms 23:1", marked **M** (edited by
    operator), 100%, with the KJV text, in the same place in the stack.
38. **Edit** again, keep the reference, pick **WEB**, Use Reference.
    Expected: the same card now shows the WEB text and "WEB" beside the
    reference; the Live tab's translation picker still says the service
    default (KJV), and the next card to arrive is in KJV. Stage this card
    and Go Live: the projector's corner label reads WEB (with the third
    layout on). **Edit**, type **jon 3 16**, Use Reference: a message above
    the stack says it is not a reference; the card is unchanged.

## What to send me

- The log lines `went live … staged_to_broadcast_us=` and
  `projector painted the live verse staged_to_paint_ms=` for a few go-lives.
  The DoD line is **under 100 ms** to the paint; the first figure is our
  share of it.
- Anything in steps 6, 9, 14, 20, 23, 27, 30, 32, 35 or 38 that did not look as described.
- A `projector painted the live verse` line from step 25 at 0 ms and one
  at 1500 ms: `staged_to_paint_ms` should be the same either way, since it
  measures the first frame, and `transition_settles_after_ms` should read 0
  and 1750 (fade: 250 out, then 1500 in).
- For auto-live, the log line `went live by auto-live … staged_to_broadcast_us=`
  followed by its `projector painted the live verse staged_to_paint_ms=`.
