# Installing SermonAI

SermonAI is in development and is shared directly with a small group of
churches. These builds are **not code signed yet**, so Windows and macOS will
both warn you the first time you open the installer. That is expected, and this
page shows exactly what you will see and what to click.

Signing is planned before public launch, and the warnings disappear at that
point. Nothing else about the app changes.

> Only install SermonAI from a link sent to you directly by the SermonAI team.
> Because these builds are unsigned, your computer cannot verify who made them —
> so the trust has to come from where you got the file.

---

## Windows

**You need:** Windows 10 (version 1909 or newer) or Windows 11. No administrator
rights, and no reboot.

1. Download `SermonAI_x.y.z_x64-setup.exe`.
2. Double-click it. Windows shows a blue box: **"Windows protected your PC"**.

   <!-- screenshot: SmartScreen dialog, initial state -->

3. This is Microsoft SmartScreen. It appears because the installer is not
   signed, not because anything is wrong with the file.
4. Click **More info** — the small link under the message. Do **not** click
   "Don't run".

   <!-- screenshot: SmartScreen dialog after clicking More info, showing the Run anyway button -->

5. Click **Run anyway**.
6. The installer runs and finishes in under a minute. SermonAI opens.

If your browser also warns you while downloading ("this file isn't commonly
downloaded"), choose **Keep** — the same reason applies.

### If you do not see "More info"

Some managed or work laptops have SmartScreen set to block without an override.
In that case:

1. Right-click the downloaded `.exe` and choose **Properties**.
2. At the bottom of the **General** tab, tick **Unblock**, then **OK**.
3. Double-click the installer again.

If it is still blocked, your organisation's IT policy is preventing it. Contact
the SermonAI team rather than changing security settings.

---

## macOS

**You need:** macOS 12 or newer, on either an Intel or Apple Silicon Mac.

1. Download the `.dmg` file and open it.
2. Drag **SermonAI** into your **Applications** folder.
3. Open Applications, then **right-click** (or Control-click) SermonAI and
   choose **Open**. This step matters: double-clicking gives you a dialog with
   no way to proceed.

   <!-- screenshot: right-click context menu in Applications with Open highlighted -->

4. macOS asks whether you are sure, because the app is from an unidentified
   developer.

   <!-- screenshot: Gatekeeper "unidentified developer" dialog with the Open button -->

5. Click **Open**. SermonAI starts.

You only do this once. After the first launch, SermonAI opens normally from the
Dock, Launchpad, or Spotlight.

### If macOS says the app "is damaged and can't be opened"

This message is misleading — the app is fine. macOS shows it when a downloaded
file is quarantined. Open Terminal and run:

```bash
xattr -dr com.apple.quarantine /Applications/SermonAI.app
```

Then open the app normally. If that does not work, contact the SermonAI team
before changing any other security settings.

---

## What the warnings actually mean

| | Signed app (later) | SermonAI today |
|---|---|---|
| Who made it | Verified by Microsoft / Apple | Not verified by the OS |
| File tampered with in transit | Detected | Not detected |
| Malware scanning | Same either way | Same either way |

Code signing proves the app came from a known publisher and has not been altered
since. Without it, your computer can confirm neither — which is why it asks you
to make the call.

That is a reasonable question for it to ask. Only install builds that came to
you directly from the SermonAI team, and if a build ever arrives from somewhere
else, do not install it.

---

## Updates

SermonAI updates itself. Updates are cryptographically signed with SermonAI's
own updater key and are rejected if the signature does not match, so the update
path is protected even while the installer is unsigned.

Updates are small — usually a few MB — and never re-download the packs you have
already installed.

---

## Uninstalling

- **Windows:** Settings → Apps → Installed apps → SermonAI → Uninstall.
- **macOS:** drag SermonAI from Applications to the Trash.

Your sermons, transcripts and summaries are stored separately and are **not**
removed by uninstalling. To delete them too, remove:

- **Windows:** `%APPDATA%\app.sermonai.desktop`
- **macOS:** `~/Library/Application Support/app.sermonai.desktop`

---

## Still stuck?

Tell us which step failed and what the screen said, and include your operating
system version. A photo of the dialog is perfect.
