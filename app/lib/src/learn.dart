// The Learn panel (docs/specs/11, docs/ux.md §1): short task cards. "Show
// me" never changes the project: it points at the views, buttons and
// panels involved, one after the other, with the same coach marks as the
// first-launch tour.

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';
import 'i18n.dart';
import 'tour.dart';

/// A place in the main window a card can point at.
enum Spot {
  pattern,
  garment,

  /// The inspector, showing the selection.
  inspector,

  /// The inspector with nothing selected: fabric, arrangement, pins.
  garmentInspector,
  simulate,
  cameras,
  body,
  sewTool,
  drawTool,
}

/// One coach mark of a card: where, a title, and how (`**bold**` marks
/// keys and button names).
typedef LearnStop = (Spot, String, String);

class _Card {
  const _Card(this.icon, this.title, this.how, this.stops);

  final IconData icon;
  final String title;
  final String how;
  final List<LearnStop> stops;
}

List<_Card> _cards(L l) => [
  _Card(Icons.play_circle_outline_rounded, l.learnDrape, l.learnDrapeHow, [
    (Spot.simulate, l.learnDrapeStop, l.learnDrapeStopText),
  ]),
  _Card(Icons.texture_rounded, l.learnFabric, l.learnFabricHow, [
    (Spot.garmentInspector, l.learnFabricStop, l.learnFabricStopText),
  ]),
  _Card(Icons.height_rounded, l.learnLength, l.learnLengthHow, [
    (Spot.pattern, l.learnPickHem, l.learnPickHemText),
    (Spot.inspector, l.learnShorterLonger, l.learnShorterLongerText),
    (Spot.garment, l.learnDragHem, l.learnDragHemText),
  ]),
  _Card(Icons.compress_rounded, l.learnFit, l.learnFitHow, [
    (Spot.pattern, l.learnPickSeam, l.learnPickSeamText),
    (Spot.inspector, l.learnTakeInLetOut, l.learnTakeInLetOutText),
    (Spot.garment, l.learnDragSeam, l.learnDragSeamText),
  ]),
  _Card(Icons.link_rounded, l.learnSew, l.learnSewHow, [
    (Spot.sewTool, l.learnSewTool, l.learnSewToolText),
    (Spot.pattern, l.learnTwoEdges, l.learnTwoEdgesText),
  ]),
  _Card(Icons.brush_rounded, l.learnDraw, l.learnDrawHow, [
    (Spot.drawTool, l.learnDrawTool, l.learnDrawToolText),
    (Spot.garment, l.learnDrawAcross, l.learnDrawAcrossText),
  ]),
  _Card(Icons.open_with_rounded, l.learnMove, l.learnMoveHow, [
    (Spot.garment, l.learnDragPiece, l.learnDragPieceText),
    (Spot.inspector, l.learnSetPlacement, l.learnSetPlacementText),
  ]),
  _Card(Icons.push_pin_outlined, l.learnPin, l.learnPinHow, [
    (Spot.garment, l.learnPullPin, l.learnPullPinText),
    (Spot.garmentInspector, l.learnYourPins, l.learnYourPinsText),
  ]),
  _Card(Icons.threesixty_rounded, l.learnLook, l.learnLookHow, [
    (Spot.cameras, l.learnViews, l.learnViewsText),
    (Spot.garment, l.learnTurnZoom, l.learnTurnZoomText),
  ]),
  _Card(Icons.accessibility_new_rounded, l.learnBody, l.learnBodyHow, [
    (Spot.body, l.learnBodyStop, l.learnBodyStopText),
  ]),
];

/// The Learn panel, as a side sheet.
class LearnPanel extends StatelessWidget {
  const LearnPanel({super.key, required this.onShow});

  /// Point at these places, one after the other (the panel is closed).
  final ValueChanged<List<LearnStop>> onShow;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final l = context.l;
    return Drawer(
      width: 380,
      backgroundColor: p.surface,
      child: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(Space.xl),
          children: [
            Row(
              children: [
                Icon(Icons.school_outlined, color: p.accent),
                const SizedBox(width: Space.s),
                Text(
                  l.learn,
                  style: ts(fontSize: 20, fontWeight: FontWeight.w700),
                ),
                const Spacer(),
                IconButton(
                  onPressed: () => Navigator.pop(context),
                  icon: Icon(Icons.close_rounded, color: p.textMuted),
                ),
              ],
            ),
            Text(
              l.learnIntro,
              style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
            ),
            const SizedBox(height: Space.l),
            for (final card in _cards(l))
              _CardView(
                card: card,
                onShow: () {
                  Navigator.pop(context);
                  onShow(card.stops);
                },
              ),
          ],
        ),
      ),
    );
  }
}

class _CardView extends StatelessWidget {
  const _CardView({required this.card, required this.onShow});

  final _Card card;
  final VoidCallback onShow;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    return Container(
      margin: const EdgeInsets.only(bottom: Space.m),
      padding: const EdgeInsets.all(Space.l),
      decoration: BoxDecoration(
        color: p.surfaceMuted,
        borderRadius: BorderRadius.circular(kRadius * 0.8),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              Icon(card.icon, size: 18, color: p.accent),
              const SizedBox(width: Space.s),
              Expanded(
                child: Text(
                  card.title,
                  style: ts(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: p.text,
                  ),
                ),
              ),
              TextButton(
                onPressed: onShow,
                style: TextButton.styleFrom(
                  visualDensity: VisualDensity.compact,
                ),
                child: Text(context.l.showMe),
              ),
            ],
          ),
          const SizedBox(height: 6),
          Text.rich(
            marked(
              context.keys(card.how),
              ts(fontSize: 12.5, color: p.textMuted, height: 1.4),
              p.text,
            ),
          ),
        ],
      ),
    );
  }
}
