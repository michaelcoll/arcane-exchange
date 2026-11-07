# OCR collector line prototype (#646, #656)

Throwaway app: opens the iPhone camera, runs Vision text recognition (accurate level, no language
correction) on every frame, and shows the raw recognised text and the time to a stable reading.
Kept in `main` as the starting point of the real scanner (report on #646); nothing here touches
`ios-app/`.

## Run

1. `xcodegen generate` in this directory (the `.xcodeproj` is git-ignored).
2. Replace `YOUR_TEAM_ID` in `project.yml` (both targets) with your Apple development team, regenerate.
3. Open `OCRPrototype.xcodeproj`, pick your iPhone, run.

The camera does not exist in the simulator: the simulator only proves it builds and that the
tests pass. Real verification needs an iPhone and physical cards.

## Card outline, zoom and focus

The small collector line came out blurred: the wide lens of an iPhone Pro does not focus closer
than ~20 cm, yet a card fills the screen much closer than that, and the autofocus aimed at the
centre (the art). `ScanGuide` (pure, tested) fixes the card's place in the image and derives:

- **Outline**: the card (63 × 88 mm) takes 80 % of the image width, high in the image, drawn in
  white on the preview. The preview shows the whole 9:16 image (`resizeAspect`) across the full
  screen width, from the top edge, so screen and image share the fractions; the results sit in a
  compact panel below the outline, raw text folded.
- **Camera**: the virtual `builtInTripleCamera` (then `builtInDualWideCamera`, then the wide lens
  alone), with automatic lens switching: when the card is closer than the wide lens can focus,
  the camera moves to the ultra wide, as the Camera app's macro does. The panel shows the
  physical lens in use.
- **Zoom**: at least the switch-over to the wide lens (the Camera app's "1×"; ×1 of a virtual
  camera is the ultra wide), and more if a card filling the outline would sit closer than
  `minimumFocusDistance` plus 10 % (Apple's AVCamBarcode approach). With the wide lens alone
  (~70°, focusing from 20 cm) that meant ×2.2 and the phone ~22 cm away, which proved awkward.
- **Focus and exposure** on the collector line (`focusPointOfInterest`,
  `exposurePointOfInterest`), autofocus restricted to `.near`, smooth autofocus off.
- **Vision region**: `regionOfInterest` is the bottom strip of the outline (dashed yellow): no
  rules text in the lines, and faster OCR.

## Card contour mode

A segmented control switches between two ways of finding the collector line, to compare them on
the same cards (switching resets the measure, so each latency belongs to one mode):

- **Bande fixe**: the fixed strip above (`ScanGuide.regionOfInterest`). A tilted card moves its
  collector line out of the strip.
- **Contour de la carte**: on every frame `VNDetectRectanglesRequest` looks for a portrait
  rectangle about a card's shape (aspect 0.55-0.9, at least a third of the image width,
  confidence ≥ 0.5). `CardContour` (pure, tested) keeps it only if it stands upright (tilt
  under 45°) and derives the collector line strip along the card's sides (bottom 16 % of the card,
  plus 4 % below it, in case Vision took the inner frame of a black-bordered card for its edge).
  `CIPerspectiveCorrection` straightens that strip (rotation and perspective) and Vision reads
  all of it. No card found: nothing is read, there is no fallback to the fixed strip, so the
  comparison stays clean.

Feedback: in the contour mode, the card found is drawn in green on the preview (the white outline
fades to a distance guide) with its strip in dashed yellow, and the panel says
`Carte détectée` / `Aucune carte détectée`. In both modes, a retained reading triggers a success
haptic. `ms / image` covers the detection and the reading.

Unverified on physical cards: detection against a dark background or with a finger on a corner,
sleeves, and whether Vision's corner order holds for every tilt below 45°.

## Timing measure

Frame timestamps come from the capture buffers (`PresentationTimer`).

- Card presented: first frame where Vision returns at least 1 text line (in the collector line
  strip) after the lens was empty (no such frame) for 1 s, or whose reading is no longer the one
  retained before (another reading, or none). Cards are swapped in front of the lens without
  turning it away: a stable reading different from the retained one is a new card. A blurred
  frame of the retained card (no reading, then the same reading again) is not a new card.
- Recognised: first later frame whose set of lines equals the previous frame's (one-frame
  stabilisation).
- Latency = recognised time - presented time. It includes the OCR time of each frame in between
  (`ms / frame` is shown too). Frames are dropped while Vision is busy.

Caveat: "presented" is when text first becomes readable, not when the card physically enters the
frame, so the figure is a lower bound.

## Collector line reader (#657)

`CollectorLineReader.read(lines:)` is a pure function: Vision's lines of one frame in, a
`CollectorReading {set, number, language}` or nil out. Understood formats (2014-2022 then since 2023), the two lines being separate or merged:

- `0123/0280 R` then `DMU • FR  Artist`
- `R 0123` then `MOM • EN`

Rarity `T` (a token) gives no reading: tokens are not scanned (see below). The "•" may come as `*`, `·`, `-`;
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

## Scryfall lookup (#659)

A retained reading triggers one lookup: `/cards/{set}/{number}/{lang}`, then on 404
`/cards/{set}/{number}`. When the English printing is shown for another language read, the screen
says so. One request per distinct reading, cached in memory per reading, at most 10 req/s,
`User-Agent` and `Accept` sent. Numbers are stored without padding (`12`, not `0012`).

Transform and modal DFCs are one Scryfall object with one number and two `card_faces`; their back
face carries the same collector line as the front, so either face leads to the same Card.

## Tokens: not scanned

Abandoned after the corpus pass (#646): Scryfall and MTGJSON give no reliable Cardmarket
`productId` for a double-faced token, so a scanned token could not be priced. The reader returns no
reading for rarity `T`, and tokens never enter a collection (#645). What Scryfall does with tokens is recorded
in the report on #646.

## Known limits (not fixed)

- Two copies of the same printing in a row are one card: the second is only retained after the
  lens has been empty for 1 s, since nothing in the reading tells them apart.
- The reader fixtures are still synthetic, not captured on real cards.
- Non-numeric collector numbers (`H13`, `★`...) are not read: no reading, no lookup.
- A token whose rarity `T` is misread gives a reading and looks up the Card with the same set and
  number (`mkm/12` instead of the token `tmkm/12`).
