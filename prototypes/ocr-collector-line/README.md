# OCR collector line prototype (#646, #656)

Throwaway app: opens the iPhone camera, runs Vision text recognition (accurate level, no language
correction) on every frame, and shows the raw recognised text and the time to a stable reading.
Never merged into `main`; nothing here touches `ios-app/`.

## Run

1. `xcodegen generate` in this directory (the `.xcodeproj` is git-ignored).
2. Replace `YOUR_TEAM_ID` in `project.yml` (both targets) with your Apple development team, regenerate.
3. Open `OCRPrototype.xcodeproj`, pick your iPhone, run.

The camera does not exist in the simulator: the simulator only proves it builds and that the
tests pass. Real verification needs an iPhone and physical cards.

## Timing measure

Frame timestamps come from the capture buffers (`PresentationTimer`).

- Card presented: first frame where Vision returns at least 3 text lines after the lens was
  empty (no such frame) for 1 s.
- Recognised: first later frame whose set of lines equals the previous frame's (one-frame
  stabilisation).
- Latency = recognised time - presented time. It includes the OCR time of each frame in between
  (`ms / frame` is shown too). Frames are dropped while Vision is busy.

Caveat: "presented" is when text first becomes readable, not when the card physically enters the
frame, so the figure is a lower bound.

## Collector line reader (#657)

`CollectorLineReader.read(lines:)` is a pure function: Vision's lines of one frame in, a
`CollectorReading {set, number, language, isToken}` or nil out. Understood formats (2014-2022 then since 2023), the two lines being separate or merged:

- `0123/0280 R` then `DMU • FR  Artist`
- `R 0123` then `MOM • EN`

Rarity `T` = token (the Scryfall set is then `t` + `set`). The "•" may come as `*`, `·`, `-`;
O/I/l in the number are read as 0/1; leading zeros are stripped; printed languages map to Scryfall
codes (SP -> es, JP -> ja...). A reading needs set, number and a known language together. No
bounding boxes are used.

Stabilisation: a reading is retained only when identical on two consecutive frames
(`PresentationTimer.observe(lines:reading:at:)`), so the latency shown includes that extra frame.

The fixtures in `OCRPrototypeTests/CollectorLineFixtures.swift` are SYNTHETIC (written from the
known printed formats, not captured): replace/extend them with real captures in #658.

## Fixtures

`Copier le texte brut` / `Exporter` give the raw lines of the current image at any time (also when
the read fails), one Vision line per row. Paste them into the test target as fixtures for the
collector line reader (#657). The screen texts are in French.

## Tests

From this directory (outside `mise`, the prototype has no task): `xcodebuild test -project
OCRPrototype.xcodeproj -scheme OCRPrototype -destination 'platform=iOS Simulator,id=<simulator id>'
CODE_SIGNING_ALLOWED=NO`. Use the id (`xcrun simctl list devices`): a device name matches several
runtimes.

## Scryfall et tokens (#659)

From the Scryfall API (checked 2026-11-05), NOT yet checked against physical cards.

- A retained reading triggers one lookup. Rarity `T`: only the token set, `/cards/t{set}/{number}`
  (English, no language request). Any other rarity or none: `/cards/{set}/{number}/{lang}` (on 404
  `/cards/{set}/{number}`) AND `/cards/t{set}/{number}`; every Card found is shown, labelled `CARTE`
  or `TOKEN` (the `mkm/12` vs `tmkm/12` collision). When the English printing is shown for another
  language read, the screen says so. One request per distinct reading, cached in memory per reading,
  at most 10 req/s, `User-Agent` and `Accept` sent.
- Double-faced token (`layout: double_faced_token`: Incubator // Phyrexian `tmom` 16-18, `teoc` 15,
  `tfrc` 13; also Punchcard, Bounty/Wanted!, The Ring, speed cards): ONE object, ONE number, two
  `card_faces`. Either face leads to the same Card. Transform and modal DFCs work the same way.
- Regular and Commander tokens: one `layout: token` object per face, each with its own number, no
  link between the two sides. If the physical card is printed back to back, reading each side gives
  a different Card (unverified, gap against the "same Card per face" criterion).
- Token set code = `t` + parent set code (`tmkm`), but the parent code collides: `mkm/12` is a card,
  `tmkm/12` a token. Only the rarity `T` (or the type) tells them apart.
- Scryfall has no non-English token at all: `tmkm/12/fr` is a 404, so a token goes straight to
  English. The 404 body does not say whether set, number or language is wrong.
- Numbers are stored without padding (`12`, not `0012`); some are not plain integers (`H13`, `T12`, `5★`).

## Known limits (not fixed)

- Post-2023 format: the layout "rarity then number" (`R 0123`) is unverified on real cards. The
  reader accepts it merged with other text on a line, but only from synthetic fixtures.
- Non-numeric collector numbers (`H13`, `★`...) are not read: no reading, no lookup.
- Token detection rests on the rarity letter `T`; when it is misread or absent, both the Card and
  the token set are looked up and shown (the disambiguation is left to the human).
