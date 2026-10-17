# M3 projector checklist — staging slot and renderer, by eye

**What this verifies:** every M3 deliverable, 1 to 10 — a verse travels
from a card to the staged slot to the projector with one keypress, in each
of the three layouts, with the translation label where the operator decided
it goes (16 Oct 2026: short abbreviation, small, bottom-right corner); with
auto-live on, a confident card reaches the projector with no keypress, with
the indicator showing; verses change by fade, slide or dissolve at the
length you set; **B** blanks and unblanks instantly with nothing lost;
arrows step the passage; pinned cards, the palette, Edit's translation
choice and the measured fit each have their steps. Steps 1–27 passed on
f261934 on 16 Oct; 28–39 passed on 8d09a9b on 17 Oct, nothing failing to
match. The sheet stays as the regression run for later M3 changes. Run on a second monitor;
the projector window is fullscreen and frameless, so keep the operator
window on the primary.

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

31. *(revised 17 Oct)* With cards arriving, click **Pin** on one. Expected:
    the card leaves the waiting cards and a **Pinned** strip appears above
    Detections — a short fixed-height row — holding it as **1** with its
    reference, Go Live · 1 and Unpin; the waiting count drops by one; it is
    still there after 30 seconds. Pin more: they line up as 2, 3 … and the
    strip scrolls sideways once it is full; the Detections area below keeps
    its height. Press **P** with a waiting card at the top: that card is
    pinned next.
32. Press **Enter** with pins and no waiting cards: nothing happens. With a
    waiting card: Enter stages the waiting card, never a pin. Press **1**:
    pin 1 goes straight to the projector, with no stop in the staged slot,
    and stays pinned. Press **1** again later: it goes live again. Click
    into any text field and type **p** or **1**: the letter is typed,
    nothing pins or goes live. A pin has no Stage button.
33. Click **Unpin** on pin 2: it leaves the strip; the others keep their
    numbers in pin order. Switch Transcribe off and on again: the cards
    clear, **the pins stay**. Click **Clear all** in the Pinned header: it
    asks "Clear N?"; **Keep** leaves them; **Clear** empties the strip. In
    the log each pin and unpin from a card appears as `operator pinned a
    detection … pinned=true|false`.

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

## Auto-fit (deliverable 9)

39. Send **Psalm 119:105** live, then **Esther 8:9**, then **John 11:35**.
    Expected: each fills the screen as large as it can while the whole
    block — verse, reference, rule — stays inside the margins with nothing
    clipped and no scrollbar; the short verses are much larger than the
    long one, and John 11:35 ("Jesus wept.") is at the ceiling, not larger.
    With Esther 8:9 live, drag the projector to the portrait display (or
    assign it there): it refits for the new shape on its own. The paint
    report lines should look the same as before: the first frame is
    already at the fitted size.

## Polish, 17 Oct

40. **Impossible references are refused everywhere.** Open the palette
    (Ctrl+K) and type **john 5 150**. Expected: no Typed row; the message
    "John 5 has 47 verses, not 150"; Enter does nothing. Type **jude 2 1**:
    "Jude has one chapter, not 2". Type **psalm 151 1**: "Psalms has 150
    chapters, not 151". Type **john 5 47**: a Typed row; Enter stages it.
    On a card click **Edit**, type **John 5:150**, Use Reference: the
    message above the stack names the count and the card is unchanged.
    With John 5:47 live, press **→**: John 6:1 appears, since that
    boundary is real; press **←** back to 5:47. With Revelation 22:21
    live, press **→**: refused, the last verse of Revelation. Nothing in
    this step changes the projector except the real verses.

41. **Pin from a slot.** Stage a verse by any route. Expected: a small
    **Pin** switch in the Staged slot's header, off. Switch it on: the verse
    appears in the Pinned strip with the next number; the switch reads on.
    Send it live: the Live slot's switch is on for the same verse. Switch
    it off from either slot: the pin leaves the strip. Pin a verse that is
    already pinned from a card: nothing doubles.

42. **Staging panel, 1366 × 768.** With the window at 1366 × 768 (or a 13"
    laptop), Live tab, a verse staged and another live: both slots show the
    reference and three lines of text without the panel scrolling; one row
    under them holds **← Prev**, **Go Live · ↵** (the wide one) and
    **Next →**; the transcript above shows at least eight lines. Nothing
    on the Live tab needs the window to scroll.

44. **Closing closes.** With the projector assigned and a verse live, close
    the operator window with its ✕. Expected: the projector window goes
    too and `sermonai.exe` is gone from Task Manager within a second or
    two; the log ends with "operator window closed; closing the app".

43. **Transcript scrolling.** With transcription running and a few
    paragraphs on screen, scroll up. Expected: the view stays where you put
    it, nothing pops up over the text, and the **↓ Latest** control in the
    panel header turns amber; the footer reads "Scrolled back". Click
    **↓ Latest**: the newest line scrolls into view and the control goes
    quiet again. Scroll to the bottom by hand: it goes quiet too.

45. **Density at 1366 × 768.** Window at 1366 × 768, Live tab, three cards
    waiting and two pins. Expected: the pinned strip, three full cards with
    their buttons, the transcript with eight or more lines, and the
    staging panel with both slots are all on screen at once with no
    window scrollbar; the card buttons are small pills you can still hit
    without care; Go Live, Transcribe and Blank are the big ones. Press
    **Aa** three times: the transcript steps small, medium, large and back.

## What to send me

- The log lines `went live … staged_to_broadcast_us=` and
  `projector painted the live verse staged_to_paint_ms=` for a few go-lives.
  The DoD line is **under 100 ms** to the paint; the first figure is our
  share of it.
- Anything in steps 6, 9, 14, 20, 23, 27, 30, 32, 35, 38 or 39 that did not look as described.
- A `projector painted the live verse` line from step 25 at 0 ms and one
  at 1500 ms: `staged_to_paint_ms` should be the same either way, since it
  measures the first frame, and `transition_settles_after_ms` should read 0
  and 1750 (fade: 250 out, then 1500 in).
- For auto-live, the log line `went live by auto-live … staged_to_broadcast_us=`
  followed by its `projector painted the live verse staged_to_paint_ms=`.
