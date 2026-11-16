// The Learn panel (docs/specs/11, docs/ux.md §1): short task cards. Some
// play as demos on the open project, through the same commands as the
// tools (so ⌘Z undoes them); the others switch to the tool and say how.

import 'package:material_ui/material_ui.dart';

import 'design/tokens.dart';

/// What the Learn panel can do in the shell.
class LearnActions {
  const LearnActions({
    required this.run,
    required this.query,
    required this.setTool,
    required this.simulate,
    required this.showBody,
    required this.look,
    required this.toast,
  });

  /// Run a command step; returns its result, or null on error.
  final Object? Function(Map<String, Object?> step) run;

  /// Run a query step (not recorded).
  final Object? Function(Map<String, Object?> step) query;
  final ValueChanged<String> setTool;
  final ValueChanged<bool> simulate;
  final VoidCallback showBody;

  /// Look from `front`, `side`, `back` or `three_quarter`.
  final ValueChanged<String> look;
  final ValueChanged<String> toast;
}

typedef _Demo = Future<void> Function(LearnActions a);

class _Card {
  const _Card(
    this.icon,
    this.title,
    this.how, {
    this.demo,
    this.tool,
    this.body = false,
  });

  final IconData icon;
  final String title;
  final String how;

  /// Plays on the open project.
  final _Demo? demo;

  /// Or switches to this tool.
  final String? tool;

  /// Or opens the Body panel.
  final bool body;
}

Future<void> _pause(int ms) => Future.delayed(Duration(milliseconds: ms));

/// The first panel's hem: its lowest edge that is not sewn.
String? _hem(LearnActions a) {
  final summary = a.query({'type': 'project.summary'}) as Map?;
  final panels = (summary?['panels'] as List?) ?? const [];
  final seams = (a.query({'type': 'seam.list'}) as List?) ?? const [];
  final sewn = {
    for (final s in seams) ...[s['a']['edge'], s['b']['edge']],
  };
  for (final p in panels) {
    final panel = a.query({'type': 'panel.get', 'panel': p['id']}) as Map?;
    final edges = ((panel?['edges'] as List?) ?? const [])
        .where((e) => !sewn.contains(e['id']))
        .toList();
    if (edges.isEmpty) continue;
    double low(Map e) =>
        ((e['start']['pos'][1] as num) + (e['end']['pos'][1] as num)) / 2;
    edges.sort((x, y) => low(x).compareTo(low(y)));
    return edges.first['id'] as String;
  }
  return null;
}

final _cards = <_Card>[
  _Card(
    Icons.play_circle_outline_rounded,
    'Drape the garment',
    'Press Simulate (Space): the pieces close their seams and fall onto the body. Restart puts them back around it.',
    demo: (a) async => a.simulate(true),
  ),
  _Card(
    Icons.texture_rounded,
    'Try another fabric',
    'Fabric changes how the garment hangs. Pick one in the panel on the right, with nothing selected.',
    demo: (a) async {
      final summary = a.query({'type': 'project.summary'}) as Map?;
      final before = summary?['garments']?[0]?['fabric'] as String?;
      a.simulate(true);
      for (final f in ['denim', 'silk']) {
        a.run({'type': 'garment.set_fabric', 'fabric': f});
        a.toast('Now in $f');
        await _pause(3500);
      }
      if (before != null) {
        a.run({'type': 'garment.set_fabric', 'fabric': before});
      }
    },
  ),
  _Card(
    Icons.height_rounded,
    'Lengthen or shorten',
    'Hold ⇧ and drag a hem on the garment, down to lengthen, up to shorten. The pattern follows.',
    demo: (a) async {
      final hem = _hem(a);
      if (hem == null) {
        a.toast('Every edge is sewn: there is no hem to move.');
        return;
      }
      a.simulate(true);
      a.run({'type': 'edge.lengthen', 'edge': hem, 'mm': 40});
      a.toast('Lengthened by 4 cm. ⌘Z undoes it.');
    },
  ),
  _Card(
    Icons.compress_rounded,
    'Take in a seam',
    'Hold ⇧ and drag a seam on the garment: towards the body takes it in, away lets it out. Near one end, only that end moves.',
    demo: (a) async {
      final seams = (a.query({'type': 'seam.list'}) as List?) ?? const [];
      if (seams.isEmpty) {
        a.toast('Sew two edges first: there is no seam yet.');
        return;
      }
      a.simulate(true);
      a.run({'type': 'seam.take_in', 'seam': seams.first['id'], 'mm': 15});
      a.toast('Taken in by 1.5 cm. ⌘Z undoes it.');
    },
  ),
  const _Card(
    Icons.link_rounded,
    'Sew two pieces',
    'With the Sew tool (S), click an edge on the pattern, then the edge to sew it to. Matching arrows show how they join.',
    tool: 'sew',
  ),
  const _Card(
    Icons.brush_rounded,
    'Draw a style line',
    'With Draw on garment (D), drag across the 3D garment: the line lands on the pattern, for a yoke or a pocket.',
    tool: 'draw',
  ),
  const _Card(
    Icons.open_with_rounded,
    'Move a piece around the body',
    'Hold ⌘ (Ctrl on Linux and Windows) and drag a piece on the garment: sideways turns it around the body, up and down changes its height. The inspector has the same settings.',
  ),
  const _Card(
    Icons.push_pin_outlined,
    'Pull and pin the cloth',
    'Drag the cloth in 3D to pull it. ⌥-click to pin it where it is; pins are listed on the right.',
  ),
  _Card(
    Icons.threesixty_rounded,
    'Look around',
    'Drag to turn around the body, scroll to zoom. Keys 1–4: front, side, back, three quarters.',
    demo: (a) async {
      for (final side in ['front', 'side', 'back', 'three_quarter']) {
        a.look(side);
        await _pause(1200);
      }
    },
  ),
  const _Card(
    Icons.accessibility_new_rounded,
    'Dress another body',
    'Choose a size or enter measurements: the body is refitted and the garment drapes on it.',
    body: true,
  ),
];

/// The Learn panel, as a side sheet.
class LearnPanel extends StatelessWidget {
  const LearnPanel({super.key, required this.actions});

  final LearnActions actions;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
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
                  'Learn',
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
              'Short how-tos. “Show me” plays on your project; ⌘Z undoes it.',
              style: ts(fontSize: 13, color: p.textMuted, height: 1.4),
            ),
            const SizedBox(height: Space.l),
            for (final card in _cards) _CardView(card: card, actions: actions),
          ],
        ),
      ),
    );
  }
}

class _CardView extends StatelessWidget {
  const _CardView({required this.card, required this.actions});

  final _Card card;
  final LearnActions actions;

  @override
  Widget build(BuildContext context) {
    final p = Tokens.of(context);
    final demo = card.demo;
    final tool = card.tool;
    final (String? label, VoidCallback? onTap) = switch (card) {
      _ when demo != null => (
        'Show me',
        () {
          Navigator.pop(context);
          demo(actions);
        },
      ),
      _ when tool != null => (
        'Try it',
        () {
          Navigator.pop(context);
          actions.setTool(tool);
        },
      ),
      _Card(body: true) => (
        'Open Body',
        () {
          Navigator.pop(context);
          actions.showBody();
        },
      ),
      _ => (null, null),
    };
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
              if (label != null)
                TextButton(
                  onPressed: onTap,
                  style: TextButton.styleFrom(
                    visualDensity: VisualDensity.compact,
                  ),
                  child: Text(label),
                ),
            ],
          ),
          const SizedBox(height: 6),
          Text(
            card.how,
            style: ts(fontSize: 12.5, color: p.textMuted, height: 1.4),
          ),
        ],
      ),
    );
  }
}
