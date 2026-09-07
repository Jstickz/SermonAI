# SermonAI — Product Requirements Document

> **Real-Time Sermon Transcription, Scripture Display & Sermon Intelligence System**
> *Listen. Detect. Project. Transcribe. Summarize.*

| Field | Value |
|---|---|
| **Document Type** | Product Requirements Document (PRD) |
| **Product Name** | SermonAI |
| **Version** | 2.1 |
| **Status** | Active. Development Reference |
| **Last Updated** | 7 September 2026 |
| **Companion file** | `docs/MILESTONES.md` (stage tracker) |
| **Purpose** | Source of truth for AI-assisted development (Claude Code) |

---

## 0. What Changed (Read This First)

### v2.1 (7 September 2026, same day as v2.0)

Two founder requirements were added after v2.0: the app must be **compatible with Windows and macOS** as equals, and it must be **lightweight and fast to download and install**. The second requirement is incompatible with the Electron + bundled Python stack in v2.0 (a 350 to 450 MB base installer, 2 to 3 GB with offline models). v2.1 therefore changes the technology stack and adds an installation footprint requirement group.

| Area | Change |
|---|---|
| Technology stack (§11) | **Electron + Python replaced by Tauri 2 + Rust.** Frontend stays React + TypeScript. One compiled binary per platform, no sidecar process. |
| Installation footprint (§9.7, §8.11) | New non-functional targets: base installer under 40 MB, install under 60 seconds, no admin rights, cold start under 1 second. New requirements FR-59 to FR-63 for on-demand packs and delta updates. |
| Architecture (§10) | Process topology simplified to one process. FAISS replaced with an in-binary vector search. Vosk replaced with whisper.cpp via `whisper-rs`. Offline summary defaults to a template based renderer, with the local LLM as an optional pack. |
| Data model (§14.3) | Pack directory layout added. |
| Roadmap (§20) | Phase 0 now includes the Tauri scaffold and pack downloader. Milestone level tracking moved to `MILESTONES.md`. |
| Risks (§22) | Rust learning curve and WebView2 availability added. |

### v2.0 (7 September 2026)

This revision follows a fresh competitive review of **TajiCast v2.0** (tajicast.com) and **PewBeam** (pewbeam.com) as of September 2026, plus one new founder requirement. The market moved. Two things matter most:

1. **TajiCast is now free, fully offline, Windows only, supports any translation via import, has a phone remote, keeps full transcripts, and generates structured sermon notes.** It has 1,000+ installs across Kenya, Nigeria, Ghana, Rwanda, USA and Poland. Several of SermonAI's original v1.0 differentiators (translation breadth, offline mode, transcript archive) are no longer unique.
2. **PewBeam has expanded into a "presentation agent"**: slides, AI slide generation, lyrics, themes, Main + Alternate output, three devices per license, at $0 / $14 / $30 per month.

**New founder requirement added in this revision:** after every sermon, the app must automatically produce a **detailed summary of the day's preaching** and save it as a **PDF that can be downloaded at any time** from within the app. This is now a core, non-negotiable v1.0 feature (see FR-40 to FR-46 and Section 13.11).

### Summary of changes

| Area | Change |
|---|---|
| Competitive landscape (§5) | Rewritten. Now compares TajiCast, PewBeam and SermonAI side by side. |
| Strategic positioning (§5.3) | Reframed. SermonAI competes on **sermon intelligence and cross-platform reach**, not on being the only offline or multi-translation option. |
| Functional requirements (§8) | 19 new requirements added: FR-40 to FR-58. Covers post-sermon summary PDF, phone remote, staging preview, searchable archive, alternate output, AI slides, translation import, Content Studio. |
| Feature specs (§13) | New sections 13.11 to 13.17. |
| Data model (§14) | New `sermon_summaries`, `exports`, `remote_sessions` tables plus FTS5 indexes. |
| Architecture (§10) | Added phone remote server, staging pipeline, summary generation pipeline. |
| Roadmap (§20) | Re-sequenced from a 7 September 2026 restart. Summary PDF pulled forward into Phase 1. |
| Pricing (§23) | Adjusted to sit sensibly against a free competitor and PewBeam's $14 / $30. |
| Platform (§9.5) | macOS is now a first-class differentiator (TajiCast is Windows only). |

---

## How to Use This Document (For Claude Code & Developers)

This PRD is the **canonical reference** while building SermonAI. When working with Claude Code:

- Reference specific sections by number when asking Claude Code for help (e.g. *"Implement FR-40 through FR-46 from section 8.8"*).
- Functional requirements are tagged `FR-XX` for direct traceability through commits, PRs and tests.
- The roadmap in Section 20 defines the build order. Do not skip phases.
- Architecture decisions in Sections 10 and 11 are binding; deviations require updating this document first.
- Every code module should map to a section in this PRD. If it doesn't, either the code is out of scope or the PRD needs an update.
- When a feature ships, tick its checkbox in Section 20 in the same PR.

---

## Table of Contents

0. [What Changed](#0-what-changed-read-this-first)
1. [Executive Summary](#1-executive-summary)
2. [Product Vision & Mission](#2-product-vision--mission)
3. [Problem Statement](#3-problem-statement)
4. [Target Users & Personas](#4-target-users--personas)
5. [Market & Competitive Landscape](#5-market--competitive-landscape)
6. [Goals, Objectives & Success Metrics](#6-goals-objectives--success-metrics)
7. [Scope](#7-scope)
8. [Functional Requirements](#8-functional-requirements)
9. [Non-Functional Requirements](#9-non-functional-requirements)
10. [Technical Architecture](#10-technical-architecture)
11. [Technology Stack](#11-technology-stack)
12. [User Flows & Journeys](#12-user-flows--journeys)
13. [Detailed Feature Specifications](#13-detailed-feature-specifications)
14. [Data Model & Storage](#14-data-model--storage)
15. [API & Integration Requirements](#15-api--integration-requirements)
16. [UI/UX Requirements](#16-uiux-requirements)
17. [Security, Privacy & Compliance](#17-security-privacy--compliance)
18. [Performance & Reliability](#18-performance--reliability)
19. [Testing & Quality Assurance](#19-testing--quality-assurance)
20. [Development Roadmap & Phasing](#20-development-roadmap--phasing)
21. [Team, Roles & Resources](#21-team-roles--resources)
22. [Risks, Assumptions & Mitigation](#22-risks-assumptions--mitigation)
23. [Pricing & Monetization](#23-pricing--monetization-revised)
24. [Launch & Go-to-Market](#24-launch--go-to-market)
25. [Post-Launch Support](#25-post-launch-support)
26. [Appendices & Glossary](#26-appendices--glossary)

---

## 1. Executive Summary

SermonAI is a desktop application for churches and ministries that listens to live sermons in real time, detects scripture references (both direct and paraphrased) and instantly displays the matching verse on a projector, LED wall, or as a lower third in a livestream, in the operator's chosen Bible translation.

The moment a service ends, SermonAI automatically produces a **detailed sermon summary** (theme, outline, every scripture with its full text, key quotes, applications, discussion questions and prayer points) and saves it as a **downloadable PDF** in the church's local library. Every sermon is also kept as a full, searchable transcript. Across a preaching series, SermonAI compiles those transcripts and summaries into a book-ready manuscript.

The category now has two established players. **TajiCast** is free, offline and Windows only. **PewBeam** is a paid presentation agent expanding into slides and lyrics. SermonAI's position is different from both:

1. **Sermon intelligence, not just verse display.** The post-service summary PDF, the searchable archive, and the series-to-book pipeline are the product, not a bonus.
2. **Cross-platform from day one.** Windows and macOS as equals, with offline mode on both. TajiCast does not run on Mac at all.
5. **Light and fast.** A base installer under 40 MB that installs in under a minute with no admin rights, against a 2 GB competitor. Heavier pieces (offline speech, extra translations, the local summary model) download on demand from inside the app.
3. **Twenty-plus properly licensed translations out of the box**, with import of any additional translation, so churches never negotiate copyright themselves.
4. **Operator-first workflow**: staging preview before anything goes live, phone remote for anyone in the room, alternate output for stage confidence monitors.

### Key Highlights

- Real-time scripture detection with sub-500ms end-to-end latency (regex path) from spoken word to projected verse.
- Automatic, detailed sermon summary PDF generated at the end of every service, downloadable at any time.
- Twenty-plus Bible translations at launch, plus import of any user-supplied translation.
- Dual-screen plus alternate output: operator control, projector, and a stage confidence monitor.
- Phone remote: anyone on the same wifi can stage, advance, search, and blank the screen.
- Full sermon transcript stored locally with search across the entire archive.
- Series-to-book compilation for turning a season of preaching into a publishable manuscript.
- Offline mode on Windows and macOS.
- NDI lower third and OBS Browser Source for livestreams.
- Base installer under 40 MB; installs in under a minute; on-demand packs for offline models and translations.
- Built on Tauri 2 (React + TypeScript frontend, Rust backend) as a single compiled binary per platform.

---

## 2. Product Vision & Mission

### 2.1 Vision Statement

To become the global standard for AI-assisted scripture projection and sermon archival, ensuring that no spoken word from the pulpit is ever lost, mistranslated, or delayed.

### 2.2 Mission Statement

SermonAI exists to free preachers from the friction of media operations and free congregants from the distraction of mismatched slides. By listening, detecting, and displaying scripture in real time, and by transforming each sermon into a permanent, summarized, written record, SermonAI helps the spoken Word become the written Word.

### 2.3 Guiding Principles

- **Ministry first, technology second.** Every feature must serve the worship experience, never disrupt it.
- **Accuracy over flash.** A correctly displayed verse beats a beautifully animated wrong one.
- **Offline by default.** Internet is a luxury in many churches; the app must work without it.
- **Operator dignity.** Volunteers running the media booth deserve a tool that makes them look competent, not one that replaces them.
- **Translation neutrality.** The app does not promote any single Bible version; it supports the operator's choice.
- **Nothing is lost.** Every sermon leaves the building as a transcript and a summary, automatically.

---

## 3. Problem Statement

### 3.1 The Core Problem

In nearly every congregation that uses projection technology, there is a measurable lag between the moment a preacher references a Bible verse and the moment that verse appears on screen. This lag, often five to fifteen seconds, disrupts the rhythm of preaching, distracts the congregation, exhausts media volunteers, and leads to displayed scriptures that are wrong, incomplete, or in the wrong translation.

A second, quieter problem sits behind the first: **once the service ends, the sermon is gone.** Most churches have no written record of what was preached, no way to search past teaching, and no practical path to turn a sermon series into a book, a devotional, or discipleship material.

### 3.2 Contributing Factors

- Preachers frequently paraphrase or jump between verses without announcing exact references.
- Media operators must guess which translation the preacher wants and search manually.
- Legacy presentation software (ProPresenter, EasyWorship, OpenLP) requires manual scripture entry.
- Most churches cannot afford a dedicated, trained, full-time media operator for every service.
- Existing AI tools produce notes as an afterthought; none produce a complete, shareable, archived summary automatically.
- Compiling a preaching series into a book requires manual transcription work that few pastors have time for.

### 3.3 Cost of the Problem

- Lost ministry impact when the wrong verse is shown or no verse is shown at all.
- Volunteer burnout among media team members under constant pressure during services.
- Poor archival of teaching content, limiting reach, discipleship resources, and book publishing opportunities.
- Reduced accessibility for congregants who rely on the projected text to follow along.

### 3.4 Why Now

Real-time speech-to-text has crossed the accuracy and latency threshold needed for live sermons. Language models now detect paraphrased scripture reliably and can produce structured, faithful summaries of hour-long transcripts. Bible APIs provide thousands of translations under flexible licensing. Two competitors have proven the category exists; neither owns the post-service value chain.

---

## 4. Target Users & Personas

### 4.1 Primary Users

#### Persona 1 — The Media Operator ("Volunteer Vincent")

- Volunteer or part-time staff member running the projection booth during services.
- Often a layperson with limited technical training; rotates with other volunteers.
- **Pain points:** anxiety during fast-paced sermons, fear of showing the wrong verse, manual searching.
- **Goals:** deliver a clean visual experience, never disrupt the preacher's flow, leave the booth feeling proud.

#### Persona 2 — The Senior Pastor ("Preaching Pastor Paul")

- Lead preacher who delivers 30 to 60 minute sermons weekly, often referencing 5 to 15 scriptures per sermon.
- Frequently paraphrases, quotes from memory, or references verses without giving the exact citation.
- **Pain points:** glancing back to check the right verse is on screen; sermons never written down; no time to write books.
- **Goals:** preach with full focus, receive a complete summary of every sermon without asking, publish books from sermon series.

#### Persona 3 — The Church Administrator ("Operations Olivia")

- Responsible for systems, software purchases, and content archival across the ministry.
- Manages livestream platforms, sermon podcasts, and discipleship resources.
- **Pain points:** transcription costs, scattered sermon archives, no searchable teaching library, nothing to hand to small groups mid-week.
- **Goals:** a single system that captures every sermon as searchable text and produces shareable assets automatically.

#### Persona 4 — The Floating Helper ("Remote Ruth") *(new in v2.0)*

- Usher, associate pastor, or youth leader who is in the room but not at the laptop.
- Needs to advance, search, or blank the screen from their phone when the operator steps away.
- **Goals:** control the screen from anywhere in the building with zero training.

### 4.2 Market Segments

- Independent local churches (50 to 500 members). Largest segment by count.
- Mid-sized denominational churches (500 to 3,000 members). Highest willingness to pay.
- Mega-churches and multi-site ministries (3,000+ members). Premium tier.
- Para-church ministries, conferences, and itinerant preachers.
- Bible colleges and seminaries.
- **Geographic priority:** Nigeria, Kenya, Ghana, Rwanda, South Africa, UK and USA. TajiCast has proven strong demand across African church media teams; SermonAI must be excellent in those environments (accented English, unreliable internet, mixed hardware).

---

## 5. Market & Competitive Landscape

### 5.1 Direct Competitors (September 2026)

#### TajiCast v2.0 (tajicast.com)

Free forever. Windows 10/11 only. Fully offline (bundled Bible database and detection model). Ships with translations and allows importing any others. Full-screen projector output, NDI lower third for OBS and vMix, phone remote over local wifi with QR pairing, staging preview before going live, full transcript of every service, searchable archive, one-click structured notes (key points, references, quotes, applications) via a built-in Content Studio. Local-only data. 1,000+ installs. About 2 GB disk footprint.

#### PewBeam (pewbeam.com)

Paid tiers: Explorer $0 (60 one-time transcription minutes), Plus $14/month, Core $30/month. Real-time speech recognition, semantic Bible search, animated verse presentations (Motion Canvas), offline reliability, AI sermon notes and export, slides and AI slide generation, all themes plus unlimited custom themes, Main + Alternate output, 3 devices per license on Core. Now positioned as a "presentation agent" covering scriptures, lyrics and service content.

### 5.2 Feature Comparison

| Capability | TajiCast v2.0 | PewBeam | SermonAI (Target) |
|---|---|---|---|
| Price | Free | $0 / $14 / $30 per month | See §23 |
| Platforms | Windows only | Windows + macOS | **Windows + macOS** |
| Offline mode | Yes (detection and STT) | Yes | **Yes, both platforms** |
| Real-time verse detection | Yes | Yes | Yes |
| Paraphrase / semantic detection | Yes | Yes | Yes (regex + vector search + LLM) |
| Translations | Built-in + import any | 6 | **20+ licensed + import any** |
| Projector full-screen | Yes | Yes | Yes |
| NDI lower third | Yes | Yes | Yes |
| OBS Browser Source | Via NDI plugin | No | **Native** |
| Alternate / confidence output | No | Yes (Core) | **Yes** |
| Phone remote | Yes | No | **Yes** |
| Stage before going live | Yes | Not advertised | **Yes** |
| Animated transitions | Basic | Yes (Motion Canvas) | Yes (Framer Motion) |
| Custom themes | Limited | Yes | Yes + shareable JSON |
| Full transcript kept | Yes | Yes | Yes |
| Searchable archive | Yes | Not advertised | Yes |
| AI sermon notes | Yes (one click, editable) | Yes (Plus tier) | **Automatic, detailed, PDF, every service** |
| Detailed summary PDF auto-saved | No (manual, notes-level) | No (manual export) | **Yes, core feature** |
| Series-to-book compilation | No | No | **Yes** |
| AI slides | No | Yes | Yes (Phase 3) |
| Lyrics / worship slides | No | Yes | Out of scope v1 |
| Devices per license | Unlimited (free) | 3 (Core) | 3 |
| Data location | Local | Local + account | Local, no account required |

### 5.3 Strategic Positioning (Revised)

The honest read: **being offline, multi-translation and transcript-keeping is now table stakes.** TajiCast gives all of that away for free. SermonAI cannot win on those alone.

SermonAI wins on four things:

1. **Sermon intelligence as the product.** The detailed summary PDF that appears in the library ninety seconds after "End Service", the searchable archive, and the series-to-book pipeline. TajiCast's notes are a one-click bonus; PewBeam's are a paid add-on. For SermonAI, they are the reason the church keeps paying.
2. **macOS.** TajiCast does not exist on Mac. A large share of church media laptops are MacBooks.
3. **Licensed translations, handled.** Twenty-plus commercially licensed translations bundled and paid for, so a church in Lagos or Nairobi never worries about publisher copyright. Import remains available for local-language Bibles.
4. **Operator confidence.** Staging preview, phone remote, alternate output, command palette and a booth-friendly dark UI, all in one place.

> **Strategic Promise**
> Every translation. Every verse on time. Every sermon summarized and saved. Every series ready to be a book.

### 5.4 Indirect Competitors

- **ProPresenter, EasyWorship, OpenLP, FreeShow, BibleShow, SmartVerses.** Manual or partially automated presentation tools. Large installed base, no post-service intelligence.

---

## 6. Goals, Objectives & Success Metrics

### 6.1 Strategic Goals

1. Establish SermonAI as the leading **sermon intelligence** tool (summary, archive, book) within twelve months of public launch.
2. Reach feature parity with TajiCast and PewBeam on live projection within six months, so no church chooses a competitor for a missing basic.
3. Empower at least one hundred pastors to publish books compiled from sermon series within twenty-four months.
4. Achieve sustainable monthly recurring revenue from a church-friendly subscription model that survives a free competitor.

### 6.2 Product Objectives (12-Month)

- Ship v1.0 on Windows and macOS within six months of the September 2026 restart.
- Onboard 250 paying churches within twelve months of public launch.
- Achieve 95%+ detection accuracy on direct references and 80%+ on paraphrases.
- Maintain end-to-end latency under 500ms (regex path) in 95% of cases.
- Generate a summary PDF within 90 seconds of "End Service" in 95% of cases.
- 90%+ of pilot pastors rate the summary PDF as "accurate and usable without edits."

### 6.3 Success Metrics (KPIs)

| Category | Metric | Target (12 months) |
|---|---|---|
| Adoption | Active paying churches | 250 |
| Adoption | Free trial-to-paid conversion | ≥25% |
| Engagement | Services per church per month | 4+ |
| Engagement | Summary PDFs opened or downloaded per church per month | 3+ |
| Engagement | Archive searches per church per month | 5+ |
| Performance | Direct scripture detection accuracy | ≥95% |
| Performance | Paraphrase detection accuracy | ≥80% |
| Performance | End-to-end latency (p95, regex path) | <500ms |
| Performance | Summary PDF generation time (p95) | <90s |
| Quality | Pastor rating of summary accuracy | ≥4.5 / 5 |
| Quality | NPS | ≥50 |
| Revenue | MRR | $8,000+ |
| Retention | Monthly churn | ≤3% |

---

## 7. Scope

### 7.1 In-Scope (v1.0 Launch)

**Live projection (parity with competitors)**
- Audio capture from any input device (microphone, USB interface, HDMI capture card, WASAPI loopback, macOS aggregate device).
- Real-time STT via Deepgram Nova-3 (online) with whisper.cpp (offline) on both platforms.
- Three-stage scripture detection: regex, in-binary semantic vector search, Claude API paraphrase detection.
- Lightweight installer with on-demand packs for offline speech, extra translations, and the optional local summary model.
- 20+ licensed translations cached locally; import of user-supplied translations (USFM, OSIS, JSON, plain text).
- Projector full-screen output, alternate/confidence output, NDI lower third, OBS Browser Source.
- Staging preview: verses are staged on the operator screen and sent live with one action.
- Phone remote over local wifi with QR pairing.
- Theme engine with animated transitions and shareable JSON themes.
- Manual override, command palette, blank hotkey.

**Sermon intelligence (SermonAI's edge)**
- Full transcript of every service persisted locally with timestamps.
- **Automatic detailed sermon summary PDF at End Service, saved to library, downloadable any time.**
- Searchable archive across all transcripts and summaries.
- Content Studio for editing summaries and exporting notes.
- PDF and DOCX export of transcripts.
- Series manager and series-to-book compilation.

### 7.2 Out of Scope (v1.0)

- Worship lyrics and general slide presentation (PewBeam's territory; SermonAI stays focused on scripture and sermon intelligence in v1).
- Live spoken-language translation.
- Speaker diarization (Phase 3).
- Cloud sync and multi-site (Phase 4).
- Congregation mobile follow-along app (Phase 3).
- Church management system integrations.

---

## 8. Functional Requirements

> Every requirement is tagged `FR-XX`. FR-01 to FR-39 carried over from v1.0. **FR-40 to FR-58 are new in v2.0.**

### 8.1 Audio Input

- **FR-01:** Enumerate all system audio input devices on launch and on demand.
- **FR-02:** Allow the operator to select a specific input device by name.
- **FR-03:** Support 16-bit PCM mono at 16kHz.
- **FR-04:** Display a live audio level meter.
- **FR-05:** Support Windows WASAPI loopback and macOS aggregate/loopback devices (BlackHole).
- **FR-06:** Start, stop, pause, and resume capture without restarting the app.

### 8.2 Speech-to-Text

- **FR-07:** Stream audio to Deepgram Nova-3 over WebSocket in online mode.
- **FR-08:** Fall back to on-device STT (whisper.cpp via `whisper-rs`) in offline mode or on network loss, **on both Windows and macOS**, provided the Offline Speech Pack is installed (see FR-60).
- **FR-09:** Apply a custom vocabulary of all 66 Bible book names and common archaic preaching terms.
- **FR-10:** Return word-level transcripts with timestamps.
- **FR-11:** Maintain a rolling 60-second transcript buffer for paraphrase analysis.

### 8.3 Scripture Detection

- **FR-12:** Apply a compiled regex to every transcript chunk for direct references.
- **FR-13:** Recognize standard ("John 3:16"), spoken ("John chapter three verse sixteen"), and shorthand ("third chapter of John") forms.
- **FR-14:** Send the rolling buffer to Claude API for paraphrase detection when no direct match has appeared for 10 seconds (online mode).
- **FR-15:** Use an in-binary vector index of all 31,102 verse embeddings (quantized, about 12 MB, brute-force cosine search under 5 ms) for semantic detection **in offline mode and as the first semantic pass in online mode**. No external vector library.
- **FR-16:** Compute and display a confidence score for every detection.
- **FR-17:** Queue multiple detections without dropping any.

### 8.4 Bible Text & Translations

- **FR-18:** Integrate with API.Bible as the primary licensed scripture source.
- **FR-19:** Pre-cache all verses of any selected translation into local SQLite.
- **FR-20:** Ship with at least 20 translations available out of the box.
- **FR-21:** Allow download of additional translations from a curated catalog.
- **FR-22:** Verify cached translation integrity on startup.
- **FR-47 (new):** Allow the operator to **import a user-supplied translation** from USFM, OSIS, JSON, or structured plain text, and use it for detection and display exactly like a bundled translation.

### 8.5 Display Output

- **FR-23:** Detect all connected displays and let the operator choose the projector.
- **FR-24:** Open a full-screen, control-free window on the projector display.
- **FR-25:** Configurable fade/slide/dissolve transitions (default 400ms).
- **FR-26:** Custom themes (background, font, color, size, layout).
- **FR-27:** Expose an NDI source named "SermonAI Scripture" as a lower third with alpha channel.
- **FR-28:** Expose a localhost URL usable as an OBS Browser Source with transparent background.
- **FR-48 (new):** Support an **Alternate Output** (stage confidence monitor) that can show a different layout from the main projector, e.g. verse plus upcoming staged verse plus clock.
- **FR-49 (new):** Support a **watermark-free** output on all paid tiers and a small watermark on the free tier.

### 8.6 Operator Control

- **FR-29:** Live transcript on the operator screen with detected scriptures highlighted.
- **FR-30:** Each detection presented as a card with Accept, Reject, Edit.
- **FR-31:** Command palette (Cmd/Ctrl+K) to search and project any verse.
- **FR-32:** Switch translation between detections without disrupting the current display.
- **FR-33:** Blank the projector with a single hotkey.
- **FR-50 (new):** **Staging area.** Accepted or searched verses land in a staging slot on the operator screen first; a single action ("Go Live", Enter, or remote tap) sends the staged verse to output. Auto-live mode is available as an opt-in setting for hands-off operation.
- **FR-51 (new):** Navigate the currently displayed passage verse by verse (next / previous) from keyboard and remote.

### 8.7 Phone Remote *(new in v2.0)*

- **FR-52:** Host a local web app on the operator machine, reachable by any phone or tablet on the same wifi.
- **FR-53:** Pair devices via a QR code and short pairing code displayed in the operator UI; sessions expire when the service ends or when revoked.
- **FR-54:** The remote shall support: view staged and live verse, Go Live, next / previous verse, search any verse, change translation, blank / unblank, open / close output.
- **FR-55:** Up to 5 concurrent remote devices; the operator UI shows who is connected and can revoke any device.

### 8.8 Post-Sermon Summary PDF *(new in v2.0, core feature)*

- **FR-40:** When the operator clicks **End Service**, the system shall automatically generate a **Detailed Sermon Summary** from the full transcript and the accepted scripture log, with no additional user action.
- **FR-41:** The summary shall contain, at minimum: sermon title (AI-suggested, editable), date, preacher name (from service settings), duration, main theme, one-paragraph overview, full outline (introduction, main points with sub-points, conclusion), every scripture referenced with full verse text in the service's default translation, key quotes with approximate timestamps, practical applications, discussion questions for small groups, prayer points, and a closing summary.
- **FR-42:** The summary shall be rendered to a **PDF** using the church's chosen export template and saved to the local library within 90 seconds of End Service (p95).
- **FR-43:** The PDF shall be **downloadable at any time** from the Sermon Library, with a one-click "Download PDF" action and an option to save to a chosen folder or re-export.
- **FR-44:** The operator shall be able to **edit** any section of the summary in Content Studio and **regenerate** the PDF; previous versions are retained.
- **FR-45:** Summary generation shall work **offline**. Default offline behaviour is a **template based summary** built directly from the transcript and accepted scripture log without an LLM (outline from scripture order and timing, quotes by extractive selection, full scripture texts). If the optional Offline Intelligence Pack is installed, the local model produces the reduced-detail AI summary instead. In both cases the full-detail AI summary is generated automatically when connectivity returns, if "enhance when online" is enabled.
- **FR-46:** The summary shall be generated in the language of the sermon transcript (English in v1.0) and shall never invent scripture references not present in the accepted scripture log or the transcript.

### 8.9 Transcript, Archive & Export

- **FR-34:** Persist the full transcript of every service with timestamps and scripture markers.
- **FR-35:** Run post-service AI analysis (this now feeds FR-40 to FR-46).
- **FR-36:** Export single sermon transcripts as PDF.
- **FR-37:** Export single sermon transcripts as DOCX.
- **FR-38:** Group sermons into named series.
- **FR-39:** Compile a series into a book-formatted PDF or DOCX with chapters, table of contents, and combined scripture index.
- **FR-56 (new):** **Searchable archive.** Full-text search across all transcripts and summaries by keyword, phrase, scripture reference, preacher, date range, or series, returning results in under 500ms for 1,000 sermons.
- **FR-57 (new):** **Content Studio.** An editor where the operator can refine the summary, produce short-form derivatives (social captions, small-group guide, devotional), and export each as PDF, DOCX, or plain text.

### 8.10 AI Slides *(new in v2.0, Phase 3)*

- **FR-58:** Generate a slide deck (PPTX and in-app) from a sermon outline or summary, one slide per main point with the supporting scripture, using the active theme.

### 8.11 Installation & On-Demand Packs *(new in v2.1)*

- **FR-59:** The base installer for each platform (signed `.msi`/`.exe` for Windows, signed and notarized `.dmg` for macOS) shall be **under 40 MB** and shall include: the application, KJV, WEB and ASV translations, the quantized verse embedding index, and the default theme. It shall install without administrator rights and without a setup wizard beyond audio device and display selection.
- **FR-60:** The app shall offer **on-demand packs** from a Settings → Packs screen, each with a clear size label, progress bar, pause/resume, and integrity check: **Offline Speech Pack** (whisper.cpp base model, about 75 MB), **Translation Packs** (one per translation, 4 to 6 MB compressed), **Offline Intelligence Pack** (quantized local summary model, 1 GB or more, clearly marked optional), and **Theme Packs**.
- **FR-61:** All packs shall be resumable across app restarts and network drops, and shall be verified by checksum before activation.
- **FR-62:** Updates shall be delivered as **delta patches** via the Tauri updater, so routine releases are a few MB and never require re-downloading packs.
- **FR-63:** The app shall run on Windows 10 (1909+) and 11 and on macOS 12+ (Intel and Apple Silicon) from a **single codebase** with feature parity, and the Windows build shall bundle or bootstrap the WebView2 runtime so that first launch never fails on a machine without it.

---

## 9. Non-Functional Requirements

### 9.1 Performance

- End-to-end latency, regex path: under 500ms (p95).
- End-to-end latency, semantic path: under 2 seconds (p95).
- Summary PDF generation: under 90 seconds (p95) for a 60-minute sermon.
- Archive search: under 500ms for 1,000 sermons.
- Phone remote action to output change: under 300ms on the same wifi.
- Cold start to ready: under 1 second on an 8GB RAM, 4-core laptop (under 3 seconds with the Offline Speech Pack loaded).

### 9.2 Reliability

- 99.5% session-level uptime; no crashes in a 90-minute service.
- Graceful degradation from cloud to local STT and detection.
- Every transcript chunk written to SQLite within 250ms.
- Summary generation is retried automatically; a failed generation never loses the transcript.

### 9.3 Scalability

- Single-user desktop instance; SQLite handles 10,000+ sermons per install.
- Licensing backend scales to 10,000+ churches.

### 9.4 Usability

- Time-to-first-verse under 10 minutes for a new operator.
- Dark mode by default; readable in a dim booth.
- Every critical function has a single keyboard shortcut and a remote button.

### 9.5 Compatibility

- **Windows 10 (1909+) and Windows 11.**
- **macOS 12+ (Intel and Apple Silicon).** First-class, not best-effort.
- Linux: community best effort.
- Outputs: HDMI, NDI 5.x, OBS 28+ Browser Source.
- Windows and macOS are equals: no feature ships on one platform without the other, and CI blocks a release if either platform build fails.

### 9.6 Maintainability

- Single Tauri project: `src/` for the React frontend, `src-tauri/` for the Rust backend.
- CI/CD builds, tests, signs, notarizes, and releases for both platforms from one pipeline.
- Opt-in crash telemetry only; no transcript content ever leaves the machine in telemetry.
- Auto-update via the Tauri updater with delta patches.

### 9.7 Installation Footprint *(new in v2.1)*

| Metric | Target |
|---|---|
| Base installer size | Under 40 MB (Windows and macOS) |
| Download to first verse on screen | Under 3 minutes on a 5 Mbps connection |
| Install time | Under 60 seconds, no admin rights, no reboot |
| Cold start | Under 1 second |
| Idle RAM | Under 120 MB |
| RAM during a service (online mode) | Under 300 MB |
| RAM during a service (offline speech loaded) | Under 900 MB |
| Disk after install (no packs) | Under 80 MB |
| Disk with Offline Speech Pack + 10 translations | Under 250 MB |
| Routine update download | Under 10 MB |

---

## 10. Technical Architecture

### 10.1 Overview

SermonAI is a single-machine desktop application built with **Tauri 2**. One compiled Rust binary hosts the backend (audio, STT, detection, storage, summary generation, export, remote server) and opens three webview windows for the React frontend (operator, projector, alternate). Frontend and backend communicate through Tauri commands and events in-process; there is no sidecar, no loopback HTTP hop for the operator UI, and no separate runtime to bundle. Outbound network traffic is limited to Deepgram, API.Bible, and Anthropic, and only in online mode.

### 10.2 The Seven Layers (revised for v2.1)

1. **Audio Input** — `cpal` crate (WASAPI on Windows, CoreAudio on macOS). 250ms PCM chunks at 16kHz mono. Loopback devices enumerated where the OS exposes them.
2. **Speech-to-Text** — Deepgram Nova-3 over `tokio-tungstenite` WebSocket online; `whisper-rs` (whisper.cpp) offline on both platforms when the Offline Speech Pack is installed.
3. **Scripture Detection** — three stages in order: regex (under 5ms), in-binary vector search over quantized verse embeddings (under 5ms), Claude API paraphrase detection (online only, 1 to 2s).
4. **Bible Text** — `rusqlite` cache seeded from bundled packs, API.Bible downloads, and user imports.
5. **Staging & Output** — staging slot on operator UI; outputs to projector webview, alternate webview, NDI, OBS overlay. Nothing reaches an output without passing through staging (unless auto-live is on).
6. **Phone Remote** — an `axum` server inside the binary serves a small React web app on the LAN only while remote is enabled; WebSocket keeps remote and operator UI in sync; pairing via short-lived token in a QR code.
7. **Sermon Intelligence** — transcript store, summary pipeline (Claude online; template renderer offline; optional local model via `llama-cpp` bindings), PDF renderer, archive search (SQLite FTS5), Content Studio, series compiler.

### 10.3 Process Topology

- **One process.** The Tauri binary owns everything.
- **Three webview windows:** operator, projector, alternate. Created and positioned by Rust using the Tauri window and monitor APIs.
- **Two optional LAN listeners**, both inside the same process and off by default: `0.0.0.0:8002` for the phone remote (only while enabled), `localhost:8001` for the OBS Browser Source overlay.
- **Background tasks** on the `tokio` runtime: audio capture, STT stream, detection pipeline, pack downloader, summary jobs, updater.

### 10.4 Data Flow

```
Live service
  audio device → PCM chunks → STT → transcript words
    → regex → vector search → (Claude) → detection card
    → operator Accept (or auto-live) → staging slot → Go Live
    → projector / alternate / NDI / OBS
  every transcript chunk + every accepted verse → SQLite (WAL)

End Service
  transcript + scripture log → summary pipeline (Claude or local model)
    → structured summary JSON → sermon_summaries table
    → PDF renderer (ReportLab, church template) → exports table + file on disk
    → Sermon Library shows "Download PDF"
    → FTS5 index updated for archive search
```

### 10.5 Summary Generation Pipeline (detail)

1. On End Service, seal the transcript and mark the sermon `status = processing`.
2. Chunk the transcript into ~8-minute windows with overlap; extract candidate key quotes and timestamps.
3. Build the prompt: transcript, accepted scripture log (with full verse text), preacher name, date, church name, and the summary schema (§26.3).
4. Online: call Claude with a strict JSON schema for the summary sections. Offline without the Intelligence Pack: run the template renderer (extractive quotes, scripture-ordered outline, full verse texts). Offline with the Intelligence Pack: run the local model with the reduced schema.
5. Validate: every scripture reference in the summary must exist in the accepted log or match a regex hit in the transcript (FR-46). Drop anything else.
6. Store the summary JSON. Render the PDF by loading the HTML template in a hidden webview and printing to PDF (pixel-exact with the in-app preview), or via `typst` for the book compiler. Store the file path and a version number.
7. Notify the operator UI; the library card flips to "Summary ready. Download PDF".
8. On failure at any step, retry three times with backoff, then mark `status = needs_attention` and keep the transcript intact.

### 10.6 Failure Modes & Recovery

- Audio device disconnect: pause, alert, re-select without restart.
- Deepgram drop: reconnect with backoff; switch to whisper.cpp after three failures if the Offline Speech Pack is installed, otherwise show a clear "transcription paused, no internet" banner and offer the pack download.
- API.Bible failure: serve from cache; otherwise show reference with "verse unavailable".
- Claude unreachable: regex and vector search continue; summary uses the template renderer (or local model if installed) and upgrades later.
- Pack download interrupted: resume from the last verified chunk on next launch; never activate a partial pack.
- WebView2 missing on Windows: installer bootstraps it silently; app refuses to start with a helpful message if bootstrap fails.
- SQLite write failure: in-memory queue, retry, alert after 10 seconds.
- Remote device loses wifi: operator UI keeps state; remote reconnects and re-syncs.

### 10.7 Project Structure

```
sermonai/
├── src/                          # React + TypeScript frontend (all three windows + remote)
│   ├── windows/
│   │   ├── operator/             # transcript, cards, staging, library, studio, settings, packs
│   │   ├── projector/            # main output
│   │   └── alternate/            # confidence monitor output
│   ├── remote/                   # phone remote web app (mobile-first), served by the Rust axum server
│   ├── components/               # shared UI
│   ├── stores/                   # Zustand stores
│   ├── lib/                      # Tauri command wrappers, event subscriptions, types
│   └── templates/                # HTML templates for summary PDF (rendered by webview print)
├── src-tauri/                    # Rust backend
│   ├── src/
│   │   ├── main.rs               # app setup, window creation, monitor placement, tray, updater
│   │   ├── commands/             # #[tauri::command] handlers grouped by domain
│   │   ├── audio/                # cpal capture, device enumeration, level meter
│   │   ├── stt/                  # deepgram.rs (websocket), whisper.rs (whisper-rs), router.rs
│   │   ├── detection/            # regex.rs, vector.rs (in-binary cosine search), llm.rs, pipeline.rs
│   │   ├── bible/                # cache.rs (rusqlite), api_bible.rs, importer.rs (USFM/OSIS/JSON)
│   │   ├── output/               # staging.rs, ndi.rs, obs_overlay.rs
│   │   ├── remote/               # axum server, pairing.rs, ws_hub.rs
│   │   ├── intelligence/         # summarizer.rs, template_renderer.rs, validator.rs, pdf.rs, search.rs
│   │   ├── export/               # transcript_pdf.rs, docx.rs, book.rs, slides.rs
│   │   ├── packs/                # manifest.rs, downloader.rs (resumable, checksummed), registry.rs
│   │   └── db/                   # migrations, models
│   ├── assets/                   # bundled: KJV/WEB/ASV packs, quantized verse index (~12 MB), default theme
│   ├── tauri.conf.json
│   └── Cargo.toml
├── scripts/
│   ├── build-verse-index.py      # one-time: embed 31,102 verses, quantize to int8, write index file
│   ├── build-translation-pack.py # API.Bible → compressed pack + checksum
│   └── publish-packs.sh          # upload packs + manifest to the pack CDN
├── packs-manifest.json           # list of downloadable packs with sizes and checksums
├── docs/
│   ├── PRD.md                    # this document
│   └── MILESTONES.md             # stage tracker
└── README.md
```

---

## 11. Technology Stack

> **Why this stack (v2.1).** The founder requires Windows and macOS parity and a lightweight, fast install. Electron ships a full Chromium (150 to 220 MB) and a bundled Python runtime adds another 90 to 180 MB. Tauri uses the webview the OS already has (WebView2 on Windows, WKWebView on macOS) and compiles the backend to a native binary, giving a 15 to 30 MB base install, sub-second cold start, and roughly a quarter of the memory. The frontend is unchanged from v2.0.

### 11.1 Frontend

| Layer | Technology | Why |
|---|---|---|
| Desktop shell | **Tauri 2** | 15 to 30 MB installs, native windows, Windows + macOS from one codebase |
| UI | React 18 + TypeScript | Mature, ecosystem-rich |
| Styling | Tailwind CSS | Fast, consistent |
| State | Zustand | Lightweight |
| Animation | Framer Motion | Verse transitions |
| Backend calls | `@tauri-apps/api` commands + events | In-process, typed, no HTTP hop |
| Remote app | React (mobile-first) | Same stack, served over LAN by the Rust server |
| Build | Vite + Tauri CLI | Fast dev, signed and notarized installers |

### 11.2 Backend (Rust, compiled into the Tauri binary)

| Layer | Crate / Technology | Why |
|---|---|---|
| Language | **Rust** (stable) | Native speed, tiny binary, memory safe, first-class Tauri integration |
| Async runtime | `tokio` | Audio, network, jobs on one runtime |
| Audio | `cpal` | WASAPI (Windows) and CoreAudio (macOS), low latency |
| STT online | `tokio-tungstenite` + Deepgram WebSocket API | Sub-400ms streaming, custom vocabulary |
| STT offline | `whisper-rs` (whisper.cpp) | Accurate, fast, same model on both platforms; loaded from the Offline Speech Pack |
| Detection LLM | `reqwest` + Anthropic Messages API | Paraphrase detection, summaries |
| Vector search | In-binary brute-force cosine over int8 verse embeddings | 31K × 384 dims, under 5 ms, ~12 MB, no external library |
| Offline summary (default) | Template renderer in Rust | Useful PDF with no model at all |
| Offline summary (optional) | `llama-cpp` Rust bindings | Reduced-detail AI summary from the optional Intelligence Pack |
| Bible client | `reqwest` | API.Bible downloads, pack fetching |
| Bible import | Custom parsers (USFM, OSIS, JSON, CSV) | User-supplied translations |
| Database | `rusqlite` (bundled SQLite, FTS5 enabled, WAL) | Zero-config, full-text search |
| PDF | Hidden webview print-to-PDF for summaries; `typst` for the book compiler | Pixel-exact preview, professional typesetting for long documents |
| DOCX | `docx-rs` | Editable exports |
| Slides | `pptx` generation via `docx-rs`-style XML writer or template | AI slides (Phase 3) |
| NDI | NDI SDK via `ndi-sys` bindings | Lower thirds |
| Remote / overlay server | `axum` | Phone remote and OBS overlay, LAN only |
| Packs | `reqwest` ranged downloads + `sha2` | Resumable, checksummed on-demand packs |
| Updates | Tauri updater plugin | Signed delta patches |

### 11.3 External Services

- **Deepgram** Nova-3 streaming STT.
- **API.Bible** licensed scripture text.
- **Anthropic Claude API** paraphrase detection and summary generation.
- **OpenAI Embeddings** one-time build job to embed all verses (never called from the app).
- **Pack CDN** (object storage + CDN) hosting translation, speech, intelligence and theme packs with a signed manifest.

### 11.4 Tooling

- GitHub, GitHub Actions CI/CD (matrix: windows-latest, macos-latest for both Intel and Apple Silicon), ESLint + Prettier, `cargo fmt` + `clippy`, Vitest + Playwright (frontend), `cargo test` (backend), `tauri-driver` for end-to-end, Sentry (opt-in).
- Code signing: Windows Authenticode certificate; Apple Developer ID with notarization.

---

## 12. User Flows & Journeys

### 12.1 First-Run Setup

1. Download and run installer (Windows or macOS).
2. Onboarding wizard: license or free trial → audio device with live meter → projector and alternate display → translations (KJV default, add more) → theme → optional phone remote pairing.
3. Land on the operator dashboard.

### 12.2 Live Sermon

1. Open SermonAI ten minutes early; confirm audio level.
2. Enter service details: preacher name, sermon title if known, series if any.
3. Click **Start Service**.
4. Transcript streams. Preacher says "Jeremiah 29:11". Regex catches it; card appears.
5. Operator presses Enter: verse lands in **staging**. Operator reads it, presses Enter again: **Go Live**. Projector fades in.
6. Preacher paraphrases. Vector search proposes a verse at 71%; card appears. Operator accepts, stages, goes live.
7. Preacher reads on. Operator or remote helper presses **Next** to advance to the following verse.
8. Operator steps away; usher opens the phone remote, blanks the screen during prayer, un-blanks after.
9. Operator clicks **End Service**.

### 12.3 Post-Service (automatic)

1. Transcript sealed. Summary pipeline runs. Progress shown on the Sermon Library card.
2. Within 90 seconds: "Summary ready. **Download PDF**."
3. Pastor opens the PDF on his phone from the church's shared folder, or the operator emails it.
4. Any time later, anyone with access opens the Library, finds the sermon, clicks Download PDF.

### 12.4 Editing a Summary

1. Open the sermon in the Library → **Open in Content Studio**.
2. Edit title, outline, applications, questions. Add or remove quotes.
3. Click **Regenerate PDF**. New version saved; old version kept.

### 12.5 Series-to-Book

1. Series tab → create series → add sermons → drag to reorder → choose template → **Compile Book**.
2. Book PDF/DOCX generated with chapters, table of contents, combined scripture index, and each chapter's summary as an opener.

### 12.6 Searching the Archive

1. Press Cmd/Ctrl+Shift+F anywhere.
2. Type "grace" or "Romans 8" or "Pastor Paul June 2027".
3. Results show sermon, timestamped transcript hits, and summary hits. Click to open.

---

## 13. Detailed Feature Specifications

### 13.1 Audio Device Selector
- Lists all input devices with channel counts and clear labels for HDMI capture cards.
- 30fps VU meter in dBFS; 3-second test playback.
- Persists selection.

### 13.2 Live Transcript Panel
- Word-by-word append; amber highlight on detected scripture; auto-scroll with manual override; in-session search; font size setting.

### 13.3 Scripture Detection Card
- Reference, verse text, translation, confidence, detection source (regex / semantic / AI).
- Accept, Reject, Edit. Accept sends to staging (or live if auto-live). Auto-dismiss after 30s.

### 13.4 Translation Picker
- 20+ bundled plus imported translations; search-as-you-type; download icon for uncached; per-service default and per-verse override.

### 13.5 Staging Slot *(new)*
- Persistent panel showing "Staged" and "Live" side by side.
- Enter or Go Live button promotes staged to live. Escape clears staging.
- Auto-live toggle in settings with an obvious on-screen indicator when enabled.

### 13.6 Projector Display
- Full-screen; layouts: verse only, verse + reference, verse + reference + translation.
- Backgrounds: color, image, looping video. Auto-fit font size. Fade / slide / dissolve. Blank hotkey.

### 13.7 Alternate Output *(new)*
- Separate window for a stage monitor: current verse large, staged verse small, clock, elapsed time. Layout configurable.

### 13.8 Phone Remote *(new)*
- Operator enables Remote; UI shows QR + 6-digit code.
- Remote pages: Now (staged / live, Go Live, Next, Prev, Blank), Search (any verse, translation), Devices (operator only).
- Works on any modern phone browser; no app install.

### 13.9 Command Palette
- Cmd/Ctrl+K; type "john 3 16"; Enter stages, Shift+Enter goes live. Recent verses at top.

### 13.10 Theme Designer
- Backgrounds, fonts, colors, animation, padding; live preview; JSON export/import. Five presets at launch.

### 13.11 Detailed Sermon Summary PDF *(new, core)*

**Trigger:** automatic on End Service.

**PDF structure (default template "Sermon Brief"):**

1. **Cover** — church name and logo, sermon title, preacher, date, series, duration, translation used.
2. **At a Glance** — main theme (one sentence), overview paragraph, three to five key takeaways.
3. **Sermon Outline** — introduction; main points with sub-points and the scripture anchoring each; conclusion.
4. **Scriptures Referenced** — every accepted scripture in order of appearance, with full verse text, translation tag, and the approximate time it was referenced. A second index sorted by book appears at the end.
5. **Key Quotes** — eight to fifteen memorable lines from the preacher with timestamps.
6. **Practical Applications** — three to seven concrete actions drawn from the sermon.
7. **Discussion Questions** — five to eight questions for small groups or family devotions.
8. **Prayer Points** — three to six prayer prompts drawn from the sermon's themes.
9. **Closing Summary** — one paragraph.
10. **Footer on every page** — sermon title, date, page number, "Generated by SermonAI".

**Rules**
- Never invent scripture references (validator enforces FR-46).
- Direct quotes are drawn from the transcript, lightly cleaned for filler words, never rewritten.
- Length target: 4 to 8 pages for a 45-minute sermon.
- Church can choose alternative templates: "Study Guide" (heavier on questions), "Devotional" (heavier on applications), "Minimal" (2 pages).

**Library behaviour**
- Sermon card shows a PDF badge and a Download button when ready.
- Download opens the system save dialog; a second option copies to a configured "auto-save" folder (e.g. a shared Google Drive or OneDrive folder the church already syncs).
- Versions retained; the latest is downloaded by default.

### 13.12 Sermon Library
- Grid of sermons with title, date, preacher, series, scripture count, summary status.
- Open transcript, open summary, Download PDF, Open in Content Studio, add to series.
- Filters: date, preacher, series, book of the Bible.

### 13.13 Searchable Archive *(new)*
- SQLite FTS5 index over transcripts and summaries.
- Global shortcut; results grouped by sermon with timestamped snippets.

### 13.14 Content Studio *(new)*
- Rich editor over the summary JSON with section-by-section editing.
- Derivative generators: social captions, small-group guide, devotional, email newsletter blurb.
- Export each derivative as PDF, DOCX, or text. Regenerate PDF after edits.

### 13.15 Translation Importer *(new)*
- Import wizard accepting USFM, OSIS, JSON (book/chapter/verse), or CSV.
- Validates structure, reports missing verses, assigns a code and name, adds to the picker and detection index.

### 13.16 Series Manager & Book Compiler
- Create series, add and reorder sermons, choose book template, compile to PDF/DOCX with chapters, table of contents, combined scripture index; each chapter opens with that sermon's summary.

### 13.17 AI Slides *(Phase 3)*
- From an outline or summary, generate one slide per main point with supporting scripture in the active theme. Export PPTX or present in-app.

---

## 14. Data Model & Storage

### 14.1 Strategy

Single local SQLite file (WAL mode, FTS5 enabled) in the app data directory. No cloud in v1.0. Optional cloud backup in Phase 4.

### 14.2 Tables

```sql
CREATE TABLE bible_verses (
  id INTEGER PRIMARY KEY,
  translation_code TEXT NOT NULL,
  book_id TEXT NOT NULL,
  chapter INTEGER NOT NULL,
  verse INTEGER NOT NULL,
  text TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(translation_code, book_id, chapter, verse)
);
CREATE INDEX idx_verse_lookup ON bible_verses(translation_code, book_id, chapter, verse);

CREATE TABLE translations (
  code TEXT PRIMARY KEY,
  full_name TEXT NOT NULL,
  language TEXT NOT NULL,
  copyright_status TEXT,
  source TEXT NOT NULL,              -- 'api_bible' | 'import'
  api_bible_id TEXT,
  imported_from TEXT,
  downloaded_at TIMESTAMP
);

CREATE TABLE series (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL,
  description TEXT,
  book_template TEXT,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sermons (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT,
  preacher TEXT,
  date TIMESTAMP NOT NULL,
  duration_seconds INTEGER,
  default_translation TEXT,
  series_id INTEGER REFERENCES series(id),
  status TEXT NOT NULL DEFAULT 'live', -- live | processing | ready | needs_attention
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE transcript_segments (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER NOT NULL REFERENCES sermons(id),
  start_time_ms INTEGER NOT NULL,
  end_time_ms INTEGER NOT NULL,
  text TEXT NOT NULL,
  confidence REAL
);
CREATE INDEX idx_transcript_sermon ON transcript_segments(sermon_id, start_time_ms);

CREATE TABLE detected_scriptures (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER NOT NULL REFERENCES sermons(id),
  segment_id INTEGER REFERENCES transcript_segments(id),
  book TEXT NOT NULL,
  chapter INTEGER NOT NULL,
  verse INTEGER NOT NULL,
  end_verse INTEGER,
  translation TEXT NOT NULL,
  confidence REAL,
  source TEXT,                        -- regex | vector | llm | manual
  accepted_by_operator BOOLEAN,
  went_live BOOLEAN,
  displayed_at TIMESTAMP
);

-- new in v2.0
CREATE TABLE sermon_summaries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER NOT NULL REFERENCES sermons(id),
  version INTEGER NOT NULL DEFAULT 1,
  generator TEXT NOT NULL,            -- claude | local
  template TEXT NOT NULL,             -- sermon_brief | study_guide | devotional | minimal
  summary_json TEXT NOT NULL,         -- structured sections (see §26.3)
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  UNIQUE(sermon_id, version)
);

-- new in v2.0
CREATE TABLE exports (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  sermon_id INTEGER REFERENCES sermons(id),
  series_id INTEGER REFERENCES series(id),
  summary_id INTEGER REFERENCES sermon_summaries(id),
  kind TEXT NOT NULL,                 -- summary_pdf | transcript_pdf | transcript_docx | book_pdf | book_docx | slides_pptx | derivative
  file_path TEXT NOT NULL,
  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- new in v2.0
CREATE TABLE remote_sessions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  device_label TEXT,
  token_hash TEXT NOT NULL,
  paired_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
  revoked_at TIMESTAMP
);

CREATE TABLE settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

-- full-text search
CREATE VIRTUAL TABLE transcript_fts USING fts5(text, content='transcript_segments', content_rowid='id');
CREATE VIRTUAL TABLE summary_fts USING fts5(summary_text, content='sermon_summaries', content_rowid='id');
```

### 14.3 File Storage

- App data directory layout: `/db/sermonai.sqlite`; `/packs/speech/` (whisper models); `/packs/translations/<code>/`; `/packs/intelligence/` (optional local LLM); `/packs/themes/`; `/exports/YYYY/MM/<sermon-slug>/` for PDFs, DOCX, PPTX; `/logs/`.
- The quantized verse embedding index (~12 MB) and KJV/WEB/ASV ship inside the binary's assets, not as packs.
- Default export folder: `Documents/SermonAI/`. Optional auto-save folder for summary PDFs.
- Removing a pack from Settings deletes its directory and frees the space immediately.

### 14.4 Retention & Privacy

- Everything local. Telemetry opt-in and content-free. Delete any sermon, summary, export, or the whole database from settings.

---

## 15. API & Integration Requirements

### 15.1 Deepgram
- `wss://api.deepgram.com/v1/listen`, model `nova-3`, 16kHz mono PCM, custom vocabulary, interim + final results. ~$0.006/min. 45,000 free minutes per year for development.

### 15.2 API.Bible
- `https://api.scripture.api.bible/v1`, `api-key` header. Bulk cache per translation. Public domain free; commercial translations ~$10/month each, paid by SermonAI and bundled into tiers.

### 15.3 Anthropic Claude
- `https://api.anthropic.com/v1/messages`, `claude-sonnet-4-6`. Two uses: live paraphrase detection (short prompts) and summary generation (structured JSON output, one call per sermon plus retries). Estimated $1 to $6 per church per month.

### 15.4 OpenAI Embeddings (build-time only)
- `text-embedding-3-small` over 31,102 verses, once, ~$2. Reduced to 384 dimensions and quantized to int8 by `build-verse-index.py`; the resulting ~12 MB index ships inside the binary. At runtime the app embeds the spoken phrase with a small on-device sentence encoder bundled in the same asset so semantic search is fully offline.

### 15.5 On-Demand Packs (offline)
- **Offline Speech Pack:** whisper.cpp `base.en` (~75 MB) default; `small.en` (~250 MB) offered as "higher accuracy".
- **Offline Intelligence Pack:** quantized 1 to 3B parameter local LLM (1 to 2 GB). Optional. Clearly labelled. Never required for a usable summary PDF.
- **Translation Packs:** one per translation, 4 to 6 MB compressed, built from API.Bible under license.
- All packs served from the Pack CDN with a signed `packs-manifest.json` listing name, version, size, and SHA-256.

### 15.6 NDI
- NDI SDK via `ndi-sys` Rust bindings, shipped as an optional pack; source "SermonAI Scripture"; 1920×1080 with alpha for lower thirds.

### 15.7 OBS Browser Source
- `http://localhost:8001/overlay`, transparent, WebSocket-driven.

### 15.8 Phone Remote
- `axum` server inside the Tauri binary on LAN port 8002 only while enabled; QR contains `http://<lan-ip>:8002/r/<token>`; token expires at End Service; WebSocket sync with the operator UI.

---

## 16. UI/UX Requirements

### 16.1 Principles
- Booth-friendly dark mode; glanceable state; forgiving actions; quiet motion.

### 16.2 Operator Window
- **Top bar:** service status, audio meter, elapsed time, remote status, blank toggle.
- **Left (55%):** live transcript.
- **Right (45%):** detection cards (top), staging slot (middle), live preview (bottom).
- **Bottom bar:** translation picker, theme, command palette, End Service.
- **Tabs:** Live, Library, Series, Studio, Settings.

### 16.3 Projector & Alternate
- Projector: verse center 70%, reference and translation bottom 10%.
- Alternate: live verse large, staged verse small, clock, elapsed.

### 16.4 Phone Remote
- Thumb-reachable buttons; Go Live is the largest; blank is red and requires no confirmation; search is one tap away.

### 16.5 Accessibility
- Keyboard-navigable; screen-reader labels; 14px minimum; color-blind safe themes.

### 16.6 Brand
- Primary deep blue `#2E5BBA`; accent amber `#F2A623`; Inter for UI, Crimson Pro for scripture.

---

## 17. Security, Privacy & Compliance

- All sermon content local; no account required; TLS to vendors; telemetry opt-in and content-free.
- License key validated periodically; 3 devices per license; 30-day offline grace.
- Vendor API keys provisioned through SermonAI licensing and stored in the OS keychain; customers never handle them.
- Phone remote: LAN only, token-based, revocable, disabled by default, expires at End Service.
- Bible translations only via licensed channels; imported translations are the church's responsibility and are flagged as such in the UI.
- GDPR and CCPA satisfied by local-only design with export and delete.

---

## 18. Performance & Reliability

### 18.1 Latency Targets

| Step | p95 | p99 |
|---|---|---|
| Audio chunk | 250ms | 300ms |
| STT word return (online) | 400ms | 700ms |
| Regex detection | 5ms | 15ms |
| Vector semantic detection | 5ms | 20ms |
| LLM paraphrase detection | 1500ms | 3000ms |
| Cache lookup | 5ms | 20ms |
| Staged → live on output | 50ms | 100ms |
| Remote tap → output | 300ms | 600ms |
| **End-to-end regex path** | **500ms** | **800ms** |
| **End-to-end semantic path** | **2000ms** | **3500ms** |
| Summary PDF (60-min sermon, online) | 90s | 150s |
| Summary PDF (offline, reduced) | 180s | 300s |
| Archive search (1,000 sermons) | 500ms | 1s |

### 18.2 Resources
- Idle RAM under 120 MB; online-mode service under 300 MB; offline speech loaded under 900 MB. CPU under 25% on 4 cores online, under 60% while whisper.cpp is transcribing. Disk write under 1 MB/s. See §9.7 for the full footprint table.

### 18.3 Reliability Engineering
- Retry with backoff everywhere; SQLite WAL; backend watchdog; circuit breakers on all vendor APIs; summary jobs persisted and resumable after crash.

---

## 19. Testing & Quality Assurance

- `cargo test` unit tests for every backend module including the summary validator (no invented scriptures).
- CI gate: installer size on each platform must be under 40 MB or the build fails.
- Integration tests: PCM → detection → staging → output; transcript → summary → PDF.
- Playwright end-to-end for operator flows and the phone remote.
- Golden-file tests for summary PDFs across the four templates.
- Test dataset: 50 sermon clips with ground-truth transcripts and scripture references, weighted toward Nigerian, Kenyan, Ghanaian, British and American English.
- Pilot: 10 to 15 churches, at least half in Nigeria and Kenya, at least three on macOS.

---

## 20. Development Roadmap & Phasing

> Restarted **7 September 2026**. Each phase is gated by exit criteria. Tick boxes in the PR that ships the item. **Fine-grained stage tracking lives in `docs/MILESTONES.md`**; this section is the phase-level view.

### 20.1 Phase 0 — Foundation (8 Sept to 21 Sept 2026)

**Exit:** Tauri project scaffolded, CI green with signed builds on Windows and macOS, vendor keys provisioned, verse index built and bundled, pack downloader working against a test manifest.

- [ ] Tauri 2 project: React + TypeScript frontend, Rust backend, three windows opening on the correct monitors.
- [ ] CI/CD matrix for Windows and macOS (Intel + Apple Silicon) with signing and notarization.
- [ ] Deepgram, API.Bible, Anthropic keys.
- [ ] `build-verse-index.py` run once; ~12 MB quantized index committed to `src-tauri/assets/`.
- [ ] KJV, WEB, ASV bundled as assets; `build-translation-pack.py` producing packs for the CDN.
- [ ] Pack downloader (resumable, checksummed) with Settings → Packs screen against a test manifest.
- [ ] Base installer measured under 40 MB on both platforms (this is a gate).
- [ ] ADRs for: Tauri over Electron, staging-first output, three-stage detection, local-only data, summary schema, packs strategy.

### 20.2 Phase 1 — MVP with Summary PDF (22 Sept to 16 Nov 2026)

**Exit:** a real Sunday service run end to end on one church laptop: verses on screen, transcript kept, summary PDF downloadable within 90 seconds of End Service.

- [ ] Audio capture and device selection (FR-01 to FR-06).
- [ ] Deepgram streaming with live transcript (FR-07, FR-10, FR-11).
- [ ] Regex + vector detection (FR-12, FR-13, FR-15, FR-16, FR-17).
- [ ] API.Bible cache with 5 translations (FR-18, FR-19, FR-22).
- [ ] Projector window, transitions, one theme (FR-23 to FR-25).
- [ ] Detection cards, staging slot, Go Live, blank, command palette (FR-29 to FR-33, FR-50, FR-51).
- [ ] Transcript persistence (FR-34).
- [ ] **Summary pipeline, validator, PDF renderer, Library with Download PDF (FR-40 to FR-44, FR-46).**
- [ ] Sermon Library basic (FR-35).

### 20.3 Phase 2 — Competitive Parity + Pilot (17 Nov 2026 to 25 Jan 2027)

**Exit:** closed pilot in 10+ churches running weekly; no feature a pilot church names as "TajiCast has it and you don't".

- [ ] Offline Speech Pack and whisper.cpp STT on Windows and macOS (FR-08, FR-60, FR-61).
- [ ] Offline summary: template renderer by default, optional Intelligence Pack, "enhance when online" (FR-45).
- [ ] Delta updates via Tauri updater (FR-62).
- [ ] Claude paraphrase stage (FR-14).
- [ ] Translations to 20+ and catalog downloads (FR-20, FR-21).
- [ ] Translation importer (FR-47).
- [ ] Alternate output (FR-48) and watermark logic (FR-49).
- [ ] NDI lower third and OBS Browser Source (FR-27, FR-28).
- [ ] Phone remote with pairing (FR-52 to FR-55).
- [ ] Theme designer (FR-26).
- [ ] Searchable archive (FR-56).
- [ ] Custom vocabulary (FR-09).
- [ ] Pilot launch (Nigeria, Kenya, UK, USA; Windows and macOS).

### 20.4 Phase 3 — Sermon Intelligence Depth + Public Launch (26 Jan to 22 Mar 2027)

**Exit:** public launch with paying customers using Studio, series, and book features.

- [ ] Content Studio with editing, regeneration, derivatives (FR-57).
- [ ] Transcript PDF and DOCX export (FR-36, FR-37).
- [ ] Series manager and book compiler (FR-38, FR-39).
- [ ] Summary templates: Study Guide, Devotional, Minimal.
- [ ] AI slides to PPTX (FR-58).
- [ ] Speaker diarization (basic).
- [ ] Congregation mobile follow-along (read-only).
- [ ] Public launch.

### 20.5 Phase 4 — Scale (Apr to Sept 2027)

- [ ] Optional cloud backup of library.
- [ ] Multi-site synchronization.
- [ ] Live in-service language translation.
- [ ] Church management integrations.
- [ ] Enterprise tier.

---

## 21. Team, Roles & Resources

### 21.1 Core Team

| Role | Responsibility | Phase |
|---|---|---|
| Founding Engineer / CTO | Rust backend, AI pipelines, summary quality, architecture | 0 onward |
| Frontend Engineer | Tauri windows, React, remote app, outputs, themes | 1 onward |
| ML / NLP Engineer | Detection accuracy, prompt and schema design, offline models | 2 onward |
| UI / UX Designer | Operator UI, remote, PDF templates, brand | 1 onward (part-time) |
| Pilot Lead | Church onboarding in Nigeria and Kenya, feedback, support | 2 onward |
| Founder / Product | Strategy, licensing deals, go-to-market | 0 onward |

### 21.2 Solo Founder + Claude Code

Realistic with this PRD as the constant reference: Phase 0 and 1 in ten weeks, Phase 2 in ten weeks, Phase 3 in eight weeks. Claude Code handles scaffolding, Rust module implementation, tests, and PDF templates; the founder reviews, runs real services, and tunes prompts against real transcripts. Rust is a good fit for this workflow because the compiler catches most mistakes before they reach a Sunday service; the founder does not need to hand-write Rust, only read it, run it, and describe what is wrong.

### 21.3 Budget (12 months)

- Engineering (lean team of 3): $180,000 to $300,000.
- Vendor APIs during dev and pilot: $2,000 to $4,000 (summary generation adds volume).
- Translation licenses (20 commercial × ~$10/month): ~$2,400/year at pilot scale; scales with tiers.
- Code signing: $400/year. Infrastructure: $2,400/year. Design: $5,000 to $10,000. Legal: $5,000 to $8,000. Marketing: $10,000 to $25,000.

---

## 22. Risks, Assumptions & Mitigation

### 22.1 Technical

| Risk | L / I | Mitigation |
|---|---|---|
| Deepgram price increase | M / H | Local STT fallback; evaluate AssemblyAI quarterly |
| API.Bible licensing change | L / H | Multi-year agreements; importer gives churches an escape hatch |
| STT accuracy on African English | M / H | Custom vocabulary; pilot in Nigeria and Kenya; model switch if needed |
| HDMI capture card compatibility | M / M | Test top 5 cards; publish compatibility list |
| LLM false-positive paraphrases | H / M | Staging-first output; confidence threshold; operator accept |
| Summary hallucination | M / H | Strict schema; validator rejects any scripture not in log or transcript; quotes only from transcript |
| Offline model too large or slow | M / M | Tiered model sizes; reduced-detail template offline; "enhance when online" |
| LAN remote security | L / M | Token pairing, LAN only, off by default, expires at End Service |
| Rust learning curve slows a solo founder | M / M | Claude Code writes and explains; strict module boundaries; keep business logic simple and tested |
| WebView2 missing or outdated on old Windows machines | M / M | Installer bootstraps the evergreen runtime; app checks version on launch; documented minimum |
| NDI SDK licensing and binary size | L / M | Ship NDI as an optional pack rather than in the base installer |
| Installer creeps above 40 MB over time | M / L | CI gate fails the build if the installer exceeds the target; new heavy assets must become packs |

### 22.2 Business

- **TajiCast is free.** Mitigation: SermonAI does not sell verse display; it sells the summary, the archive, the book, the Mac, and the licenses handled. Free tier keeps display available so churches try it.
- **PewBeam moves into summaries.** Mitigation: ship first, ship deeper (validator, templates, series), and own the "every sermon becomes a book" story.
- **Churches resist subscriptions.** Mitigation: perpetual license option; regional non-profit discount.
- **Publisher copyright complaints.** Mitigation: licensed channels only; imported translations clearly marked as church-supplied.

### 22.3 Operational

- Solo burnout: strict phase gates; Claude Code for throughput; pilot lead early.
- Pilot disengagement: weekly check-ins; fix Sunday bugs by Wednesday.

### 22.4 Assumptions

- Churches will install desktop software on Windows and Mac.
- Pastors value an automatic, accurate summary enough to pay for it when display is free elsewhere.
- Vendor pricing stays within a range that keeps $15 to $60 monthly tiers profitable.

---

## 23. Pricing & Monetization (Revised)

Against a free competitor and PewBeam at $14 / $30, the free tier must be truly usable for live display, and paid tiers must be justified by sermon intelligence.

| Feature | Free | Plus | Pro |
|---|---|---|---|
| **Price / month** | $0 | $15 | $35 |
| Live verse detection and projector | Yes (small watermark) | Yes | Yes |
| Offline mode | Yes | Yes | Yes |
| Translations | KJV, WEB, ASV + import your own | 15 licensed + import | All 20+ licensed + import |
| Transcription | 90 min/month cloud, unlimited offline | Unlimited | Unlimited |
| Summary PDF after every service | Minimal template, 4 per month | All templates, unlimited | All templates, unlimited |
| Searchable archive | Last 10 sermons | Unlimited | Unlimited |
| Content Studio and derivatives | No | Yes | Yes |
| Phone remote | 1 device | 3 devices | 5 devices |
| NDI, OBS, alternate output | OBS only | Yes | Yes |
| Series-to-book compiler | No | 2 books/year | Unlimited |
| AI slides | No | Yes | Yes |
| Devices per license | 1 | 2 | 3 |
| Support | Community | Email + chat | Priority |

- Annual plans 20% off. Verified ministries in low-income regions 50% off Plus. Perpetual license at 2× annual.
- Unit economics: vendor cost per Plus church ~$6 to $12/month; gross margin ~55 to 60% on Plus, ~70% on Pro.

---

## 24. Launch & Go-to-Market

- **Pre-launch (Sept to Nov 2026):** landing page, waitlist from Nigerian and Kenyan church media groups, demo video showing End Service → PDF in 60 seconds.
- **Closed pilot (Nov 2026 to Jan 2027):** 10 to 15 churches, half in Nigeria/Kenya, three on macOS, white-glove setup.
- **Public launch (March 2027):** Product Hunt, church media press, YouTube with pilot pastors, free tier with no card.
- **Channels:** direct, church-tech creators, NRB and regional conferences, software directories.
- **Themes:** "Every sermon becomes a book", "The verse is on screen before you find it", "Works on Mac, works offline", "Licenses handled".

---

## 25. Post-Launch Support

- Email 24h weekdays / 48h weekends; in-app chat on paid tiers; knowledge base; community forum.
- Weekly bug-fix releases for the first 90 days, monthly minor releases, six-monthly majors, 24h security patches.
- Weekly standup, monthly metrics review, quarterly pastor advisory board.

---

## 26. Appendices & Glossary

### 26.1 Glossary

- **ASR / STT** speech to text. **Diarization** who said what. **FTS5** SQLite full-text search. **NDI** network video protocol for broadcast. **PCM** uncompressed audio. **USFM / OSIS** Bible text markup formats. **WASAPI Loopback** Windows system-audio capture. **Staging** holding a verse on the operator screen before it goes live. **Tauri** a desktop app framework that uses the OS webview and a Rust backend. **WebView2 / WKWebView** the system web renderers on Windows and macOS. **Pack** an optional, downloadable, checksummed bundle (speech model, translation, local LLM, theme).

### 26.2 Reference Translations (v1.0)

| Code | Name | License |
|---|---|---|
| KJV | King James Version | Public domain |
| WEB | World English Bible | Public domain |
| ASV | American Standard Version | Public domain |
| YLT | Young's Literal Translation | Public domain |
| DARBY | Darby Bible | Public domain |
| DRA | Douay-Rheims | Public domain |
| NKJV | New King James Version | Commercial |
| NIV | New International Version | Commercial |
| NLT | New Living Translation | Commercial |
| ESV | English Standard Version | Commercial |
| NASB | New American Standard Bible | Commercial |
| CSB | Christian Standard Bible | Commercial |
| NET | New English Translation | Mixed |
| AMP | Amplified Bible | Commercial |
| MSG | The Message | Commercial |
| GNT | Good News Translation | Commercial |
| CEV | Contemporary English Version | Commercial |
| ISV | International Standard Version | Mixed |
| NRSV | New Revised Standard Version | Commercial |
| RSV | Revised Standard Version | Commercial |

### 26.3 Summary JSON Schema (for Claude Code)

```json
{
  "title": "string",
  "theme": "string",
  "overview": "string",
  "takeaways": ["string"],
  "outline": {
    "introduction": "string",
    "points": [
      { "heading": "string", "subpoints": ["string"], "scriptures": ["Book C:V"] }
    ],
    "conclusion": "string"
  },
  "scriptures": [
    { "reference": "Book C:V", "translation": "KJV", "text": "string", "time_ms": 0 }
  ],
  "quotes": [ { "text": "string", "time_ms": 0 } ],
  "applications": ["string"],
  "discussion_questions": ["string"],
  "prayer_points": ["string"],
  "closing_summary": "string"
}
```

### 26.4 Revision History

| Version | Date | Change |
|---|---|---|
| 0.1 | May 2026 | Initial outline |
| 1.0 | May 2026 | Comprehensive PRD |
| 1.1 | May 2026 | Converted to markdown for development |
| 2.0 | 7 Sept 2026 | Competitive refresh (TajiCast, PewBeam), 19 new requirements, automatic detailed summary PDF as core feature, re-sequenced roadmap, revised pricing |
| 2.1 | 7 Sept 2026 | Stack changed to Tauri 2 + Rust for Windows/macOS parity and lightweight install; new §8.11 (FR-59 to FR-63) and §9.7 installation footprint; on-demand packs; `MILESTONES.md` introduced |

---

## Quick-Reference Index for Claude Code

| Working on... | Sections |
|---|---|
| Audio capture | §8.1, §10.2, §13.1 |
| Speech-to-text (online + offline) | §8.2, §15.1, §15.5 |
| Scripture detection (3 stages) | §8.3, §10.2, §18.1 |
| Bible cache + import | §8.4, §13.15, §14.2, §15.2 |
| Outputs (projector, alternate, NDI, OBS) | §8.5, §13.6, §13.7, §15.6, §15.7 |
| Staging + operator UI | §8.6, §13.3, §13.5, §16.2 |
| Phone remote | §8.7, §13.8, §15.8, §16.4, §17 |
| **Summary PDF pipeline** | **§8.8, §10.5, §13.11, §14.2 (sermon_summaries, exports), §26.3** |
| Archive search | §8.9 (FR-56), §13.13, §14.2 (FTS) |
| Content Studio, exports, book | §8.9, §13.14, §13.16 |
| AI slides | §8.10, §13.17 |
| **Installer, packs, updates** | **§8.11, §9.7, §10.7, §11, §14.3, §15.5** |
| Build order | §20 and `docs/MILESTONES.md` |

---

*End of Document — SermonAI PRD v2.1*
